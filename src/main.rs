use env_logger;
use log::{debug, info};
use lru::LruCache;
use rust_decimal::Decimal;
use std::collections::HashSet;
use std::io;
use std::num::NonZero;

use crate::server::get_fills_api;
use crate::server::Fill;

pub mod server;

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let mut processor = Processor::new();
    let mut cache_hits = 0;
    let mut api_calls = 0;

    info!("Starting query processing...");

    for query in io::stdin().lines() {
        processor.process_query(query?, &mut cache_hits, &mut api_calls)?;
    }

    info!("{}", processor.print_cache_stats());
    info!(
        "Cache hit rate: {:.2}%",
        (cache_hits as f64 / (cache_hits + api_calls) as f64) * 100.0
    );
    info!("Cache hits: {}", cache_hits);
    info!("API calls: {}", api_calls);

    Ok(())
}

/// A proxy server implementation for orderbook trades that caches hourly trade data
/// to minimize expensive API calls.
///
/// Caching Strategy:
/// - Uses LRU cache with 168-hour capacity (one week of data)
/// - Caches full hourly data to handle arbitrary queries within each hour
/// - Trades within an hour are cached together to optimize for temporal locality
pub struct Processor {
    /// LRU cache stores hourly trade data
    /// Key: Hour timestamp (rounded down)
    /// Value: Vector of fills for that hour
    cache: LruCache<i64, Vec<Fill>>,
    /// Temporary storage for current query processing
    current_fills: Vec<Fill>,
}

impl Processor {
    /// Returns the size of the cache in terms of:
    /// - Total number of fills
    /// - Total number of bytes
    /// - Maximum number of fills in a single hour
    fn get_cache_size(&self) -> (usize, usize, usize) {
        let mut total_fills = 0;
        let mut total_bytes = std::mem::size_of::<LruCache<i64, Vec<Fill>>>();
        let mut max_fills = 0;

        // Add size of each cache entry
        for (_, fills) in self.cache.iter() {
            total_fills += fills.len();
            total_bytes += std::mem::size_of::<i64>(); // key size
            total_bytes += std::mem::size_of::<Vec<Fill>>(); // vector overhead
            total_bytes += fills.len() * std::mem::size_of::<Fill>(); // actual fills
            max_fills = max_fills.max(fills.len());
        }

        (total_fills, total_bytes, max_fills)
    }

    /// Prints the cache statistics in a formatted string
    pub fn print_cache_stats(&self) -> String {
        let (total_fills, total_bytes, max_fills) = self.get_cache_size();
        let cache_stats = format!(
            r#"
Cache Statistics:
    Number of hours cached: {}
    Total fills stored: {}
    Maximum fills in a single hour: {}
    Approximate memory usage: {} bytes ({:.2} MB)"#,
            self.cache.len(),
            total_fills,
            max_fills,
            total_bytes,
            total_bytes as f64 / 1_000_000.0
        );
        cache_stats
    }

    /// Creates a new Processor with:
    /// - LRU cache sized for one week of data (168 hours)
    /// - Temporary vector to store fills for the current query
    pub fn new() -> Self {
        Processor {
            cache: LruCache::new(NonZero::new(168).unwrap()),
            current_fills: Vec::new(),
        }
    }

    /// Rounds timestamp down to the start of its hour
    fn get_start_hour(&self, time: i64) -> i64 {
        time - (time % 3600)
    }

    /// Processes a single query and prints the result
    /// Query format: "TYPE START_TIME END_TIME"
    /// where TYPE is one of: buy (B), sell (S), total count (C), or volume (V)
    pub fn process_query(
        &mut self,
        query: String,
        cache_hits: &mut usize,
        api_calls: &mut usize,
    ) -> anyhow::Result<()> {
        debug!("Processing query: {}", query);

        let query_parts = query.split_whitespace().collect::<Vec<&str>>();
        if query_parts.len() != 3 {
            return Err(anyhow::anyhow!("Invalid query format: {}", query));
        }

        let query_type = query_parts[0];
        let start_time = query_parts[1].parse::<i64>()?;
        let end_time = query_parts[2].parse::<i64>()?;

        let start_hour = self.get_start_hour(start_time);
        let end_hour = self.get_start_hour(end_time);

        self.current_fills.clear();

        // Retrieve fills for the start hour
        if let Some(stored_fills) = self.cache.get(&start_hour) {
            debug!("Cache hit for hour: {}", start_hour);
            self.current_fills.extend(stored_fills);
            *cache_hits += 1;
        } else {
            debug!("Cache miss for hour: {}", start_hour);
            let fills = get_fills_api(start_hour, start_hour + 3600)?;
            self.current_fills.extend(&fills);
            self.cache.put(start_hour, fills);
            *api_calls += 1;
        }

        // Retrieve fills for the end hour if it's different from the start hour
        if start_hour != end_hour {
            if let Some(next_fills) = self.cache.get(&end_hour) {
                debug!("Cache hit for hour: {}", end_hour);
                self.current_fills.extend(next_fills);
                *cache_hits += 1;
            } else {
                debug!("Cache miss for hour: {}", end_hour);
                let next_hour_fills = get_fills_api(end_hour, end_hour + 3600)?;
                self.current_fills.extend(&next_hour_fills);
                self.cache.put(end_hour, next_hour_fills);
                *api_calls += 1;
            }
        }

        // Process fills within time range
        let mut buy_count = 0;
        let mut sell_count = 0;
        let mut total_volume = Decimal::ZERO;
        let mut unique_sequences = HashSet::with_capacity(self.current_fills.len());

        for fill in &self.current_fills {
            if fill.time.timestamp() > start_time && fill.time.timestamp() <= end_time {
                if unique_sequences.insert(fill.sequence_number) {
                    if fill.direction == 1 {
                        buy_count += 1;
                    } else {
                        sell_count += 1;
                    }
                }
                total_volume += fill.quantity * fill.price;
            }
        }

        match query_type {
            "S" => println!("{}", sell_count),
            "B" => println!("{}", buy_count),
            "C" => println!("{}", buy_count + sell_count),
            "V" => println!("{}", total_volume),
            _ => return Err(anyhow::anyhow!("Invalid query type: {}", query_type)),
        }

        Ok(())
    }
}

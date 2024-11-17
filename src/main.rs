use lru::LruCache;
use rust_decimal::Decimal;
use std::collections::HashSet;
use std::io;
use std::num::NonZero;

use crate::server::get_fills_api;
use crate::server::Fill;

pub mod server;

fn main() -> anyhow::Result<()> {
    let mut processor = Processor::new();
    for query in io::stdin().lines() {
        processor.process_query(query?)?;
    }
    Ok(())
}

/* ~~~~~~~~~~~~~~~~~~~~~~~~~~~ YOUR CODE HERE ~~~~~~~~~~~~~~~~~~~~~~~~~~~ */

pub struct Processor {
    cache: LruCache<i64, Vec<Fill>>,
    current_fills: Vec<Fill>,
}

impl Processor {
    pub fn new() -> Self {
        Processor {
            cache: LruCache::new(NonZero::new(165).unwrap()),
            current_fills: Vec::with_capacity(5000),
        }
    }

    fn get_start_hour(&self, time: i64) -> i64 {
        time - (time % 3600)
    }

    pub fn process_query(&mut self, query: String) -> anyhow::Result<()> {
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

        if let Some(stored_fills) = self.cache.get(&start_hour) {
            self.current_fills.extend(stored_fills);
        } else {
            let fills = get_fills_api(start_hour, start_hour + 3600)?;
            self.current_fills.extend(&fills);
            self.cache.put(start_hour, fills);
        }

        if start_hour != end_hour {
            if let Some(next_fills) = self.cache.get(&end_hour) {
                self.current_fills.extend(next_fills);
            } else {
                let next_hour_fills = get_fills_api(end_hour, end_hour + 3600)?;
                self.current_fills.extend(&next_hour_fills);
                self.cache.put(end_hour, next_hour_fills);
            }
        }

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

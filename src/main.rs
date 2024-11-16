use rust_decimal::Decimal;
use std::collections::HashSet;
use std::io;

use crate::server::get_fills_api;

pub mod server;

fn main() -> anyhow::Result<()> {
    let mut processor = Processor::new();
    for query in io::stdin().lines() {
        processor.process_query(query?);
    }
    Ok(())
}

/* ~~~~~~~~~~~~~~~~~~~~~~~~~~~ YOUR CODE HERE ~~~~~~~~~~~~~~~~~~~~~~~~~~~ */

pub struct Processor {
    // TODO
}

impl Processor {
    pub fn new() -> Self {
        Processor {}
    }

    pub fn process_query(&mut self, query: String) {
        let query_parts = query.split_whitespace().collect::<Vec<&str>>();
        if query_parts.len() != 3 {
            eprintln!("Invalid query: {}", query);
            return;
        }

        let query_type = query_parts[0];
        let start_time = query_parts[1].parse::<i64>().unwrap();
        let end_time = query_parts[2].parse::<i64>().unwrap();

        let fills = get_fills_api(start_time, end_time).unwrap();

        // print the count of fills for trades with unique sequence numbers and the given query type
        // if there are multiple fills with the same sequence number, only count the fills once

        let unique_fills = fills
            .iter()
            .map(|fill| (fill.sequence_number, fill.direction))
            .collect::<HashSet<_>>();

        match query_type {
            "S" => println!(
                "{:?}",
                unique_fills.iter().filter(|fill| fill.1 == -1).count()
            ),
            "B" => println!(
                "{:?}",
                unique_fills.iter().filter(|fill| fill.1 == 1).count()
            ),
            "C" => println!("{:?}", unique_fills.len()),
            "V" => println!(
                "{:?}",
                fills
                    .iter()
                    .map(|fill| fill.quantity * fill.price)
                    .sum::<Decimal>()
            ),
            _ => println!("Invalid query type"),
        }
    }
}

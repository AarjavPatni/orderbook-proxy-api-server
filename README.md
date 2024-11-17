# Orderbook Trade Query Processor

This project implements a proxy server for orderbook trades, designed to cache hourly trade data and minimize expensive API calls. It is optimized for use in financial trading systems where efficient data retrieval and processing are critical.

## Table of Contents

- [Orderbook Query Constraints](#orderbook-query-constraints)
- [Program Input](#program-input)
- [Instructions](#instructions)
- [Key Features](#key-features)
- [Caching Strategy](#caching-strategy)
   - [Core Implementation](#core-implementation)
   - [Data Flow](#data-flow)
   - [Performance Benchmarks](#performance-benchmarks)
- [Assumptions](#assumptions)
   - [RAM Requirements Analysis](#ram-requirements-analysis)
- [Tradeoffs](#tradeoffs)
- [Other Design Choices](#other-design-choices)
- [Logging Usage](#logging-usage)


## Orderbook Query Constraints

-  The duration between `END_TIME` and `START_TIME` must not exceed 3600 seconds.
-  All time inputs will fall within the range of available trading data.
-  A _taker trade_ is uniquely identified by a sequence number. If two fills share the same sequence number, they correspond to the same taker trade. (Note: Taker trades include two types: market buys and market sells.)

## Program Input

The program receives a list of input queries formatted as follows:

```
QUERY_TYPE START_TIME END_TIME
```

`QUERY_TYPE` can be one of the following: `C`, `B`, `S`, or `V`. The server should output the following for each query type:

-  `C`: Outputs the count of all taker trades within the specified time range (> start, <= end).
-  `B`: Outputs the count of all market buys within the specified time range (> start, <= end).
-  `S`: Outputs the count of all market sells within the specified time range (> start, <= end).
-  `V`: Outputs the total trading volume in USD within the specified time range (> start, <= end).

`START_TIME` is a Unix timestamp in seconds, indicating that only trades occurring after this time should be considered.

`END_TIME` is a Unix timestamp in seconds, indicating that only trades occurring before or at this time should be considered.


## Instructions

Navigate to the root directory containing the trades data (`trades.csv`) and input queries (`input.txt`). Then run:

```
cat input.txt | cargo --quiet run
```

## Key Features
- **Efficient Caching**: LRU cache with a 168-hour capacity, providing a 64% speedup for query processing.
- **Diverse Query Handling**: Supports buy (B), sell (S), total count (C), and volume (V) queries.
- **Detailed Logging**: Captures cache statistics, hit rates, and API calls.


## Caching Strategy

The system implements an LRU (Least Recently Used) cache optimized for hourly trade data:

### Core Implementation
- Cache capacity: 168 hours (one week of data)
- Key: Hour timestamp (rounded down to hour boundary)
- Value: Complete vector of `Fill` (trades) for that hour

### Data Flow
1. When a query arrives:
   - Round timestamps to hour boundaries
   - Check cache for each required hour
   - If it doesn't exist, fetch missing data from API and add to cache
   - For two-hour queries, repeat the process for each hour and merge the results
   - Filter the combined results based on the exact timestamp range

2. Cache Management:
   - Automatic eviction of data for the least recently used hours when capacity is reached
   - Each hour's data is fetched only once and reused for all subsequent queries, unless it is evicted

### Performance Benchmarks
- **Environment**: MacBook Pro (16GB RAM, M2 Pro chip)
- **Dataset**: 235,834 trades over ~165 hours
- **Query Set**: 1000 random queries
- **Results**:
  - **Memory Usage**: 13.2MB (normal), 47MB (worst case)
  - **API Calls**: Reduced by 83.6% (from 1000 to 164)
  - **Query Processing Time**: Decreased by 64% (from 31.8s to 11.5s)



## Assumptions
1. **Data Characteristics**
    - Trade volume varies; average 1,438 trades/hour, peak 4,211 trades/hour 
    - Data remains static during program execution
    - Trades are either buy or sell, not both (direction is either 1 or -1)

2. **System Resources**
    - Memory usage (~13MB for one week) is acceptable for the performance gains
    - Single-threaded execution is sufficient
    - The system has at least 50 MB of RAM. This is analyzed below.

3. **Allowed to Use External Libraries**
    - `anyhow` for error handling
    - `lru` for the LRU cache
    - `rust_decimal` for precise financial calculations
    - `HashSet` for storing unique timestamps

### RAM Requirements Analysis
   - Total fills in the dataset = 235834
   - Average fills per hour ≈ 1438
   - Maximum fills in a single hour = 4211
   - Size of cache containing all fills from dataset = 13212016 bytes ≈ 13.2 MB
   - Size of each fill = 13212016 / 235834 ≈ 56 bytes
   - Assuming fills in a peak hour = 5000
   - Size of cache holding one week of peak hours data = 5000 * 56 * 168 = 47040000 bytes ≈ 47 MB

## Tradeoffs
1. Store raw `Fill` data vs. processed results
   - Pros: Maintains flexibility for different query types
   - Cons: Uses more memory, requires processing on each query

2. Cache entire hours vs. exact query ranges
   - Pros: Simpler management, better cache hit rate
   - Cons: May store some unused data, using more memory

3. LRU Cache vs. Simple HashMap
   - Pros:
     - Predetermined capacity
     - Evicts least recently used data
   - Cons:
     - Slightly more complex implementation
     - Uses more memory because it uses both a HashMap and a doubly linked list


## Other Design Choices

1. `Decimal` vs. `f64`
   - Pros: More precise for financial calculations. Avoids floating point precision issues.
   - Cons: Slightly more memory and slower to process

2. Anyhow (`anyhow`) vs. Standard Error Handling (`eprintln!`)
   - Pros:
     - More informative error messages
     - Provides a dynamic error type (`anyhow::Error`) that can encapsulate any error
     - Idiomatic error handling in the Rust ecosystem
   - Cons:
     - Slightly slower to process
     - Uses more memory

3. Uninitialized `current_fills` vector
   - Pros: Allows for dynamic resizing based on the hourly trade volume
   - Cons: Requires constant reallocation of memory
   - If this becomes a performance issue, we can preallocate to the maximum number of fills in a single hour (5000)


## Logging Usage
- Level of logging can be adjusted by setting the `RUST_LOG` environment variable.
  - `RUST_LOG=info cargo run` will show important information, warnings, and errors.
  - `RUST_LOG=debug cargo run` will show detailed information about each query and cache hit/miss.

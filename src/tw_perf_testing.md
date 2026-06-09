## Performance Tests For Production

### Production Performance Pyramid
| Test Type | Tool | Focus | When to Run |
| :--- | :--- | :--- | :--- |
| **Micro-bench** | **Criterion** | `buy_impl` algorithmic efficiency | Every PR |
| **Load Test** | **Drill / wrk** | Lock contention & RPS | Release candidates |
| **Profiling** | **Flamegraph** | CPU "Hot paths" | During optimization |



### 1. Micro-benchmarking (The "Algorithm" Level)
Since we are optimizing specific operations like `retain` vs `pop_first` on a `BTreeMap`, we should use **Criterion.rs**. This is the industry standard for measuring small units of code with statistical rigor.
*   **The Goal:** Isolate the `buy_impl` logic from all network/locking overhead to see how it scales with the number of orders.
*   **Setup:** Create benchmarks that populate a map with $10^3, 10^4, \text{and } 10^5$ bids to measure the exact nanosecond cost of rebalancing.

#### Implementation:

Note: The following is for "the first version" of `sell_impl`, also see the bench fn for `buy_impl` in the same file.

A sample **Criterion** benchmark `benches/allocation.rs` measures how long `sell_impl` takes to allocate all outstanding bids as the number of bids scales from 1K to 100K.

Run it with:
```rust
cargo bench --bench allocation -- sell_impl
```

```rust
Gnuplot not found, using plotters backend
Benchmarking sell_impl/retain/1000: Warming up for 3.0000 s
                                           // median
sell_impl/retain/1000   time:   [1.1392 ms 1.1933 ms 1.2575 ms]
                        // p — statistical significance;
                        // p < 0.05 means the change is real, not random noise
                        // change - vs last cargo bench
                        change: [-38.435% -34.694% -31.078%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 4 outliers among 100 measurements (4.00%)
  4 (4.00%) high mild
    // Outliers — samples that deviate significantly from the median; 
    // "mild" = slightly off, "severe" = very off, 
    // often caused by OS scheduling or GC

sell_impl/retain/10000  time:   [11.681 ms 12.267 ms 12.946 ms]
                        change: [-44.856% -39.987% -34.194%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 10 outliers among 100 measurements (10.00%)
  6 (6.00%) high mild
  4 (4.00%) high severe
Benchmarking sell_impl/retain/100000: Warming up for 3.0000 s


sell_impl/retain/100000 time:   [132.57 ms 136.94 ms 141.61 ms]
                        change: [-21.288% -17.064% -12.507%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 9 outliers among 100 measurements (9.00%)
  8 (8.00%) high mild
  1 (1.00%) high severe

```

#### Bench Result:

Linear scaling — roughly O(n):

| Bids | Time |
|------|------|
| 1K | ~1.2ms |
| 10K | ~12ms |
| 100K | ~137ms |

10x more bids → ~10x more time, confirms O(n) behavior of `retain`. But for 100K+ concurrent open bids we might want to bench with `pop_first` with early exit.



### 2. HTTP Load Testing (The "System" Level)
Once the algorithm is fast, we must test the **lock contention** in Axum handlers under high load. Use a tool like **Drill** or **Goku**, which are written in Rust for high-throughput benchmarking.
*   **The Goal:** Measure how many Requests Per Second (RPS) `buy` handler can process before the fine-grained locks cause latency spikes (P99s).
*   **Metric to Watch:** **Tail Latency.** In financial systems, the average latency is often a "lie"; we care about the P99 or P99.9—the worst-case delay experienced by users.

### 3. Continuous Profiling (The "Visibility" Level)
In production, we cannot always reproduce performance issues locally. Use **Flamegraphs** (via `cargo-flamegraph`) to visualize exactly where CPU time is being spent—whether it's inside the `BTreeMap` search or waiting for a `Mutex`.
*   **The Goal:** Identify "hot paths" and locking bottlenecks visually.
*   **Tooling:** Use the **Tracing** crate to instrument code. This allows we to collect timing data across `buy` and `buy_impl` boundary without stopping the service.

---

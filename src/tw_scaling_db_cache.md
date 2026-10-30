
# Plan

Important to **benchmark each architecture stage separately**, not just run `hey` once after everything is added.


**- Baseline**

```bash
hey -z 30s -c 50 http://localhost:8080/your-endpoint
```

Record:

* Requests/sec
* Average latency
* p50/p95/p99
* Errors

**- Scale API horizontally**

Run 2, 4, 8 API instances behind a load balancer and repeat the exact same test:

```text
1 instance → 2 → 4 → 8
```

**- Add DB**

Then test different concurrency:

```bash
hey -z 30s -c 10 ...
hey -z 30s -c 50 ...
hey -z 30s -c 100 ...
hey -z 30s -c 500 ...
```

DB connection-pool size becomes an important variable.  

**- Add caching**

Measure separately:

```text
cache hit
cache miss
```

For example:

```text
100% cache hit
90% hit / 10% miss
50% hit / 50% miss
0% hit
```

**- Find the saturation point**

Don't only ask *"what is my max RPS?"*.

Plot roughly:

```text
concurrency
    │
    │          ─────── throughput
    │       ──
    │    ──
    │ ──
    └────────────────────
```

Once RPS stops increasing while latency explodes, we've found a bottleneck.

## Benchmark matrix:

For every cell record **RPS + average latency + p50 + p95 + p99 + error rate**.

| Architecture       | c=10 | c=50 | c=100 | c=500 |
| ------------------ | ---: | ---: | ----: | ----: |
| 1 API              |      |      |       |       |
| 2 API              |      |      |       |       |
| 4 API              |      |      |       |       |
| 4 API + DB         |      |      |       |       |
| 4 API + DB + cache |      |      |       |       |


Once we introduce DB/cache and horizontal scaling, add **resource measurements** (CPU, memory, DB connections, DB CPU, cache hit rate).


## Key Points 

-  Add Warm-up Runs: Run a short 5–10 second load test before each recorded benchmark to ensure thread pools, JIT/compilers, and connection pools are warm.

- Introduce more realistic profiles:    
    Instead of only:

    ```text
    100 concurrent requests continuously for 30s
    ```

    use a profile that resembles actual traffic:

    ```text
    0–10s   → ramp from 0 → 50 users
    10–40s  → steady 50 users
    40–50s  → ramp 50 → 200
    50–80s  → steady 200
    80–90s  → ramp down
    ```

    And mix operations, e.g.:

    ```text
    70% GET /...
    20% POST /buy
    10% GET /metrics
    ```

    So we're testing **traffic patterns**, not just maximum sustained concurrency.


- Different workload types

    ```
    Read-heavy       → GET/query operations
    Write-heavy      → buy/order submission
    Mixed            → e.g. 80% reads + 20% buys
    Cache-friendly   → repeated requests for same data
    Cache-unfriendly → requests for mostly unique data
    ```
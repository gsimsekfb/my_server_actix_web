
# App Desctiption
(1) A tiny Rust/actix-web HTTP service implementing a priority-based order matching engine, built with per-field locking (`Mutex`, `RwLock`, `DashMap`, `AtomicU64`) for concurrent correctness.

(2) A high-performance order-matching system that leverages fine-grained synchronization and a sorted BTreeMap to efficiently process buy requests against a price-time prioritized bid book.

# Architecture

```          
                         Clients
                    ┌────────┴────────┐
                    │                 │
                 HTTP              gRPC
              (Actix Web)         (Tonic)
                    │                 │
                    ▼                 ▼
              ┌──────────┐      ┌──────────┐
              │ buy()    │      │ Buy()    │
              │ sell()   │      │ Sell()   │
              │allocation│      │GetAlloc()│
              └────┬─────┘      └────┬─────┘
                   │                 │
                   └────────┬────────┘
                            ▼
                       TwnEngine
                         (trait)
                            │
                 ┌──────────┴──────────┐
                 │                     │
                 ▼                     ▼
          InMemoryStorage        PostgresStorage
                 │                     │
          DashMap / BTreeMap       PostgreSQL
          AtomicU64 etc.            via sqlx
```          

# Key Topics Covered
1. Lock order - deadlock prevention enforced with fns and with AI pre-commit hook
   - see ordered_locks_*() fns
   - see [tw_ai_pre_commit_hook.txt](tw_ai_pre_commit_hook.txt)
2. Full separation between HTTP layer and business logic 
   - See buy and buy_impl fns
3. Granular per AppState field locks vs one Mutex for all AppState designs
   - See commit: "tw: Using lock/lockfree per AppState field instead of one 
     Mutex for all AppState"
4. Using Vec (manual sort) vs BTreeMap (auto sort) for sorted bids
5. Tests:
   - Unit tests - business logic
   - Property-based tests
   - Concurrency tests (spawn tasks that hammer /buy & /sell; assert invariants).
   - Black-box integration tests using the HTTP layer.

# Files Info:

[`tw_assignment.md`](tw_assignment.md)
   - main full spec doc.

[`tw_assignment_specs.md`](tw_assignment_specs.md)   
  - minimal summary of business logic doc.

[`tw_perf_testing.md`](tw_perf_testing.md)   
  - performance tests for production

[`tw_perf_optimization.md`](tw_perf_optimization.md)  

[`tw_design.md`](tw_design.md)  
  - self explanatory

[`tw_known_issues.md`](tw_known_issues.md)  
  - things omitted, possible improvements.

[`tw_main.rs`](tw_main.rs)  
  - main app implementation
  - important commits:
  - tw: Using lock/lockfree per AppState field instead of one Mutex for all AppState
  - refactor: replace bids Vec with BTreeMap
  - refactor: Full separation between HTTP layer and business logic (_impl fns and thus tests do not see mutex or Arc)

[`tw_buys.bat`](tw_buys.bat)  
  - batch script with buy requests from u1,u2,u3,u4 (from the example in full spec. md).

[`tw_load_test.ps1`](tw_load_test.ps1)  
  - 1000 buy requests, 1 sell and checks

[`tw_ai_pre_commit_hook.txt`](tw_ai_pre_commit_hook.txt) 
  - asserts code changes do not violate lock order to prevent deadlock.  

[`tw_openapi.yaml`](tw_openapi.yaml)  
  - self explanatory
  

[`tw_api_versioning.md`](tw_api_versioning.md)  
[`tw_auth.rs`](tw_auth.rs)  
[`tw_data_valid_error_handling.md`](tw_data_valid_error_handling.md)  
[`tw_debug.md`](tw_debug.md)  
[`tw_error.rs`](tw_error.rs)  
[`tw_err_circuit_breaker.rs`](tw_err_circuit_breaker.rs)  
[`tw_main_fn.rs`](tw_main_fn.rs)  
[`tw_main_v0.rs`](tw_main_v0.rs)  
[`tw_perf_optimization.md`](tw_perf_optimization.md)  


# Next
- Both for REST and gRPC - buy_impl/sell_impl results are ignored. If these operations can fail or return meaningful status, gRPC always reports success.
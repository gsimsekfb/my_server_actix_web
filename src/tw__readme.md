
### App Desctiption
(1) A tiny Rust/actix-web HTTP service implementing a priority-based order matching engine, built with per-field locking (`Mutex`, `RwLock`, `DashMap`, `AtomicU64`) for concurrent correctness.

(2) A high-performance order-matching system that leverages fine-grained synchronization and a sorted BTreeMap to efficiently process buy requests against a price-time prioritized bid book.

### Key Topics Covered
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

### Cheat sheet
```
cargo r --bin twin  // run server
cargo t --bin twin  // run tests
``` 
```
// hot reload
watchexec -e rs -r -- cargo run --bin twin
watchexec -e rs -r -- cargo run --bin twin --release
set RUST_LOG=actix_web=debug && watchexec -e rs -r -- cargo run --bin twin --release
```

```
// debug: shows AppState
curl -s localhost:8080
curl -is localhost:8080

// sell
curl -s -X POST localhost:8080/sell -H "Content-Type: application/json" -d "{\"volume\":250}"

// buys
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u1\",\"volume\":100,\"price\":3}"
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u2\",\"volume\":150,\"price\":2}"
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u3\",\"volume\":50,\"price\":4}"
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d "{\"user\":\"u4\",\"volume\":50,\"price\":4}"

// allocation
curl -s localhost:8080/allocation?username=u1
``` 


### Files quick summary:

tw_assignment.md
   - main full spec doc.

tw_assignment_specs.md  
  - minimal summary of business logic doc.

tw_design.md
  - self explanatory

tw_known_issues.md
  - things omitted, possible improvements.

tw_main.rs
  - main app implementation
  - important commits:
  - tw: Using lock/lockfree per AppState field instead of one Mutex for all AppState
  - refactor: replace bids Vec with BTreeMap
  - refactor: Full separation between HTTP layer and business logic (_impl fns and thus tests do not see mutex or Arc)

tw_buys.bat
  - batch script with buy requests from u1,u2,u3,u4 (from the example in full spec. md).

tw_load_test.ps1
  - 1000 buy requests, 1 sell and checks

tw_ai_pre_commit_hook.txt
  - asserts code changes do not violate lock order to prevent deadlock.  

tw_openapi.yaml
  - self explanatory
  
tw_perf_testing.md  
  - performance tests for production

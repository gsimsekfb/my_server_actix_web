## 1) Design Rationale

### Data Structures

Bids are stored in a `BTreeMap<(Reverse<u64>, u64), Bid>` keyed by `(Reverse(price), seq)`. This gives O(log n) insert on each `/buy` and O(n) iteration on `/sell`, compared to O(n log n) sort-on-insert with a `Vec`. The composite key encodes both priority and arrival order, so no explicit sorting is ever needed — the tree maintains invariants automatically. `Reverse` flips the natural ascending order so the highest-price bid is always the first element.

Allocations use `DashMap<String, u64>` — a sharded concurrent hash map providing O(1) reads and writes without a global lock. Since `/allocation` is read-only and `/buy`/`/sell` both write, a `RwLock<HashMap>` would also work, but `DashMap` reduces contention further under concurrent load.

### Concurrency Strategy

Each field is independently synchronized. `supply` is behind a `Mutex<u64>`, `bids` behind a `RwLock<BTreeMap>`, and `buy_seq_no` is an `AtomicU64`. Since `/buy` and `/sell` must update `supply` and `bids` together atomically, both locks are always acquired via dedicated `ordered_locks_buy` and `ordered_locks_sell` functions that enforce a fixed lock order (`supply` before `bids`). This eliminates deadlock risk regardless of how many handlers run concurrently.

### Tie-Break Approach

Each `/buy` request atomically increments `buy_seq_no` and embeds the resulting sequence number into the `BTreeMap` key. Since `AtomicU64::fetch_add` is globally ordered, no two bids can receive the same sequence number even under concurrent load. Within a price level, the `BTreeMap` sorts by `seq` ascending, so earlier arrivals always appear first and fill first.

### Failure Modes

- **Mutex poison**: if a thread panics while holding `supply` or `bids`, subsequent `lock()` calls return a `PoisonError`. Currently handled via `unwrap()` — a production system would recover (e.g. to a stable state) or restart.
- **No persistence**: all state is in-memory. A crash loses all bids and allocations.
- **`retain` visits all bids**: once supply hits zero, remaining iterations are no-ops. A `while let Some(...) = bids.pop_first()` loop would exit early, but `retain` was chosen for clarity.
- **Integer overflow**: `volume` and `supply` are `u64`; no overflow protection beyond Rust's debug-mode panic on arithmetic overflow.
    > - Checked arithmetic — use `checked_add`/`checked_sub` returning Option, return 4xx if overflow would occur
    > - Input validation — reject unreasonably large `volume` or `price` values at the API boundary
    > - Monitoring — alert when values approach limits

---

## 2) Decision Log

**[1] BTreeMap over Vec for bids**
Chose `BTreeMap<(Reverse<u64>, u64), Bid>` over `Vec<Bid>`. Vec required O(n log n) sort on every insert; BTreeMap gives O(log n) insert and maintains order automatically. Composite key encodes price priority and FIFO order without extra sorting logic.

**[2] Per-field locking over single `Mutex<Inner>`**
Chose individual locks (`Mutex<supply>`, `RwLock<bids>`, `DashMap<allocations>`, `AtomicU64`) over a single `Mutex<AppStateImpl>`. More complex but reduces contention — `/allocation` never blocks `/buy` or `/sell`. Mitigated deadlock risk with enforced lock-order functions `ordered_locks_buy` / `ordered_locks_sell`.

**[3] DashMap over RwLock<HashMap> for allocations**
`DashMap` provides finer-grained sharded locking with no external lock management. `/allocation` reads and `/buy`/`/sell` writes can proceed with minimal contention. Tradeoff: external dependency, slightly less transparent than stdlib types.

**[4] AtomicU64 for sequence number over Mutex**
`buy_seq_no` is a single integer incremented on every `/buy`. `AtomicU64::fetch_add` is lock-free and sufficient — no need to protect it behind a mutex. Guarantees strict monotonic ordering even under concurrent load.

**[5] `retain` over `pop_first` loop in sell_impl**
`retain` is more readable and idiomatic for filter-in-place. Tradeoff: no early exit — iterates all bids even after supply hits zero. `pop_first` loop would exit early but requires manually re-inserting partial fills. Chose clarity over micro-optimization at this scale.

**[6] In-memory only, no persistence**
Per assignment requirements. Known limitation: full state lost on crash. Production solution would be a write-ahead log or event sourcing to replay state on restart.
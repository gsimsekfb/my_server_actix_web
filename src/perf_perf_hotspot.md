# Profiling w/ perf & Hotspot

## Contents
1. [Steps to get perf.data](#1-steps-to-get-perfdata)
2. [Perf Report - Profiling Twin Program](#2-perf-report---profiling-twin-program)
3. [Line by Line Profiling](#3-line-by-line-profiling)
4. [Hotspot](#4-hotspot)
5. [Profiling Results](#5-profiling-results)


## 1. Steps to get perf.data

Profiling "twin" executable:
```bash
RUSTC_WRAPPER="" CARGO_PROFILE_RELEASE_DEBUG=true cargo r--release --bin twin

perf record -F 999 -g -p $(pgrep twin) -- sleep 10
    // perf might be here: 
    // /usr/lib/linux-tools/5.15.0-186-generic/perf
    // -F 999: sampling freq. 999 Hz (samples/sec)
    // `-g` captures call graphs (stack traces) so you get the full call
    // chain not just the top frame,
    // `sleep 10` makes `perf` sample for exactly 10 seconds

// run 10 secs of traffic
hey -c 10 -z 10s -m POST -d '{"user":"u1","volume":10,"price":3}' -H "Content-Type: application/json" http://localhost:8080/buy

// after 10 secs, perf.data file should be created
-rwxrwxrwx 1 gok 3.0M Aug  1 15:12 perf.data

done
```
[↑ Back to top](#contents)


## 2. perf report - profiling twin program

```
perf report   
    // looks for `.\perf.data` by default;   
    // also use `perf report -i /path/perf.data`

// hint: maximize window for better view
```
```
Samples: 19K of event 'cycles', Event count (approx.): 40744335545
  Children      Self  Command          Shared Object      Symbol
+   48.45%     0.03%  actix-rt|system  [kernel.kallsyms]  [k] 0xffffffff9f200134
+   47.30%     0.05%  actix-rt|system  [kernel.kallsyms]  [k] 0xffffffff9f0785aa
+   37.94%     0.16%  actix-rt|system  twin               [.] <&mio::net::tcp::stream::TcpStream as st
+   37.75%     0.27%  actix-rt|system  libc.so.6          [.] __send

...

+    1.20%     1.11%  actix-rt|system  twin               [.] actix_hello::tw_main::buy_impl        
     1.20%     1.18%  actix-rt|system  twin               [.] <actix_http::requests::request::Reques
+    1.18%     1.18%  actix-rt|system  libc.so.6          [.] malloc                                
     1.12%     0.93%  actix-rt|system  twin               [.] actix_web::handler::handler_service::

```

[↑ Back to top](#contents)


## 3. Line by line profiling

`perf annotate <symbol>` filters the recorded samples down to just that one function and shows its disassembly (with source lines interleaved, if debug info exists), with each instruction/line tagged by the % of samples that landed on it — so it focuses narrowly on that one function's internal cost breakdown, not its callers or callees.

Hint: maximize window for better view
```
perf annotate --symbol=actix_hello::tw_main::buy_impl

or

perf report      // opens ./perf.data
Use key "/" to search "buy_impl", enter on the found line, and annotate to get the view below
```

```
Samples: 217  of event 'cycles', 999 Hz, Event count (approx.): 450696749
actix_hello::tw_main::buy_impl  /mnt/c/code/be/actix_hello/target/release/twin [Percent: local period]
Percent│                                                                               
       │     Disassembly of section .text:                                             
       │                                                                                      │     00000000003e4340 <actix_hello::tw_main::buy_impl>:                               │     actix_hello::tw_main::buy_impl:                                           
       │     ///   later.
       │     ///
       │     /// Big O: log N - btreemap insert
       │     ///
       │     #[instrument(skip(buy_seq_no, supply, allocations, bids))]
       │     pub fn buy_impl(
  0.84 │       push   %rbp     
  
...

       │397  alloc::collections::btree::search::<impl alloc::collections::btree::node::NodeRef<Borrow▒
 34.38 │       mov    %esi,%edx
  0.91 │       shl    $0x4,%edx
       │       lea    (%rdx,%rdx,2),%rdi
  0.42 │       mov    $0xffffffffffffffff,%rdx
       │       xor    %r8d,%r8d
       │       mov    %rax,%r9 
       │     ↓ jmp    379      
       │232  core::tuple::<impl core::cmp::Ord for (U,T)>::cmp:
  4.09 │360:   seta   %r10b    
  0.55 │       sbb    $0x0,%r10b        
       │232  actix_hello::tw_main::buy_impl:                                                         ◆
  1.62 │       add    $0x10,%r9
       │234  alloc::collections::btree::search::<impl alloc::collections::btree::node::NodeRef<Borrow
       │       add    $0x30,%r8
  5.46 │       inc    %rdx     
  0.40 │       cmp    $0x1,%r10b        
       │     ↓ jne    390      
       │230  <core::ptr::non_null::NonNull<T> as core::cmp::PartialEq>::eq:
  3.10 │379:   cmp    %r8,%rdi 
       │1718 <core::slice::iter::Iter<T> as core::iter::traits::iterator::Iterator>::next:
       │     ↓ je     3a0      
       │181  core::tuple::<impl core::cmp::Ord for (U,T)>::cmp:          
  1.07 │       cmp    %r14,(%r9)                                         
       │     ↑ jne    360                                                
 13.55 │       cmp    0x8(%r9),%r15                                                                

 ...                           
                                            
```

AI analysis (Claude Sonnet 5):

~66% of samples are in the `BTreeMap` key search loop (`find_key_index`, the linear scan comparing `(Reverse<u64>, u64)` tuples: 34.4% + 13.6% + 5.5% + 4.1% + 2.6% + others), confirming the O(log n) insert's constant factor — specifically tuple comparison and index scanning inside each B-tree node — dominates `buy_impl`, while `tracing`/`#[instrument]` span setup/teardown adds a smaller but non-trivial ~10-12% overhead (span creation, log-enabled checks, drop).

Notable: no single line is a leak — the cost is spread across the B-tree node's linear key scan (up to ~11 keys per node in Rust's B-tree), which matches your `tw_perf_optimization.md` note about `bids` being the next optimization target (DashMap swap).

[↑ Back to top](#contents)


## 4. Hotspot

```bash
hotspot perf.data
```
Note:  
Hotspot vs `flamegraph`/`cargo-flamegraph`:  

Hotspot gives a fuller interactive exploration of the same perf.data (call trees, source annotation, timelines, and a flamegraph view too), while `cargo-flamegraph` just automates generating a single flamegraph SVG — so Hotspot is a superset of what flamegraph shows, with more views but more manual setup (you still run `perf record` yourself, then open the file in Hotspot).

![alt text](hotspot-perf-files\hotspot-summary-tab.png)
---  
![alt text](hotspot-perf-files\hotspot-bottomup-tab.png)
---  
![alt text](hotspot-perf-files\hotspot-bottomup-buy.png)
---  
![alt text](hotspot-perf-files\hotspot-topdown-tab.png) 
---  
![alt text](hotspot-perf-files\hotspot-topdown-buy.png) 
---  
![alt text](hotspot-perf-files\hotspot-flamegraph-tab.png)  

* **X-axis = proportion of samples**, not time — width of a box shows what % of total cycles that function consumed, sorted alphabetically among siblings (not chronological).
* **Y-axis = call stack depth** — bottom row is where sampling started (e.g., thread entry), each row up is one level deeper into nested function calls.
* **Wider box = more expensive** (inclusive of everything it calls); a wide box with little going up from it means the cost is in that function itself, not its children.
* **Click a box to zoom** into that subtree (rescales it to full width so you can inspect a specific hot path); click the background to reset zoom.
* **"Bottom-Up View" checkbox** (top of the tab) flips it to show which functions are hot when aggregated regardless of caller — useful for "what's expensive everywhere" vs. the default "where does time go from main() down."

---  
![alt text](hotspot-perf-files\hotspot-caller-tab.png) 
---  
![alt text](hotspot-perf-files\hotspot-caller-buy.png)   


[↑ Back to top](#contents)

## 5. Profiling Results

Based on this profiling session:

1. **Tracing overhead** — `#[instrument]` on `buy_impl` adds \~10-12% overhead per the earlier `perf annotate` output; gate it behind a feature flag or remove it from the hot path in production builds.
2. **BTreeMap key comparison dominates real business logic** — confirms your `tw_perf_optimization.md` "Next" item (swap `bids` to DashMap/lock-free) is correctly prioritized, not premature.
3. **Symbol resolution is too degraded to trust further micro-conclusions** — `twin` is missing 63/4885 debug symbols and kernel symbols aren't resolving at all in WSL2; before chasing more leads (like the `send`/socket buffer confusion), rebuild with `CARGO_PROFILE_RELEASE_DEBUG=true` and `debug-assertions=false`, and try `sudo sysctl kernel.kptr_restrict=0` to get real kernel-space attribution.
4. **Verify `hey`'s connection reuse** — if it's opening a new TCP connection per request instead of keep-alive, that would explain inflated `send`/socket-related syscall percentages that have nothing to do with `buy_impl` itself; add `-disable-keepalive=false` (default) confirmation or check with `hey`'s docs, since this could be inflating your load test numbers independent of server code quality.
5. **Allocation churn** (`malloc`/`free`/`dealloc_nonnull` \~3-4% combined) — `user.clone()` in `buy_impl` on every CAS retry is avoidable; consider `Arc<str>` for `user` to make clones cheap, or restructure to clone once.

Item 2 is your highest-leverage change; items 1 and 5 are cheap wins; item 3 should happen before trusting any more profiling data. 

[↑ Back to top](#contents)

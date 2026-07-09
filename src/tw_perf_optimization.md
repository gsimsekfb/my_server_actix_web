
## Main Goals

- Maximize throughput 
- Minimize P99 latency 

under concurrent load while maintaining correctness guarantees.


## Environment

```
Surface Pro 7, 16 GB
$ lscpu | grep -E "Model name|CPU\(s\)"
CPU(s):                 8
On-line CPU(s) list:    0-7
Model name:             Intel(R) Core(TM) i7-1065G7 CPU @ 1.30GHz
NUMA node0 CPU(s):      0-7

$ uname -r
6.6.87.2-microsoft-standard-WSL2
```


## Next

- Replace `RwLock<BTreeMap>` on `bids` with a lock-free structure like `DashMap` or a channel-based design. DashMap uses a HashMap internally so we lose the sorted order that BTreeMap provides — we'd need to sort manually when processing bids. BTreeMap maintains sorted order automatically on every insert - O(log n), while DashMap is unsorted so we'd pay O(n log n) to sort at sell time instead.



## Optimizations

```
// Server   
RUSTC_WRAPPER="" cargo r --release --bin twin

// Client  
hey -c 10 -z 10s -m POST -d '{"user":"u1","volume":10,"price":3}' -H "Content-Type: application/json" http://localhost:8080/buy
```


```

v4
  Requests/sec: 51780
  99% in 0.0005 secs
  Change: AppState.supply is now AtomicU64 instead of Mutex<u64> 

v3
  Requests/sec: 44130
  99% in 0.0006 secs
  Change: Revert: Increase workers count from 2 to 8. 
          Workers count is 2 again.
          HttpServer::new(move || { App::new()}).workers(2) 

v2
  Requests/sec: 40232
  99% in 0.0006 secs
  Change: Increase workers count from 2 to 8 
          HttpServer::new(move || { App::new()}).workers(8) 

v1  
  Requests/sec: 45208
  99% in 0.0007 secs  
  Change: Removed Logger, unused Middleware fn and `dbg!()` line
          `dbg!(has_feature_new_alloc);`

v0  
  Requests/sec: 18887  
  99% in 0.0012 secs  
```


## Details

v0
```
hey -c 10 -z 10s -m POST -d '{"user":"u1","volume":10,"price":3}' -H "Content-Type: application/json" http://localhost:8080/buy

Summary:
  Total:        10.0052 secs
  Slowest:      0.0175 secs
  Fastest:      0.0001 secs
  Average:      0.0016 secs
  Requests/sec: 6148.1335

  Total data:   18811872 bytes
  Size/request: 305 bytes

Response time histogram:
  0.000 [1]     |
  0.002 [41407] |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
  0.004 [15780] |■■■■■■■■■■■■■■■
  0.005 [3203]  |■■■
  0.007 [815]   |■
  0.009 [212]   |
  0.011 [59]    |
  0.012 [27]    |
  0.014 [4]     |
  0.016 [2]     |
  0.017 [3]     |


Latency distribution:
  10% in 0.0005 secs
  25% in 0.0008 secs
  50% in 0.0013 secs
  75% in 0.0021 secs
  90% in 0.0031 secs
  95% in 0.0040 secs
  99% in 0.0061 secs

Details (average, fastest, slowest):
  DNS+dialup:   0.0000 secs, 0.0001 secs, 0.0175 secs
  DNS-lookup:   0.0000 secs, 0.0000 secs, 0.0008 secs
  req write:    0.0000 secs, 0.0000 secs, 0.0076 secs
  resp wait:    0.0014 secs, 0.0000 secs, 0.0174 secs
  resp read:    0.0001 secs, 0.0000 secs, 0.0096 secs

Status code distribution:
  [200] 61513 responses
```

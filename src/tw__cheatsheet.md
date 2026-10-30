
# Cheat sheet
```
cargo r --bin twn  // run server
cargo t --bin twn  // run tests

cargo r --bin twn --release --features disable_logs 
  // e.g. for perf tests
cargo r --bin twn --release --features enable_tracing_spans 
``` 

```
// hot reload
watchexec -e rs -r -- cargo run --bin twn
watchexec -e rs -r -- cargo run --bin twn --release
set RUST_LOG=actix_web=debug && watchexec -e rs -r -- cargo run --bin twn --release
```

```
// debug: shows AppState
curl -s localhost:8080
curl -is localhost:8080

// sell
curl -s -X POST localhost:8080/sell -H "Content-Type: application/json" -d '{"volume":250}'

// buys
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d '{"user":"u1","volume":100,"price":3}'
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d '{"user":"u2","volume":150,"price":2}'
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d '{"user":"u3","volume":50,"price":4}'
curl -s -X POST http://localhost:8080/buy -H "Content-Type: application/json" -d '{"user":"u4","volume":50,"price":4}'

// allocation
curl -s localhost:8080/allocation?username=u1
``` 




# Observability

```
Application
   │
   ├── metrics ──► Prometheus
   ├── traces  ──► OpenTelemetry/collector
   └── logs    ──► logging system
```


## Traces

Function level
```
Trace: a066... (one POST /buy request)
└── Span: buy               (527µs)
    └── Span: buy_impl       (244µs)
```

Services level
```
GET /checkout                 2000ms
├─ Span: User Service           40ms
├─ Span: Inventory Service      80ms
└─ Span: Payment Service      1850ms  ← problem
```

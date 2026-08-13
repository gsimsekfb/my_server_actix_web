# Cross Service Communication Architectures

| Approach | Protocol/Format | Latency | Coupling | Use Case |
|----------|-----------------|---------|----------|----------|
| REST/JSON | HTTP/1.1, text | Medium | Loose | Public APIs, simple CRUD |
| gRPC | HTTP/2, binary protobuf | Low | Tighter (shared .proto) | Internal microservices, high throughput |
| GraphQL | HTTP/1.1, text | Medium | Loose | Client-facing, flexible queries |
| Message Queue (Kafka, RabbitMQ) | Binary/text, async | N/A (async) | Loose | Event-driven, decoupled services |
| WebSockets | HTTP/1.1 upgrade | Low | Loose | Real-time bidirectional (chat, live feeds) |
| Unix sockets / Shared memory | Binary | Very low | Tight (same host) | Same-host high-perf IPC |
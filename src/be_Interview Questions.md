
# Top 3 Most Asked Interview Topics

Across entry-to-mid level backend technical rounds, these three topics account for the majority of core questions:

1. **Database Design, SQL & Query Optimization:**
* *Common questions:* SQL vs. NoSQL, indexing trade-offs, `EXPLAIN` query analysis, transactions/isolation levels, and connection pooling.
2. **API Design & Architecture (REST / gRPC):**
* *Common questions:* HTTP methods, status codes, idempotent vs. safe requests, pagination strategies (Cursor vs. Offset), and error payload structures.
3. **Caching Strategies & In-Memory Storage (Redis):**
* *Common questions:* Cache-aside pattern, cache invalidation, dealing with cache stampedes/thundering herds, and cache eviction policies (LRU, LFU).


**Top 3 most-asked in backend interviews:**   
(1) DB design/SQL — indexing, transactions, N+1 queries;   
(2) caching strategies — cache invalidation, TTL, write-through vs write-back;   
(3) system design/scalability — horizontal scaling, load balancing, and where to add caching/queues under load.  


-------------  


# Questions

## Pagination strategies (Cursor vs. Offset)

**Offset:**
```
GET /items?offset=40&limit=20
```

**Cursor:**
```
GET /items?after=item_123&limit=20
```

 Offset pagination uses a page/skip number (e.g., "skip 40, take 20") but gets slow and inconsistent on large/changing datasets, 
 
 Cursor pagination uses a pointer to the last seen item (e.g., "give me 20 after ID 123") for consistent, efficient results on large or frequently updated data.


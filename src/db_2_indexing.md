
# DB Indexing 

## Quick Summary - Indexing Trade-offs 

| Aspect | Without Index | With Index |
|:---|:---|:---|
| Read/query speed | Slow — full table scan, O(n) | Fast — direct lookup, O(log n) |
| Write speed (insert/update/delete) | Fast — append only, O(1) | Slower — must find position + rebalance, O(log n) |
| Storage cost | Low — only the table data | Higher — index itself takes disk space |
| Best used for | Rarely-queried or write-heavy columns | Frequently filtered/joined/sorted columns (e.g. `WHERE`, `JOIN`, `ORDER BY`) |
| Risk of overuse | N/A | Too many indexes slow down writes and bloat storage |

\* About O(log n)s, because most indexes are implemented as a B-tree (a balanced, sorted tree structure).


## Explain & Explain Analyze

- `EXPLAIN` — hits server for planning only, no data read/written
- `EXPLAIN ANALYZE` — hits server AND executes fully, reads data
- `EXPLAIN ANALYZE` on `INSERT/UPDATE/DELETE` — **actually modifies data**

Staging/dev:

```sql
BEGIN;
EXPLAIN ANALYZE INSERT INTO ...;
ROLLBACK;  -- undo the actual execution
```

### Explain


`EXPLAIN` shows how the database **executes a query** — which indexes it uses, how many rows it scans, join order etc.

```sql
EXPLAIN SELECT * FROM users WHERE email = 'alice@example.com';
```

Output (PostgreSQL):
```
Seq Scan on users  (cost=0.00..25.00 rows=1 width=100)
  Filter: (email = 'alice@example.com')
```

`Seq Scan` means it's scanning every row — no index. If you add an index:

```sql
CREATE INDEX ON users(email);
EXPLAIN SELECT * FROM users WHERE email = 'alice@example.com';
```

Output changes to:
```
Index Scan using users_email_idx on users  (cost=0.00..8.00 rows=1 width=100)
```

Much cheaper. `EXPLAIN` is how you diagnose slow queries.  

-----------

PostgreSQL specific:

- **cost** — `0.00..25.00` — estimated startup cost .. total cost (in arbitrary units)
- **rows** — estimated number of rows returned
- **width** — estimated average row size in bytes

```
Seq Scan on users  (cost=0.00..25.00 rows=2 width=100)
```

- `0.00` — cost to return first row
- `25.00` — cost to return all rows
- `rows=2` — expects 2 rows back
- `width=100` — each row ~100 bytes

These are **estimates** based on table statistics, not exact values. Use `EXPLAIN ANALYZE` to get actual measured values alongside estimates.


### Explain Analyze

```sql
EXPLAIN ANALYZE SELECT * FROM users WHERE email = 'alice@example.com';
```
```
Index Scan using users_email_key on users (cost=0.14..8.16 rows=1 width=552) (actual time=0.600..0.700 rows=0 loops=1)
Index Cond: ((email)::text = 'alice@example.com'::text)
Planning Time: 17.200 ms
Execution Time: 3.500 ms
```

- **Index Scan using users_email_key on users** - using the index on email column
- **cost=0.14..8.16** — estimated cost (startup..total)
- **rows=1** — estimated 1 row returned
- **width=552** — estimated row size 552 bytes
- **actual time=0.600..0.700** — real measured time in milliseconds (first row..last row)
- **rows=0** — actually returned 0 rows (no matching email in table)
- **loops=1** — this node executed once
- **Planning Time: 17.200 ms** — time to build the execution plan
- **Execution Time: 3.500 ms** — actual query execution time 

Notable: `rows=1` estimated but `rows=0` actual — the table is empty or has no matching row, so the estimate was wrong.


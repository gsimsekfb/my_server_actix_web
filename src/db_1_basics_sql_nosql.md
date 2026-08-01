

# SQL vs NoSQL


**Important Note:**  
These are general tendencies rather than absolute rules, so it is not like "MongoDB (NoSQL) can't be consistent" or "Postgres (SQL) can't scale out."

## A. Explaining Main Differences


| # | SQL                        | NoSQL                  |
|:--|:---------------------------|:-----------------------|
| 1 | Structured schema          | Flexible               |
| 2 | Relational data            | Denormalized           |
| 3 | Strict consistency         | Eventual consistency   |

and as a result of these:  

| # | SQL                        | NoSQL                  |
|:--|:---------------------------|:-----------------------|
| 1 | Optimized for correctness  | Optimized for scale    |



### Differences
[1. Structured vs Flexible](#1--structured-vs-flexible)   
[2. Relational vs Denormalized](#2--relational-vs-denormalized)   
[3. Strict consistency vs Eventual consistency](#3--strict-consistency-vs-eventual-consistency)  


### 1- Structured vs Flexible

**SQL - "Structured" data**  

- must fit a predefined schema 
- fixed columns with fixed types
- enforced by the database itself

e.g. 

```sql
orders:  
id  | user_id | total | id | name
101 | 1       | 59.98 | 1  | Alice
102 | 2       | 15.00 | 2  | Bob

CREATE TABLE orders (
  id INTEGER PRIMARY KEY,
  user_id INTEGER,
  total DECIMAL
  name TEXT
);
```

`orders` requires every row to have `id` (integer), `user_id` (integer), `total` (decimal), name (text) — you can't insert an order missing `user_id` or with `total="fifty dollars"`, the DB rejects it.


**NoSQL - Flexible data**

- has no such enforcement 

e.g.  

One chat document could have a `messages` array, another could add a `reactions` field, and the DB doesn't care.

chat JSON document:  
```json
[
  {
    "chat_id": "c1",
    "members": ["Alice", "Bob"],
    "messages": [
      {"from": "Alice", "text": "hey", "ts": 1690000000},
      {"from": "Bob",   "text": "hi!", "ts": 1690000005}
    ]
  },
  {
    "chat_id": "c2",
    "members": ["Alice", "Carol"],
    "messages": [
      {"from": "Carol", "text": "lunch?", "ts": 1690001000},
      {"from": "Alice", "text": "sure",   "ts": 1690001010}
    ]
  }
]
```

[↑ Back to top](#differences)

NoSQL also includes Key-Value (Redis), Wide-Column (Cassandra), and Graph databases (Neo4j), as denormalization works differently across them.


## 2- Relational vs Denormalized

SQL - "Relational" data
- data is split into separate tables linked by foreign keys, 
- so no duplication — each fact lives in exactly one place
- joins reassemble related data at query time

e.g.

We want `orders.user_id` to reference `users.id` — Alice's name is stored once in `users`, and every order just points to it via `user_id`. 


```sql
users:  

id | name
------------
1  | Alice
2  | Bob

orders:

id  | user_id | total 
----------------------
101 | 1       | 59.98 
102 | 2       | 15.00 
```

Setup cmds:  

```sql
CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  name TEXT
);

CREATE TABLE orders (
  id INTEGER PRIMARY KEY,
  user_id INTEGER,
  total DECIMAL,
  FOREIGN KEY (user_id) REFERENCES users(id)  -- relation set up here
);

INSERT INTO users (id, name) VALUES (1, 'Alice'), (2, 'Bob');
INSERT INTO orders (id, user_id, total) VALUES (101, 1, 59.98), (102, 2, 15.00);
```

Join (reassembles the full picture):  

```sql
SELECT * FROM orders JOIN users ON orders.user_id = users.id;

id  | user_id | total | id | name
101 | 1       | 59.98 | 1  | Alice
102 | 2       | 15.00 | 2  | Bob
```


NoSQL - "Denormalized" data
- related data is embedded directly inside one document
- duplication is expected and fine
- no joins — everything needed is already nested together

e.g.

chats JSON document/collection:  
```json
[
  {
    "chat_id": "c1",
    "members": ["Alice", "Bob"],
    "messages": [
      {"from": "Alice", "text": "hey", "ts": 1690000000},
      {"from": "Bob",   "text": "hi!", "ts": 1690000005}
    ]
  },
  {
    "chat_id": "c2",
    "members": ["Alice", "Carol"],
    "messages": [
      {"from": "Carol", "text": "lunch?", "ts": 1690001000},
      {"from": "Alice", "text": "sure",   "ts": 1690001010}
    ]
  }
]
```

`members` and `messages` live inside the same chat document — no separate "users" collection to join against, everything you need to render this chat is right there in one read.


[↑ Back to top](#differences)



### 3- Strict consistency vs Eventual consistency

It's primarily about the **replication/multi-node level** — how quickly a write propagates to all copies of the data and whether reads can be stale.

**SQL - "Strict consistency"**

- writes are confirmed only after replicas acknowledge them (synchronous replication)
- every read, from any node, sees the latest committed write
- trades write speed for guaranteed accuracy

e.g.

```sql
-- primary waits for replica to confirm before COMMIT returns success
UPDATE orders SET total = 59.98 WHERE id = 101;
COMMIT;

SELECT total FROM orders WHERE id = 101;
-- returns 59.98 immediately, from any node
```

Once `COMMIT` returns, the write is guaranteed to be everywhere — there's no window where a different node still shows the old value.

Note: SQL isn't strictly synchronous by default: Most SQL databases run with asynchronous replication out of the box to keep write speeds high. Strict consistency across nodes usually requires explicit configuration (e.g., synchronous replication, consensus protocols like Raft).  


**NoSQL - "Eventual consistency"**

- writes are confirmed immediately by the primary, replicas catch up in the background (asynchronous replication)
- a read hitting a not-yet-synced replica can return stale data
- trades guaranteed accuracy for write speed

e.g.

```json
// write confirmed instantly on primary:
{"chat_id": "c1", "messages": [..., {"from": "Bob", "text": "hi!"}]}

// read hits a replica a moment later, still catching up:
{"chat_id": "c1", "messages": [...]}
// "hi!" not visible yet — still propagating from primary
```

The write succeeds right away, but it takes a short time to reach every replica — reads during that window may miss it.

**Important Note:**  
These are general tendencies rather than absolute rules, so it is not like "MongoDB (NoSQL) can't be consistent" or "Postgres (SQL) can't scale out."  

NoSQL does support strong consistency: Many modern NoSQL databases (like MongoDB or DynamoDB) allow you to request strongly consistent reads or single-document ACID transactions when needed.  

[↑ Back to top](#differences)


## B. Summary


**SQL (tables, relational data):**

```sql
users:  
id | name
------------
1  | Alice
2  | Bob

orders:
id  | user_id | total 
----------------------
101 | 1       | 59.98 
102 | 2       | 15.00 
```

**NoSQL (documents, JSON, chats collection):**
```json
[
  {
    "chat_id": "c1",
    "members": ["Alice", "Bob"],
    "messages": [
      {"from": "Alice", "text": "hey", "ts": 1690000000},
      {"from": "Bob",   "text": "hi!", "ts": 1690000005}
    ]
  },
  {
    "chat_id": "c2",
    "members": ["Alice", "Carol"],
    "messages": [
      {"from": "Carol", "text": "lunch?", "ts": 1690001000},
      {"from": "Alice", "text": "sure",   "ts": 1690001010}
    ]
  }
]
```


| # | DB Property        | SQL                        | NoSQL                          |
|:--|:--------------------|:-----------------------------|:---------------------------------|
| 1 | Consistency| Strict| Eventual|
| 2 | Scalability| *Vertical<br>(optimized for correctness) | Horizontal<br>(optimized for scale) |
| 3 | Availability| Lower<br>(locks/transactions can block) | Higher<br>(built for uptime under failures) |
| 4 | Read/write speed| Slower on complex joins| Faster on simple key-based ops  |
| 5 | Schema flexibility| Structured/fixed| Flexible                        |

*Modern SQL scales horizontally

### Top 5 DB properties by importance for modern systems:

1. **Consistency guarantees** — does every read see the latest write, or can it be stale? (determines correctness for things like payments/inventory)
2. **Scalability** — can it grow horizontally (more machines) vs only vertically (bigger machine) as data/traffic grows?
3. **Availability** — does it stay up and responsive during node failures or network issues?
4. **Read/write speed (latency & throughput)** — how fast are queries, and how many ops/sec can it handle?
5. **Schema flexibility** — how easily can the data model evolve without migrations/downtime?

Note: consistency, availability, and partition tolerance trade off against each other (CAP theorem) — no DB maxes out all three at once, which is exactly why SQL and NoSQL make different tradeoffs.

# A. Cheatsheet


```sh    

#### ---------
export DATABASE_URL="postgres://twn:twn@localhost:5432/twn"
cr --bin twn --features postgres
curl -s localhost:8080/allocation?username=u55

#### --------- DB
psql "postgres://twn:twn@localhost:5432/twn"
\dt
SELECT * FROM allocations;
INSERT INTO allocations (username, volume) VALUES ('u1', 100);

# Deploy tables using migration file instead of manial cmds
sqlx migrate add init_schema
    Creating migrations/20260917095201_init_schema.sql
sqlx migrate run
```    


# B. Manual DB Setup

### 1. Connect with `psql`

```bash
psql "postgres://twn:twn@localhost:5432/twn"
```

---

### 2. Run the schema by hand

```sql
CREATE TABLE bids (
    id          BIGSERIAL PRIMARY KEY,
    username    TEXT NOT NULL,
    price       BIGINT NOT NULL,
    volume      BIGINT NOT NULL,
    seq         BIGINT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_bids_price_seq ON bids (price DESC, seq ASC) WHERE volume > 0;

CREATE TABLE allocations (
    username    TEXT PRIMARY KEY,
    volume      BIGINT NOT NULL DEFAULT 0
);

CREATE TABLE supply_state (
    id       INT PRIMARY KEY DEFAULT 1,
    supply   BIGINT NOT NULL DEFAULT 0,
    buy_seq  BIGINT NOT NULL DEFAULT 0,
    CHECK (id = 1)
);
INSERT INTO supply_state (id, supply, buy_seq) VALUES (1, 0, 0);
```

---

### 3. Check tables exist

```sql
\dt
\d bids
\d supply_state
```

---

### 4. Insert sample rows

```sql
INSERT INTO bids (username, price, volume, seq) VALUES
    ('u1', 3, 100, 1),
    ('u2', 2, 150, 2),
    ('u3', 4, 50,  3);

UPDATE supply_state SET buy_seq = 3 WHERE id = 1;
```

---

### 5. Run your actual queries

```sql
-- simulate a /sell of 250 — highest price first, FIFO within price
SELECT id, username, volume FROM bids WHERE volume > 0
ORDER BY price DESC, seq ASC
FOR UPDATE;

-- add supply
UPDATE supply_state SET supply = supply + 250 WHERE id = 1;

-- fill u3 (50 @ 4) fully
UPDATE supply_state SET supply = supply - 50 WHERE id = 1;
UPDATE bids SET volume = 0 WHERE username = 'u3';
INSERT INTO allocations (username, volume) VALUES ('u3', 50)
    ON CONFLICT (username) DO UPDATE SET volume = allocations.volume + 50;

-- fill u1 (100 @ 3) fully
UPDATE supply_state SET supply = supply - 100 WHERE id = 1;
UPDATE bids SET volume = 0 WHERE username = 'u1';
INSERT INTO allocations (username, volume) VALUES ('u1', 100)
    ON CONFLICT (username) DO UPDATE SET volume = allocations.volume + 100;

-- partial fill u2 (150 @ 2, only 100 supply left)
UPDATE supply_state SET supply = 0 WHERE id = 1;
UPDATE bids SET volume = volume - 100 WHERE username = 'u2';
INSERT INTO allocations (username, volume) VALUES ('u2', 100)
    ON CONFLICT (username) DO UPDATE SET volume = allocations.volume + 100;

-- verify
SELECT * FROM allocations;
SELECT * FROM bids WHERE volume > 0;  -- u2 should show 50 remaining
```

---

### (TODO: double check) 6. Verify indexing with `EXPLAIN`

It verifies that order-matching query (highest price first, then oldest seq) is served straight from the index instead of reading and sorting the whole bids table on every match.

```sql
EXPLAIN SELECT * FROM bids WHERE volume > 0 ORDER BY price DESC, seq ASC;
```

Look for `Index Scan using idx_bids_price_seq` — not `Seq Scan`.  

```sql
twn=# EXPLAIN SELECT * FROM bids WHERE volume > 0 ORDER BY price DESC, seq ASC;
                          QUERY PLAN
--------------------------------------------------------------
 Sort  (cost=31.03..31.70 rows=270 width=72)
   Sort Key: price DESC, seq
   ->  Seq Scan on bids  (cost=0.00..20.12 rows=270 width=72)
         Filter: (volume > 0)
(4 rows)
```
Plan showed Seq Scan + Sort.  
That’s expected at ~270 rows, where the planner thinks a seq scan is cheaper.  
Only the SET enable_seqscan = off; test shows whether the index can serve the query.  
With much more data, the planner should switch to the index on its own.  

More info: 

```sql
twn=# \d bids
                                       Table "public.bids"
   Column   |           Type           | Collation | Nullable |             Default
------------+--------------------------+-----------+----------+----------------------------------
 id         | bigint                   |           | not null | nextval('bids_id_seq'::regclass)
 username   | text                     |           | not null |
 price      | bigint                   |           | not null |
 volume     | bigint                   |           | not null |
 seq        | bigint                   |           | not null |
 created_at | timestamp with time zone |           | not null | now()
Indexes:
    "bids_pkey" PRIMARY KEY, btree (id)
    "idx_bids_price_seq" btree (price DESC, seq) WHERE volume > 0
```



---

### 7. Clean up

```sql
DROP TABLE bids, allocations, supply_state;
```

```bash
\q
```

or wipe everything:

```bash
docker compose down -v
docker compose up -d postgres
```

---

### 8. Then automate

Applies any pending SQL migrations from migrations/ folder to the database in DATABASE_URL.

```bash
export DATABASE_URL="postgres://twn:twn@localhost:5432/twn"
sqlx migrate run
```


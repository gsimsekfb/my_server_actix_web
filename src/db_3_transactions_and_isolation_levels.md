

## Transactions:

A transaction is a group of one or more database operations that execute as a single unit — either **all** of them succeed and get saved - "commit", or if anything fails, **none** of them take effect - "rollback", leaving the database as if nothing happened.

Example — a bank transfer needs two updates to happen together:

```sql
BEGIN;
UPDATE accounts SET balance = balance - 100 WHERE id = 1;  -- take from Alice
UPDATE accounts SET balance = balance + 100 WHERE id = 2;  -- give to Bob
COMMIT;
```

If the second `UPDATE` failed (e.g. Bob's account doesn't exist), you don't want Alice's money to just vanish — wrapping both in a transaction guarantees either both updates happen or neither does. This is the "Atomicity" in *ACID.

*ACID is the set of four guarantees a transaction gets in a strictly consistent database:

- **Atomicity** — the transaction either fully completes or fully fails, never partially (e.g. money leaves one account AND arrives in another, or neither happens)
- **Consistency** — a transaction only moves the DB from one valid state to another valid state (constraints, foreign keys, etc. are never violated)
- **Isolation** — concurrent transactions don't see each other's half-finished work
- **Durability** — once committed, the write survives even a crash right after (it's on disk, not just in memory)


### More on Transactions:

The two required "start" and "end" pairs for a transaction:

**Start:** `BEGIN` (or `START TRANSACTION` — same thing, different syntax depending on the DB)

**End (pick one):**
- `COMMIT` — save everything
- `ROLLBACK` — undo everything

Every transaction must end with exactly one of those two — you can't just leave it hanging (though if a connection drops mid-transaction, the DB auto-rolls-back).

```sql
BEGIN;
  ...operations...
COMMIT;      -- ends successfully
```

-- or

```sql
BEGIN;
  ...operations...
ROLLBACK;    -- ends by undoing
```

These SQL **transaction control statements**, a distinct category of SQL commands (separate from `SELECT`/`INSERT`/`UPDATE` which are data commands). 


-------------


## Transactions & isolation levels 

Isolation levels control what a transaction is allowed to see while other transactions are running concurrently, trading correctness for speed.

**The four standard levels (weakest → strongest):**

| Level | Problem it prevents | Still allows | Typical production use |
|:---|:---|:---|:---|
| Read Uncommitted | — | Dirty reads (see uncommitted data) | Rarely used; extreme cases like rough analytics where any speed gain matters more than accuracy |
| Read Committed | Dirty reads | Non-repeatable reads (same row changes between two reads) | **Default in Postgres** — most web APIs, catalogs, feeds, dashboards |
| Repeatable Read | Non-repeatable reads | Phantom reads (new rows appear between two reads) | **Default in MySQL**. Reports/batch jobs needing a consistent snapshot; MySQL's default |
| Serializable | Phantom reads | — (fully isolated, but slowest) | Financial transfers, inventory/stock systems, anything where races could double-spend or oversell |

**Example — showing the "dirty read" problem ("Read Uncommitted"  level):**

```sql
-- Transaction A                      -- Transaction B
BEGIN;
UPDATE accounts SET balance = balance - 100
  WHERE id = 1;
-- not committed yet...

                                       BEGIN;
                                       SELECT balance FROM accounts WHERE id = 1;
                                       -- (Read Uncommitted) sees balance-100, even
                                       -- though A hasn't committed!
                                       COMMIT;  -- or just done reading

ROLLBACK;
-- A undoes its change — the -100 never really happened
```


With **Read Committed** or stricter, B would only see the balance *after* A commits (or its old value if A hasn't committed) — never a value from an uncommitted, possibly-rolled-back transaction.


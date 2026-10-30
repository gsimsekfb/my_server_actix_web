### 1. Add Postgres to `docker-compose.yml`

```yaml
services:
  postgres:
    image: postgres:17
    environment:
      POSTGRES_USER: twn
      POSTGRES_PASSWORD: twn
      POSTGRES_DB: twn
    ports:
      - "5432:5432"
    volumes:
      - postgres_data:/var/lib/postgresql/data

volumes:
  postgres_data:
```

```bash
docker compose up -d postgres
```

---

### 2. `Cargo.toml`

```toml
[features]
postgres = ["dep:sqlx"]

[dependencies]
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "macros", "migrate"], optional = true }
```

---

### 3. Migrations

```bash
cargo install sqlx-cli --no-default-features --features rustls,postgres
mkdir migrations
sqlx migrate add init_schema
```

`migrations/<timestamp>_init_schema.sql`
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

```bash
export DATABASE_URL="postgres://twn:twn@localhost:5432/twn"
sqlx migrate run
```

---

### 4. `src/tw_storage.rs` — the trait

```rust
use async_trait::async_trait;
use crate::tw_main::{BuyRequest, SellRequest};

#[async_trait]
pub trait Storage: Send + Sync {
    async fn buy(&self, req: BuyRequest);
    async fn sell(&self, req: SellRequest);
    async fn allocation(&self, username: &str) -> Option<u64>;
}
```

```toml
# Cargo.toml, add:
async-trait = "0.1"
```

---

### 5. `src/tw_storage_memory.rs` — wraps existing in-memory logic

```rust
use async_trait::async_trait;
use std::sync::RwLock;
use std::collections::BTreeMap;

use crate::tw_main::{
    AppState as MemState, BuyRequest, SellRequest,
    buy_impl, sell_impl, ordered_locks_buy, ordered_locks_sell,
};
use crate::tw_storage::Storage;

pub struct InMemoryStorage {
    inner: MemState,
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self { inner: MemState::default() }
    }
}

#[async_trait]
impl Storage for InMemoryStorage {
    async fn buy(&self, req: BuyRequest) {
        let mut bids = ordered_locks_buy(&self.inner);
        buy_impl(
            &self.inner.buy_seq_no,
            &self.inner.supply,
            &self.inner.allocations,
            &mut bids,
            req,
        );
    }

    async fn sell(&self, req: SellRequest) {
        let mut bids = ordered_locks_sell(&self.inner);
        sell_impl(&self.inner.supply, &self.inner.allocations, &mut bids, req);
    }

    async fn allocation(&self, username: &str) -> Option<u64> {
        self.inner.allocations.get(username).map(|v| *v)
    }
}
```

---

### 6. `src/tw_storage_postgres.rs` — Postgres impl

```rust
use async_trait::async_trait;
use sqlx::{PgPool, Row};
use crate::tw_main::{BuyRequest, SellRequest};
use crate::tw_storage::Storage;

pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    pub async fn connect(database_url: &str) -> Self {
        let pool = PgPool::connect(database_url)
            .await
            .expect("failed to connect to postgres");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("failed to run migrations");
        Self { pool }
    }
}

#[async_trait]
impl Storage for PostgresStorage {
    async fn buy(&self, req: BuyRequest) {
        let mut tx = self.pool.begin().await.unwrap();

        // lock supply row, bump seq
        let row = sqlx::query("SELECT supply, buy_seq FROM supply_state WHERE id = 1 FOR UPDATE")
            .fetch_one(&mut *tx).await.unwrap();
        let supply: i64 = row.get("supply");
        let seq: i64 = row.get::<i64, _>("buy_seq") + 1;

        let volume = req.volume as i64;
        let price = req.price as i64;

        if supply <= 0 {
            // no supply: store as open bid
            sqlx::query("INSERT INTO bids (username, price, volume, seq) VALUES ($1,$2,$3,$4)")
                .bind(&req.user).bind(price).bind(volume).bind(seq)
                .execute(&mut *tx).await.unwrap();
        } else if supply >= volume {
            // full fill
            sqlx::query("UPDATE supply_state SET supply = supply - $1 WHERE id = 1")
                .bind(volume).execute(&mut *tx).await.unwrap();
            upsert_allocation(&mut tx, &req.user, volume).await;
        } else {
            // partial fill
            let remainder = volume - supply;
            sqlx::query("UPDATE supply_state SET supply = 0 WHERE id = 1")
                .execute(&mut *tx).await.unwrap();
            upsert_allocation(&mut tx, &req.user, supply).await;
            sqlx::query("INSERT INTO bids (username, price, volume, seq) VALUES ($1,$2,$3,$4)")
                .bind(&req.user).bind(price).bind(remainder).bind(seq)
                .execute(&mut *tx).await.unwrap();
        }

        sqlx::query("UPDATE supply_state SET buy_seq = $1 WHERE id = 1")
            .bind(seq).execute(&mut *tx).await.unwrap();

        tx.commit().await.unwrap();
    }

    async fn sell(&self, req: SellRequest) {
        let mut tx = self.pool.begin().await.unwrap();

        sqlx::query("UPDATE supply_state SET supply = supply + $1 WHERE id = 1")
            .bind(req.volume as i64).execute(&mut *tx).await.unwrap();

        // pull bids highest-price-first, FIFO within price level
        let bids = sqlx::query(
            "SELECT id, username, volume FROM bids WHERE volume > 0
             ORDER BY price DESC, seq ASC FOR UPDATE"
        ).fetch_all(&mut *tx).await.unwrap();

        for bid in bids {
            let row = sqlx::query("SELECT supply FROM supply_state WHERE id = 1 FOR UPDATE")
                .fetch_one(&mut *tx).await.unwrap();
            let supply: i64 = row.get("supply");
            if supply == 0 { break; }

            let bid_id: i64 = bid.get("id");
            let username: String = bid.get("username");
            let bid_volume: i64 = bid.get("volume");

            if supply >= bid_volume {
                sqlx::query("UPDATE supply_state SET supply = supply - $1 WHERE id = 1")
                    .bind(bid_volume).execute(&mut *tx).await.unwrap();
                sqlx::query("UPDATE bids SET volume = 0 WHERE id = $1")
                    .bind(bid_id).execute(&mut *tx).await.unwrap();
                upsert_allocation(&mut tx, &username, bid_volume).await;
            } else {
                sqlx::query("UPDATE supply_state SET supply = 0 WHERE id = 1")
                    .execute(&mut *tx).await.unwrap();
                sqlx::query("UPDATE bids SET volume = volume - $1 WHERE id = $2")
                    .bind(supply).bind(bid_id).execute(&mut *tx).await.unwrap();
                upsert_allocation(&mut tx, &username, supply).await;
            }
        }

        tx.commit().await.unwrap();
    }

    async fn allocation(&self, username: &str) -> Option<u64> {
        sqlx::query("SELECT volume FROM allocations WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .unwrap()
            .map(|row| row.get::<i64, _>("volume") as u64)
    }
}

async fn upsert_allocation(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, username: &str, add: i64) {
    sqlx::query(
        "INSERT INTO allocations (username, volume) VALUES ($1, $2)
         ON CONFLICT (username) DO UPDATE SET volume = allocations.volume + $2"
    ).bind(username).bind(add).execute(&mut **tx).await.unwrap();
}
```

---

### 7. Wire it into `AppState` behind the feature flag

```rust
// src/tw_main.rs (or a new tw_app_state.rs)

#[cfg(feature = "postgres")]
use crate::tw_storage_postgres::PostgresStorage as ActiveStorage;
#[cfg(not(feature = "postgres"))]
use crate::tw_storage_memory::InMemoryStorage as ActiveStorage;

pub struct AppState {
    pub storage: ActiveStorage,
}
```

---

### 8. Init in `src/tw_main_fn.rs`

```rust
#[cfg(feature = "postgres")]
let storage = ActiveStorage::connect(
    &std::env::var("DATABASE_URL").expect("DATABASE_URL not set")
).await;

#[cfg(not(feature = "postgres"))]
let storage = ActiveStorage::default();

let app_state = web::Data::new(AppState { storage });
```

Handlers then call `state.storage.buy(req.0).await;` instead of touching `buy_impl` directly.

---

### 9. Run

```bash
# in-memory (default)
cargo r --bin twn

# postgres-backed
export DATABASE_URL="postgres://twn:twn@localhost:5432/twn"
docker compose up -d postgres
cargo r --bin twn --features postgres
```
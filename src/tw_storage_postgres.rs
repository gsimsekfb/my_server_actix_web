use async_trait::async_trait;
use sqlx::{PgPool, Row};
use crate::tw_main::{BuyRequest, SellRequest};
use crate::tw_matching_engine_trait::TwnEngine;

#[derive(Debug)]
pub struct PostgresStorage {
    pool: PgPool,
}

impl PostgresStorage {
    pub async fn connect(database_url: &str) -> Self {
        let pool = PgPool::connect(database_url).await
            .expect("failed to connect to postgres");

        // Optional, can be done w/o code
        // sqlx::migrate!("./migrations").run(&pool).await
        //     .expect("failed to run migrations");

        Self { pool }
    }
}

#[async_trait]
impl TwnEngine for PostgresStorage {

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

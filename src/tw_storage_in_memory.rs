use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use std::collections::BTreeMap;
use std::sync::atomic::AtomicU64;

use super::tw_err_circuit_breaker::{PriceFeed};
use crate::tw_main::{
    ActiveStorage, AppState, Bid, BuyRequest, SellRequest,
    buy_impl, sell_impl,
    PriceSeqPair
};
use crate::tw_matching_engine_trait::TwnEngine;

#[derive(Default, Debug)]
pub struct InMemoryStorage {
    pub buy_seq_no: AtomicU64, // buy sequence number
    pub supply: AtomicU64,                 // unallocated 
        // this could be Atomic, using Mutex to show lock order / deadlock topic
    pub allocations: DashMap<String, u64>,  // allocated 
        // DashMap: lock-free concurrent HashMap which uses Mutex sharding
    // Highest price top element. For same price bids, smaller seq on top
    // (Reverse(price), seq)
    pub bids: RwLock<BTreeMap<PriceSeqPair, Bid>>, // RwLock for educ. purposes
        // Mutex when frequently modified/written
        // RwLock when frequently read (reads 10 times more than writes)
        //  Vec<Bid>
            // buy : O(n log n) - sort bids vec
            // sell: O(n) - iterate bids vec
        // BTreeMap<PriceSeqPair, Bid>
            // buy : O(log n) - insert
            // sell: O(n) - retain will visit every elem
    pub price_feed: Arc<PriceFeed>
}

#[async_trait]
impl TwnEngine for InMemoryStorage {

    async fn buy(&self, req: BuyRequest) {
        let mut bids = ordered_locks_buy(&self);
        // !! enforcing lock order w/ fn, instead of writing:
            // let mut supply = state.supply.lock().unwrap();
            // let mut bids = state.bids.write().unwrap();

        // todo: move this fn to this file and/or combine it w/ this.buy fn 
        buy_impl(
            &self.buy_seq_no,
            &self.supply,
            &self.allocations,
            &mut bids,
            req,
        );
    }

    async fn sell(&self, req: SellRequest) {
        let mut bids = ordered_locks_sell(&self);
        // !! enforcing lock order w/ fn, instead of writing:
            // let mut supply = state.supply.lock().unwrap();
            // let mut bids = state.bids.write().unwrap();

        // todo: move this fn to this file and/or combine it w/ this.sell fn 
        sell_impl(&self.supply, &self.allocations, &mut bids, req);
    }

    async fn allocation(&self, username: &str) -> Option<u64> {
        self.allocations.get(username).as_deref()
            .copied()  // Option<&u64> to Option<64>
    }

}

//// ----- Ordered Locks
////
//// To avoid deadlock, the lock order must be the same in these fns 
//// 
//// Also see: pre-commit hook ai check lock order for deadlock:
//// src\tw_ai_pre_commit_hook.txt

pub fn ordered_locks_buy(state: &InMemoryStorage) -> 
    RwLockWriteGuard<'_, BTreeMap<PriceSeqPair, Bid>>
{
    // let supply = state.supply.lock().unwrap();
    state.bids.write().unwrap()
}

pub fn ordered_locks_sell(state: &InMemoryStorage) -> 
    RwLockWriteGuard<'_, BTreeMap<PriceSeqPair, Bid>>
{
    state.bids.write().unwrap()
}

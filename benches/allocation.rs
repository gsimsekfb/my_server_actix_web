use criterion::{
    criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion,
};
use actix_hello::tw_main::*;

//// bench cmds
// cargo bench --bench allocation -- buy_impl
// cargo bench --bench allocation -- sell_impl

// Helper fn
pub fn buy_impl_for_test(state: &AppState, buy_req: BuyRequest) {
    let (mut supply, mut bids) = ordered_locks_buy(state);
    buy_impl(
        &state.buy_seq_no,
        &mut supply,
        &state.allocations,
        &mut bids,
        buy_req
    );
}

// Helper fn
pub fn sell_impl_for_test(state: &AppState, sell_req: SellRequest) {
    let (mut supply, mut bids) = ordered_locks_sell(state);
    sell_impl(
        &mut supply,
        &state.allocations,
        &mut bids,
        sell_req
    );
}

/// Measure the cost of a single buy_impl call on a BTreeMap
/// pre-populated with 1K, 10K, and 100K bids. This isolates the BTreeMap 
/// insert + rebalance cost at different map sizes (10^3, 10^4, 10^5).
/// 
fn bench_buy(c: &mut Criterion) {
    let mut group = c.benchmark_group("buy_impl");

    for size in [1_000u64, 10_000, 100_000] {
        group.bench_with_input(
            BenchmarkId::new("insert", size), &size, |b, &size| {
                b.iter_batched(
                    // Setup: create state and pre-populate with `size` bids,
                    // setup (populating the map) runs outside the measurement
                    || {
                        let state = AppState::default();
                        for i in 0..size {
                            buy_impl_for_test(
                                &state,
                                BuyRequest::new(format!("u{i}"), 10, i % 100),
                            );
                        }
                        state
                    },
                    // Routine: measure one more buy_impl call
                    |state| {
                        buy_impl_for_test(
                            &state,
                            BuyRequest::new("new_user", 10, 50),
                        );
                    },
                    BatchSize::PerIteration,
                );
        });
    }

    group.finish();
}

/// Measure the cost of sell_impl's retain() call on a BTreeMap with size bids.
/// So: O(n) retain traversal + n DashMap inserts at 1K/10K/100K map sizes, 
/// with the population cost excluded from timing.
/// e.g. For size = 1_000:
/// Setup (not timed): insert 1,000 bids into the BTreeMap, each with volume 10
/// Measured: call sell_impl with volume 1_000 × 10 = 10,000 — drains all 1,000 
/// bids via retain.
/// 
fn bench_sell(c: &mut Criterion) {
    let mut group = c.benchmark_group("sell_impl");

    for size in [1_000u64, 10_000, 100_000] {
        group // .sample_size(50)
            .measurement_time(std::time::Duration::from_secs(15))
            .bench_with_input(
            BenchmarkId::new("retain", size), &size, |b, &size| {
                b.iter_batched(
                    // Setup: populate bids — excluded from measurement
                    || {
                        let state = AppState::default();
                        for i in 0..size {
                            buy_impl_for_test(
                                &state,
                                BuyRequest::new(format!("u{i}"), 10, i % 100),
                            );
                        }
                        state
                    },
                    // Routine: measure only sell_impl
                    |state| {
                        sell_impl_for_test(
                            &state,
                            SellRequest { volume: size * 10 },
                        );
                    },
                    BatchSize::PerIteration,
                );
        });
    }

    group.finish();
}

criterion_group!(benches, bench_buy, bench_sell);
criterion_main!(benches);
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use actix_hello::tw_main::*;

//// Todos
// further isolate buy_impl 


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


fn bench_sell(c: &mut Criterion) {
    let mut group = c.benchmark_group("sell_impl");

    for size in [1_000u64, 10_000, 100_000] {
        group // .sample_size(50)
            .measurement_time(std::time::Duration::from_secs(15))
            .bench_with_input(
            BenchmarkId::new("retain", size), &size, |b, &size| {
                b.iter(|| {
                    let state = AppState::default();
                    // populate bids
                    for i in 0..size {
                        buy_impl_for_test(
                            &state,
                            BuyRequest::new(format!("u{i}"), 10, i % 100)
                        );
                    }
                    // measure sell allocation
                    sell_impl_for_test(
                        &state,
                        SellRequest { volume: size * 10 }
                    );
                });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_sell);
criterion_main!(benches);
use criterion::{criterion_group, criterion_main, Criterion};
use lobster::{OrderBook, OrderType, Side};

fn small_limit_ladder(c: &mut Criterion) {
    c.bench_function("small limit ladder", |b| {
        let mut ob = OrderBook::default();
        b.iter(|| {
            for i in 0..5_000 {
                ob.event(OrderType::Limit {
                    id: i as u128,
                    price: 12345 + i as u64,
                    qty: i as u64,
                    side: Side::Bid,
                });
            }
        });
    });
}

fn big_limit_ladder(c: &mut Criterion) {
    c.bench_function("big limit ladder", |b| {
        let mut ob = OrderBook::default();
        b.iter(|| {
            for i in 0..100_000 {
                ob.event(OrderType::Limit {
                    id: i as u128,
                    price: 12345 + i as u64,
                    qty: i as u64,
                    side: Side::Bid,
                });
            }
        });
    });
}

criterion_group!(benches, small_limit_ladder, big_limit_ladder);
criterion_main!(benches);

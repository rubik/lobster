use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lobster::{OrderBook, OrderType, Side};
use std::fs::File;

type Record = (u128, String, u64, u64);

fn all_orders(c: &mut Criterion) {
    c.bench_function("all orders", |b| {
        let mut orders: Vec<OrderType> = Vec::new();
        let mut ord_id = 0;
        let mut ob = OrderBook::default();
        load_orders("data/orders.csv", &mut orders, &mut ord_id);

        b.iter(|| {
            for ord in &orders {
                ob.execute(*ord);
            }
        });
    });
}

fn all_orders_with_stats(c: &mut Criterion) {
    c.bench_function("all orders with stats tracking", |b| {
        let mut orders: Vec<OrderType> = Vec::new();
        let mut ord_id = 0;
        let mut ob = OrderBook::default();
        ob.track_stats(true);
        load_orders("data/orders.csv", &mut orders, &mut ord_id);

        b.iter(|| {
            for ord in &orders {
                ob.execute(*ord);
            }
        });
    });
}

fn all_orders_with_stats_and_queries(c: &mut Criterion) {
    c.bench_function("all orders with stats tracking and queries", |b| {
        let mut orders: Vec<OrderType> = Vec::new();
        let mut ord_id = 0;
        let mut ob = OrderBook::default();
        ob.track_stats(true);
        load_orders("data/orders.csv", &mut orders, &mut ord_id);

        b.iter(|| {
            for ord in &orders {
                ob.execute(*ord);
                let stats = |ob: &OrderBook| {
                    ob.last_trade();
                    ob.traded_volume();
                };
                stats(black_box(&ob));
            }
        });
    });
}

fn load_orders(path: &str, orders: &mut Vec<OrderType>, mut ord_id: &mut u128) {
    let file = File::open(path).unwrap();
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(file);
    for result in rdr.deserialize() {
        let record = result.unwrap();
        orders.push(convert_to_order(&mut ord_id, record));
    }
}

fn convert_to_order(id: &mut u128, record: Record) -> OrderType {
    if record.2 == 0 {
        OrderType::Cancel(record.3 as u128)
    } else {
        *id += 1;
        OrderType::Limit {
            id: *id,
            side: match record.1.as_str() {
                "Bid" => Side::Bid,
                "Ask" => Side::Ask,
                _ => panic!("the side can only be 'Bid' or 'Ask'"),
            },
            qty: record.3,
            price: record.2,
        }
    }
}

criterion_group!(
    benches,
    all_orders,
    all_orders_with_stats,
    all_orders_with_stats_and_queries
);
criterion_main!(benches);

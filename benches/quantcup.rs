use criterion::{criterion_group, criterion_main, Criterion};
use lobster::{OrderBook, OrderType, Side};
use std::fs::File;

type Record = (u128, String, u64, u64);

fn all_orders(c: &mut Criterion) {
    c.bench_function("all orders", |b| {
        let file = File::open("data/orders.csv").unwrap();
        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(file);
        let mut orders: Vec<OrderType> = Vec::new();
        let mut ord_id = 0;
        for result in rdr.deserialize() {
            let record = result.unwrap();
            orders.push(convert_to_order(&mut ord_id, record));
        }
        let mut ob = OrderBook::default();

        b.iter(|| {
            for ord in &orders {
                ob.execute(*ord);
            }
        });
    });
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

criterion_group!(benches, all_orders);
criterion_main!(benches);

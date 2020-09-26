use lobster::{OrderBook, OrderType, Side};
use std::fs::File;
use std::time::Instant;

type Record = (u128, String, u64, u64);

fn main() {
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

    let total_orders = orders.len();

    let batch_size: usize = 5000;
    let replay_count: usize = 200;

    let mut latencies: Vec<u64> =
        Vec::with_capacity(replay_count * (total_orders / batch_size));

    let mut total_time = 0;
    for _ in 0..replay_count {
        let mut ob: OrderBook = OrderBook::default();

        let mut i = batch_size;
        while i < total_orders {
            let begin = Instant::now();

            for ord in orders.iter().take(i).skip(i - batch_size) {
                let _new_fills = ob.event(*ord);
            }
            let elapsed = begin.elapsed();
            let nanos = elapsed.as_secs() * 1_000_000_000
                + u64::from(elapsed.subsec_nanos());
            latencies.push(nanos);
            total_time += nanos;
            i += batch_size;
        }
    }

    let mean: f64 = latencies.as_slice().mean();
    let std_dev = latencies.as_slice().std_dev();

    println!();
    println!("{: <15} = {:>12} ns", "Total time", total_time);
    println!("{: <15} = {:>12.0} ns", "Mean per batch", mean);
    println!("{: <15} = {:>12.0} ns", "SD", std_dev);
    println!("{: <15} = {:>12.0}\n", "Score", 0.5 * (mean + std_dev));
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

// Code below is directly copied from https://github.com/rust-lang/rust/blob/master/src/libtest/stats.rs
trait Stats {
    fn mean(&self) -> f64;
    fn var(&self) -> f64;
    fn std_dev(&self) -> f64;
}

impl Stats for [u64] {
    fn mean(&self) -> f64 {
        assert!(!self.is_empty());
        self.iter().sum::<u64>() as f64 / (self.len() as f64)
    }
    fn var(&self) -> f64 {
        if self.len() < 2 {
            0.0
        } else {
            let mean = self.mean();
            let mut v: f64 = 0.0;
            for s in self {
                let x = *s as f64 - mean;
                v += x * x;
            }
            // NB: this is _supposed to be_ len-1, not len. If you
            // change it back to len, you will be calculating a
            // population variance, not a sample variance.
            let denom = (self.len() - 1) as f64;
            v / denom
        }
    }

    fn std_dev(&self) -> f64 {
        self.var().sqrt()
    }
}

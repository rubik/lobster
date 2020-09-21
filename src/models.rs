#[derive(Debug, PartialEq)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug)]
pub enum OrderEvent {
    Market {
        id: u128,
        side: Side,
        qty: u64,
    },
    Limit {
        id: u128,
        side: Side,
        qty: u64,
        price: u64,
    },
    Cancel(u128),
}

#[derive(Debug)]
pub enum OrderEventResult {
    Unfilled,
    PartiallyFilled(Vec<FillMetadata>),
    Filled(Vec<FillMetadata>),
    Placed(u128),
    Canceled(u128),
}

#[derive(Debug)]
pub struct FillMetadata {
    pub order_1: u128,
    pub order_2: u128,
    pub qty: u64,
    pub price: u64,
}

#[derive(Debug)]
pub struct LimitOrder {
    pub id: u128,
    pub qty: u64,
    pub price: u64,
}

impl LimitOrder {
    pub fn new(id: u128, qty: u64, price: u64) -> Self {
        Self { id, qty, price }
    }
}

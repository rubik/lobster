#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug)]
pub enum OrderType {
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

#[derive(Debug, PartialEq)]
pub enum OrderEvent {
    Unfilled,
    PartiallyFilled {
        id: u128,
        filled_qty: u64,
        fills: Vec<FillMetadata>,
    },
    Filled {
        id: u128,
        filled_qty: u64,
        fills: Vec<FillMetadata>,
    },
    Placed(u128),
    Canceled(u128),
}

#[derive(Debug, PartialEq)]
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

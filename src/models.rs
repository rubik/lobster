/// An order book side.
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Side {
    /// The bid (or buy) side.
    Bid,
    /// The ask (or sell) side.
    Ask,
}

/// An order to be executed by the order book.
#[derive(Debug, Copy, Clone)]
pub enum OrderType {
    /// A market order, which is either filled immediately (even partially), or
    /// canceled.
    Market {
        /// The unique ID of this order.
        id: u128,
        /// The order side. It will be matched against the resting orders on the
        /// other side of the order book.
        side: Side,
        /// The order quantity.
        qty: u64,
    },
    /// A limit order, which is either filled immediately, or added to the order
    /// book.
    Limit {
        /// The unique ID of this order.
        id: u128,
        /// The order side. It will be matched against the resting orders on the
        /// other side of the order book.
        side: Side,
        /// The order quantity.
        qty: u64,
        /// The limit price. The order book will only match this order with
        /// other orders at this price or better.
        price: u64,
    },
    /// A cancel order, which removes the order with the specified ID from the
    /// order book.
    Cancel(u128),
}

/// An event resulting from the execution of an order.
#[derive(Debug, PartialEq, Clone)]
pub enum OrderEvent {
    /// Indicating that the corresponding order was not filled. It is only sent
    /// in response to market orders.
    Unfilled(u128),
    /// Indicating that the corresponding order was placed on the order book. It
    /// is only send in response to limit orders.
    Placed(u128),
    /// Indicating that the corresponding order was removed from the order book.
    /// It is only sent in response to cancel orders.
    Canceled(u128),
    /// Indicating that the corresponding order was only partially filled. It is
    /// sent in response to market or limit orders.
    PartiallyFilled {
        /// The ID of the order this event is referring to.
        id: u128,
        /// The filled quantity.
        filled_qty: u64,
        /// A vector with information on the order fills.
        fills: Vec<FillMetadata>,
    },
    /// Indicating that the corresponding order was filled completely. It is
    /// sent in response to market or limit orders.
    Filled {
        /// The ID of the order this event is referring to.
        id: u128,
        /// The filled quantity.
        filled_qty: u64,
        /// A vector with information on the order fills.
        fills: Vec<FillMetadata>,
    },
}

/// Information on a single order fill. When an order is matched with multiple
/// resting orders, it generates multiple `FillMetadata` values.
#[derive(Debug, PartialEq, Copy, Clone)]
pub struct FillMetadata {
    /// The ID of the order that triggered the fill.
    pub order_1: u128,
    /// The ID of the matching order.
    pub order_2: u128,
    /// The quantity that was traded.
    pub qty: u64,
    /// The price at which the trade happened.
    pub price: u64,
}

/// A snapshot of the order book up to a certain depth level. Multiple orders at
/// the same price points are merged into a single [`BookLevel`] struct.
///
/// [`BookLevel`]: /struct.BookLevel.html
#[derive(Debug, Clone, PartialEq)]
pub struct BookDepth {
    /// The requested level. This field will always contain the level that was
    /// requested, even if some or all levels are empty.
    pub levels: usize,
    /// A vector of price points with the associated quantity on the ask side.
    pub asks: Vec<BookLevel>,
    /// A vector of price points with the associated quantity on the bid side.
    pub bids: Vec<BookLevel>,
}

/// A single level in the order book. This struct is used both for the bid and
/// ask side.
#[derive(Debug, Clone, PartialEq)]
pub struct BookLevel {
    /// The price point this level represents.
    pub price: u64,
    /// The total quantity of all orders resting at the specified price point.
    pub qty: u64,
}

/// A trade that happened as part of the matching process.
#[derive(Debug, Clone)]
pub struct Trade {
    /// The total quantity transacted as part of this trade.
    pub qty: u64,
    /// The volume-weighted average price computed from all the order fills
    /// within this trade.
    pub avg_price: f64,
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

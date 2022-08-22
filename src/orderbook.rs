use std::collections::BTreeMap;

use crate::arena::OrderArena;
use crate::models::{
    BookDepth, BookLevel, FillMetadata, OrderEvent, OrderType, Side, Trade,
};

const DEFAULT_ARENA_CAPACITY: usize = 10_000;
const DEFAULT_QUEUE_CAPACITY: usize = 10;

/// An order book that executes orders serially through the [`execute`] method.
///
/// [`execute`]: #method.execute
#[derive(Debug)]
pub struct OrderBook {
    last_trade: Option<Trade>,
    traded_volume: u64,
    min_ask: Option<u64>,
    max_bid: Option<u64>,
    asks: BTreeMap<u64, Vec<usize>>,
    bids: BTreeMap<u64, Vec<usize>>,
    arena: OrderArena,
    default_queue_capacity: usize,
    track_stats: bool,
}

impl Default for OrderBook {
    /// Create an instance representing a single order book, with stats tracking
    /// disabled, a default arena capacity of 10,000 and a default queue
    /// capacity of 10.
    fn default() -> Self {
        Self::new(DEFAULT_ARENA_CAPACITY, DEFAULT_QUEUE_CAPACITY, false)
    }
}

impl OrderBook {
    /// Create an instance representing a single order book.
    ///
    /// The `arena_capacity` parameter represents the number of orders that will
    /// be pre-allocated.
    ///
    /// The `queue_capacity` parameter represents the capacity of each vector
    /// storing orders at the same price point.
    ///
    /// The `track_stats` parameter indicates whether to enable volume and
    /// trades tracking (see [`last_trade`] and [`traded_volume`]).
    ///
    /// [`last_trade`]: #method.last_trade
    /// [`traded_volume`]: #method.traded_volume
    pub fn new(
        arena_capacity: usize,
        queue_capacity: usize,
        track_stats: bool,
    ) -> Self {
        Self {
            last_trade: None,
            traded_volume: 0,
            min_ask: None,
            max_bid: None,
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            arena: OrderArena::new(arena_capacity),
            default_queue_capacity: queue_capacity,
            track_stats,
        }
    }

    #[cfg(test)]
    #[doc(hidden)]
    pub fn _asks(&self) -> BTreeMap<u64, Vec<usize>> {
        self.asks.clone()
    }

    #[cfg(test)]
    #[doc(hidden)]
    pub fn _bids(&self) -> BTreeMap<u64, Vec<usize>> {
        self.bids.clone()
    }

    /// Return the lowest ask price, if present.
    #[inline(always)]
    pub fn min_ask(&self) -> Option<u64> {
        self.min_ask
    }

    /// Return the highest bid price, if present.
    #[inline(always)]
    pub fn max_bid(&self) -> Option<u64> {
        self.max_bid
    }

    /// Return the difference of the lowest ask and highest bid, if both are
    /// present.
    #[inline(always)]
    pub fn spread(&self) -> Option<u64> {
        match (self.max_bid, self.min_ask) {
            (Some(b), Some(a)) => Some(a - b),
            _ => None,
        }
    }

    /// Return the last trade recorded while stats tracking was active as a
    /// [`Trade`] object, if present.
    ///
    /// [`Trade`]: struct.Trade.html
    #[inline(always)]
    pub fn last_trade(&self) -> Option<Trade> {
        self.last_trade
    }

    /// Return the total traded volume for all the trades that occurred while
    /// the stats tracking was active.
    #[inline(always)]
    pub fn traded_volume(&self) -> u64 {
        self.traded_volume
    }

    /// Return the order book depth as a [`BookDepth`] struct, up to the
    /// specified level. Bids and offers at the same price level are merged in a
    /// single [`BookLevel`] struct.
    ///
    /// [`BookDepth`]: struct.BookDepth.html
    /// [`BookLevel`]: struct.BookLevel.html
    pub fn depth(&self, levels: usize) -> BookDepth {
        let mut asks: Vec<BookLevel> = Vec::with_capacity(levels);
        let mut bids: Vec<BookLevel> = Vec::with_capacity(levels);

        for (ask_price, queue) in self.asks.iter() {
            let mut qty = 0;
            for idx in queue {
                qty += self.arena[*idx].qty;
            }
            if qty > 0 {
                asks.push(BookLevel {
                    price: *ask_price,
                    qty,
                });
            }
        }

        for (bid_price, queue) in self.bids.iter() {
            let mut qty = 0;
            for idx in queue {
                qty += self.arena[*idx].qty;
            }
            if qty > 0 {
                bids.push(BookLevel {
                    price: *bid_price,
                    qty,
                });
            }
        }

        BookDepth { levels, asks, bids }
    }

    /// Toggle the stats tracking on or off, depending on the `track` parameter.
    pub fn track_stats(&mut self, track: bool) {
        self.track_stats = track;
    }

    /// Execute an order, returning immediately an event indicating the result.
    pub fn execute(&mut self, event: OrderType) -> OrderEvent {
        let event = self._execute(event);
        if !self.track_stats {
            return event;
        }

        match event.clone() {
            OrderEvent::Filled {
                id: _,
                filled_qty,
                fills,
            } => {
                self.traded_volume += filled_qty;
                // If we are here, fills is not empty, so it's safe to unwrap it
                let last_fill = fills.last().unwrap();
                self.last_trade = Some(Trade {
                    total_qty: filled_qty,
                    avg_price: fills
                        .iter()
                        .map(|fm| fm.price * fm.qty)
                        .sum::<u64>() as f64
                        / (filled_qty as f64),
                    last_qty: last_fill.qty,
                    last_price: last_fill.price,
                });
            }
            OrderEvent::PartiallyFilled {
                id: _,
                filled_qty,
                fills,
            } => {
                self.traded_volume += filled_qty;
                // If we are here, fills is not empty, so it's safe to unwrap it
                let last_fill = fills.last().unwrap();
                self.last_trade = Some(Trade {
                    total_qty: filled_qty,
                    avg_price: fills
                        .iter()
                        .map(|fm| fm.price * fm.qty)
                        .sum::<u64>() as f64
                        / (filled_qty as f64),
                    last_qty: last_fill.qty,
                    last_price: last_fill.price,
                });
            }
            _ => {}
        }
        event
    }

    fn _execute(&mut self, event: OrderType) -> OrderEvent {
        match event {
            OrderType::Market { id, side, qty } => {
                let (fills, partial, filled_qty) = self.market(id, side, qty);
                if fills.is_empty() {
                    OrderEvent::Unfilled { id }
                } else if partial {
                    OrderEvent::PartiallyFilled {
                        id,
                        filled_qty,
                        fills,
                    }
                } else {
                    OrderEvent::Filled {
                        id,
                        filled_qty,
                        fills,
                    }
                }
            }
            OrderType::Limit {
                id,
                side,
                qty,
                price,
            } => {
                let (fills, partial, filled_qty) =
                    self.limit(id, side, qty, price);
                if fills.is_empty() {
                    OrderEvent::Placed { id }
                } else if partial {
                    OrderEvent::PartiallyFilled {
                        id,
                        filled_qty,
                        fills,
                    }
                } else {
                    OrderEvent::Filled {
                        id,
                        filled_qty,
                        fills,
                    }
                }
            }
            OrderType::Cancel { id } => {
                self.cancel(id);
                OrderEvent::Canceled { id }
            }
        }
    }

    fn cancel(&mut self, id: u128) -> bool {
        if let Some((price, idx)) = self.arena.get(id) {
            if let Some(ref mut queue) = self.asks.get_mut(&price) {
                if let Some(i) = queue.iter().position(|i| *i == idx) {
                    queue.remove(i);
                }
                self.update_min_ask();
            }
            if let Some(ref mut queue) = self.bids.get_mut(&price) {
                if let Some(i) = queue.iter().position(|i| *i == idx) {
                    queue.remove(i);
                }
                self.update_max_bid();
            }
        }
        self.arena.delete(&id)
    }

    fn market(
        &mut self,
        id: u128,
        side: Side,
        qty: u64,
    ) -> (Vec<FillMetadata>, bool, u64) {
        let mut fills = Vec::new();

        let remaining_qty = match side {
            Side::Bid => self.match_with_asks(id, qty, &mut fills, None),
            Side::Ask => self.match_with_bids(id, qty, &mut fills, None),
        };

        let partial = remaining_qty > 0;

        (fills, partial, qty - remaining_qty)
    }

    fn limit(
        &mut self,
        id: u128,
        side: Side,
        qty: u64,
        price: u64,
    ) -> (Vec<FillMetadata>, bool, u64) {
        let mut partial = false;
        let remaining_qty;
        let mut fills: Vec<FillMetadata> = Vec::new();

        match side {
            Side::Bid => {
                remaining_qty =
                    self.match_with_asks(id, qty, &mut fills, Some(price));
                if remaining_qty > 0 {
                    partial = true;
                    let index = self.arena.insert(id, price, remaining_qty);
                    let queue_capacity = self.default_queue_capacity;
                    self.bids
                        .entry(price)
                        .or_insert_with(|| Vec::with_capacity(queue_capacity))
                        .push(index);
                    match self.max_bid {
                        None => {
                            self.max_bid = Some(price);
                        }
                        Some(b) if price > b => {
                            self.max_bid = Some(price);
                        }
                        _ => {}
                    };
                }
            }
            Side::Ask => {
                remaining_qty =
                    self.match_with_bids(id, qty, &mut fills, Some(price));
                if remaining_qty > 0 {
                    partial = true;
                    let index = self.arena.insert(id, price, remaining_qty);
                    if let Some(a) = self.min_ask {
                        if price < a {
                            self.min_ask = Some(price);
                        }
                    }
                    let queue_capacity = self.default_queue_capacity;
                    self.asks
                        .entry(price)
                        .or_insert_with(|| Vec::with_capacity(queue_capacity))
                        .push(index);
                    match self.min_ask {
                        None => {
                            self.min_ask = Some(price);
                        }
                        Some(a) if price < a => {
                            self.min_ask = Some(price);
                        }
                        _ => {}
                    };
                }
            }
        }

        (fills, partial, qty - remaining_qty)
    }

    fn match_with_asks(
        &mut self,
        id: u128,
        qty: u64,
        fills: &mut Vec<FillMetadata>,
        limit_price: Option<u64>,
    ) -> u64 {
        let mut remaining_qty = qty;
        let mut update_bid_ask = false;
        for (ask_price, queue) in self.asks.iter_mut() {
            if queue.is_empty() {
                continue;
            }
            if (update_bid_ask || self.min_ask.is_none()) && !queue.is_empty() {
                self.min_ask = Some(*ask_price);
                update_bid_ask = false;
            }
            if let Some(lp) = limit_price {
                if lp < *ask_price {
                    break;
                }
            }
            if remaining_qty == 0 {
                break;
            }
            let filled_qty = Self::process_queue(
                &mut self.arena,
                queue,
                remaining_qty,
                id,
                Side::Bid,
                fills,
            );
            if queue.is_empty() {
                update_bid_ask = true;
            }
            remaining_qty -= filled_qty;
        }

        self.update_min_ask();
        remaining_qty
    }

    fn match_with_bids(
        &mut self,
        id: u128,
        qty: u64,
        fills: &mut Vec<FillMetadata>,
        limit_price: Option<u64>,
    ) -> u64 {
        let mut remaining_qty = qty;
        let mut update_bid_ask = false;
        for (bid_price, queue) in self.bids.iter_mut().rev() {
            if queue.is_empty() {
                continue;
            }
            if (update_bid_ask || self.max_bid.is_none()) && !queue.is_empty() {
                self.max_bid = Some(*bid_price);
                update_bid_ask = false;
            }
            if let Some(lp) = limit_price {
                if lp > *bid_price {
                    break;
                }
            }
            if remaining_qty == 0 {
                break;
            }
            let filled_qty = Self::process_queue(
                &mut self.arena,
                queue,
                remaining_qty,
                id,
                Side::Ask,
                fills,
            );
            if queue.is_empty() {
                update_bid_ask = true;
            }
            remaining_qty -= filled_qty;
        }

        self.update_max_bid();
        remaining_qty
    }

    fn update_min_ask(&mut self) {
        let mut cur_asks = self.asks.iter().filter(|(_, q)| !q.is_empty());
        self.min_ask = cur_asks.next().map(|(p, _)| *p);
    }

    fn update_max_bid(&mut self) {
        let mut cur_bids =
            self.bids.iter().rev().filter(|(_, q)| !q.is_empty());
        self.max_bid = cur_bids.next().map(|(p, _)| *p);
    }

    fn process_queue(
        arena: &mut OrderArena,
        opposite_orders: &mut Vec<usize>,
        remaining_qty: u64,
        id: u128,
        side: Side,
        fills: &mut Vec<FillMetadata>,
    ) -> u64 {
        let mut qty_to_fill = remaining_qty;
        let mut filled_qty = 0;
        let mut filled_index = None;

        for (index, head_order_idx) in opposite_orders.iter_mut().enumerate() {
            if qty_to_fill == 0 {
                break;
            }
            let head_order = &mut arena[*head_order_idx];
            let traded_price = head_order.price;
            let available_qty = head_order.qty;
            if available_qty == 0 {
                filled_index = Some(index);
                continue;
            }
            let traded_quantity: u64;
            let filled;

            if qty_to_fill >= available_qty {
                traded_quantity = available_qty;
                qty_to_fill -= available_qty;
                filled_index = Some(index);
                filled = true;
            } else {
                traded_quantity = qty_to_fill;
                qty_to_fill = 0;
                filled = false;
            }
            head_order.qty -= traded_quantity;
            let fill = FillMetadata {
                order_1: id,
                order_2: head_order.id,
                qty: traded_quantity,
                price: traded_price,
                taker_side: side,
                total_fill: filled,
            };
            fills.push(fill);
            filled_qty += traded_quantity;
        }
        if let Some(index) = filled_index {
            opposite_orders.drain(0..index + 1);
        }

        filled_qty
    }
}

#[cfg(test)]
mod test {
    use crate::{
        BookDepth, BookLevel, FillMetadata, OrderBook, OrderEvent, OrderType,
        Side, Trade,
    };
    use std::collections::BTreeMap;

    const DEFAULT_QUEUE_SIZE: usize = 10;
    const BID_ASK_COMBINATIONS: [(Side, Side); 2] =
        [(Side::Bid, Side::Ask), (Side::Ask, Side::Bid)];

    // In general, floating point values cannot be compared for equality. That's
    // why we don't derive PartialEq in lobster::models, but we do it here for
    // our tests in some very specific cases.
    impl PartialEq for Trade {
        fn eq(&self, other: &Self) -> bool {
            self.total_qty == other.total_qty
                && (self.avg_price - other.avg_price).abs() < 1.0e-6
                && self.last_qty == other.last_qty
                && self.last_price == other.last_price
        }
    }

    fn init_ob(events: Vec<OrderType>) -> (OrderBook, Vec<OrderEvent>) {
        let mut ob = OrderBook::default();
        ob.track_stats(true);
        let mut results = Vec::new();
        for e in events {
            results.push(ob.execute(e));
        }
        (ob, results)
    }

    fn init_book(orders: Vec<(u64, usize)>) -> BTreeMap<u64, Vec<usize>> {
        let mut bk = BTreeMap::new();
        for (p, i) in orders {
            bk.entry(p)
                .or_insert_with(|| Vec::with_capacity(DEFAULT_QUEUE_SIZE))
                .push(i);
        }
        bk
    }

    fn init_book_holes(
        orders: Vec<(u64, usize)>,
        holes: Vec<u64>,
    ) -> BTreeMap<u64, Vec<usize>> {
        let mut bk = init_book(orders);
        for h in holes {
            bk.insert(h, Vec::new());
        }
        bk
    }

    #[test]
    fn empty_book() {
        let (ob, results) = init_ob(Vec::new());
        assert_eq!(results, Vec::new());
        assert_eq!(ob.min_ask(), None);
        assert_eq!(ob.max_bid(), None);
        assert_eq!(ob._asks(), BTreeMap::new());
        assert_eq!(ob._bids(), BTreeMap::new());
        assert_eq!(ob.spread(), None);
        assert_eq!(ob.traded_volume(), 0);
        assert_eq!(
            ob.depth(2),
            BookDepth {
                levels: 2,
                asks: Vec::new(),
                bids: Vec::new()
            }
        );
        assert_eq!(ob.last_trade(), None);
    }

    #[test]
    fn one_resting_order() {
        for (bid_ask, _) in &BID_ASK_COMBINATIONS {
            let (ob, results) = init_ob(vec![OrderType::Limit {
                id: 0,
                side: *bid_ask,
                qty: 12,
                price: 395,
            }]);
            assert_eq!(results, vec![OrderEvent::Placed { id: 0 }]);
            if *bid_ask == Side::Bid {
                assert_eq!(ob.min_ask(), None);
                assert_eq!(ob.max_bid(), Some(395));
                assert_eq!(ob._asks(), BTreeMap::new());
                assert_eq!(ob._bids(), init_book(vec![(395, 9999)]));
                assert_eq!(ob.spread(), None);
                assert_eq!(ob.traded_volume(), 0);
                assert_eq!(
                    ob.depth(3),
                    BookDepth {
                        levels: 3,
                        asks: Vec::new(),
                        bids: vec![BookLevel {
                            price: 395,
                            qty: 12
                        }],
                    }
                );
                assert_eq!(ob.last_trade(), None);
            } else {
                assert_eq!(ob.min_ask(), Some(395));
                assert_eq!(ob.max_bid(), None);
                assert_eq!(ob._asks(), init_book(vec![(395, 9999)]));
                assert_eq!(ob._bids(), BTreeMap::new());
                assert_eq!(ob.spread(), None);
                assert_eq!(ob.traded_volume(), 0);
                assert_eq!(
                    ob.depth(4),
                    BookDepth {
                        levels: 4,
                        asks: vec![BookLevel {
                            price: 395,
                            qty: 12
                        }],
                        bids: Vec::new()
                    }
                );
                assert_eq!(ob.last_trade(), None);
            }
        }
    }

    #[test]
    fn two_resting_orders() {
        for (bid_ask, ask_bid) in &BID_ASK_COMBINATIONS {
            let (ob, results) = init_ob(vec![
                OrderType::Limit {
                    id: 0,
                    side: *bid_ask,
                    qty: 12,
                    price: 395,
                },
                OrderType::Limit {
                    id: 1,
                    side: *ask_bid,
                    qty: 2,
                    price: 398,
                },
            ]);
            if *bid_ask == Side::Bid {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Placed { id: 1 }
                    ]
                );
                assert_eq!(ob.min_ask(), Some(398));
                assert_eq!(ob.max_bid(), Some(395));
                assert_eq!(ob._asks(), init_book(vec![(398, 9998)]));
                assert_eq!(ob._bids(), init_book(vec![(395, 9999)]));
                assert_eq!(ob.spread(), Some(3));
                assert_eq!(ob.traded_volume(), 0);
                assert_eq!(
                    ob.depth(4),
                    BookDepth {
                        levels: 4,
                        asks: vec![BookLevel { price: 398, qty: 2 }],
                        bids: vec![BookLevel {
                            price: 395,
                            qty: 12
                        }],
                    }
                );
                assert_eq!(ob.last_trade(), None);
            } else {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Filled {
                            id: 1,
                            filled_qty: 2,
                            fills: vec![FillMetadata {
                                order_1: 1,
                                order_2: 0,
                                qty: 2,
                                price: 395,
                                taker_side: *ask_bid,
                                total_fill: false,
                            }],
                        }
                    ]
                );
                assert_eq!(ob.min_ask(), Some(395));
                assert_eq!(ob.max_bid(), None);
                assert_eq!(ob._asks(), init_book(vec![(395, 9999)]));
                assert_eq!(ob._bids(), init_book(vec![]));
                assert_eq!(ob.spread(), None);
                assert_eq!(ob.traded_volume(), 2);
                assert_eq!(
                    ob.depth(4),
                    BookDepth {
                        levels: 4,
                        asks: vec![BookLevel {
                            price: 395,
                            qty: 10,
                        }],
                        bids: Vec::new(),
                    }
                );
                assert_eq!(
                    ob.last_trade(),
                    Some(Trade {
                        total_qty: 2,
                        avg_price: 395.0,
                        last_qty: 2,
                        last_price: 395,
                    })
                );
            }
        }
    }

    #[test]
    fn two_resting_orders_merged() {
        for (bid_ask, _) in &BID_ASK_COMBINATIONS {
            let (ob, results) = init_ob(vec![
                OrderType::Limit {
                    id: 0,
                    side: *bid_ask,
                    qty: 12,
                    price: 395,
                },
                OrderType::Limit {
                    id: 1,
                    side: *bid_ask,
                    qty: 2,
                    price: 395,
                },
            ]);
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed { id: 0 },
                    OrderEvent::Placed { id: 1 }
                ]
            );
            if *bid_ask == Side::Bid {
                assert_eq!(ob.min_ask(), None);
                assert_eq!(ob.max_bid(), Some(395));
                assert_eq!(ob._asks(), BTreeMap::new());
                assert_eq!(
                    ob._bids(),
                    init_book(vec![(395, 9999), (395, 9998)])
                );
                assert_eq!(ob.spread(), None);
                assert_eq!(ob.traded_volume(), 0);
                assert_eq!(
                    ob.depth(3),
                    BookDepth {
                        levels: 3,
                        asks: Vec::new(),
                        bids: vec![BookLevel {
                            price: 395,
                            qty: 14
                        }],
                    }
                );
                assert_eq!(ob.last_trade(), None);
            } else {
                assert_eq!(ob.min_ask(), Some(395));
                assert_eq!(ob.max_bid(), None);
                assert_eq!(
                    ob._asks(),
                    init_book(vec![(395, 9999), (395, 9998)])
                );
                assert_eq!(ob._bids(), BTreeMap::new());
                assert_eq!(ob.spread(), None);
                assert_eq!(ob.traded_volume(), 0);
                assert_eq!(
                    ob.depth(3),
                    BookDepth {
                        levels: 3,
                        asks: vec![BookLevel {
                            price: 395,
                            qty: 14
                        }],
                        bids: Vec::new(),
                    }
                );
                assert_eq!(ob.last_trade(), None);
            }
        }
    }

    #[test]
    fn two_resting_orders_stacked() {
        for (bid_ask, _) in &BID_ASK_COMBINATIONS {
            let (ob, results) = init_ob(vec![
                OrderType::Limit {
                    id: 0,
                    side: *bid_ask,
                    qty: 12,
                    price: 395,
                },
                OrderType::Limit {
                    id: 1,
                    side: *bid_ask,
                    qty: 2,
                    price: 398,
                },
            ]);
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed { id: 0 },
                    OrderEvent::Placed { id: 1 }
                ]
            );
            if *bid_ask == Side::Bid {
                assert_eq!(ob.min_ask(), None);
                assert_eq!(ob.max_bid(), Some(398));
                assert_eq!(ob._asks(), BTreeMap::new());
                assert_eq!(
                    ob._bids(),
                    init_book(vec![(398, 9998), (395, 9999)])
                );
                assert_eq!(ob.spread(), None);
            } else {
                assert_eq!(ob.min_ask(), Some(395));
                assert_eq!(ob.max_bid(), None);
                assert_eq!(
                    ob._asks(),
                    init_book(vec![(398, 9998), (395, 9999)])
                );
                assert_eq!(ob._bids(), BTreeMap::new());
                assert_eq!(ob.spread(), None);
            }
        }
    }

    #[test]
    fn three_resting_orders_stacked() {
        for (bid_ask, ask_bid) in &BID_ASK_COMBINATIONS {
            let (ob, results) = init_ob(vec![
                OrderType::Limit {
                    id: 0,
                    side: *bid_ask,
                    qty: 12,
                    price: 395,
                },
                OrderType::Limit {
                    id: 1,
                    side: *ask_bid,
                    qty: 2,
                    price: 399,
                },
                OrderType::Limit {
                    id: 2,
                    side: *bid_ask,
                    qty: 2,
                    price: 398,
                },
            ]);
            if *bid_ask == Side::Bid {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Placed { id: 1 },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(ob.min_ask(), Some(399));
                assert_eq!(ob.max_bid(), Some(398));
                assert_eq!(ob._asks(), init_book(vec![(399, 9998)]));
                assert_eq!(
                    ob._bids(),
                    init_book(vec![(398, 9997), (395, 9999)])
                );
                assert_eq!(ob.spread(), Some(1));
            } else {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Filled {
                            id: 1,
                            filled_qty: 2,
                            fills: vec![FillMetadata {
                                order_1: 1,
                                order_2: 0,
                                qty: 2,
                                price: 395,
                                taker_side: *ask_bid,
                                total_fill: false,
                            }],
                        },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(ob.min_ask(), Some(395));
                assert_eq!(ob.max_bid(), None);
                assert_eq!(
                    ob._asks(),
                    init_book(vec![(398, 9998), (395, 9999)])
                );
                assert_eq!(ob._bids(), init_book(vec![]));
                assert_eq!(ob.spread(), None);
            }
        }
    }

    #[test]
    fn crossing_limit_order_partial() {
        for (bid_ask, ask_bid) in &BID_ASK_COMBINATIONS {
            let (mut ob, results) = init_ob(vec![
                OrderType::Limit {
                    id: 0,
                    side: *bid_ask,
                    qty: 12,
                    price: 395,
                },
                OrderType::Limit {
                    id: 1,
                    side: *ask_bid,
                    qty: 2,
                    price: 399,
                },
                OrderType::Limit {
                    id: 2,
                    side: *bid_ask,
                    qty: 2,
                    price: 398,
                },
            ]);
            let result = ob.execute(OrderType::Limit {
                id: 3,
                side: *ask_bid,
                qty: 1,
                price: 397,
            });

            if *bid_ask == Side::Bid {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Placed { id: 1 },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(
                    result,
                    OrderEvent::Filled {
                        id: 3,
                        filled_qty: 1,
                        fills: vec![FillMetadata {
                            order_1: 3,
                            order_2: 2,
                            qty: 1,
                            price: 398,
                            taker_side: *ask_bid,
                            total_fill: false,
                        }]
                    }
                );
                assert_eq!(ob.min_ask(), Some(399));
                assert_eq!(ob.max_bid(), Some(398));
                assert_eq!(ob._asks(), init_book(vec![(399, 9998)]));
                assert_eq!(
                    ob._bids(),
                    init_book(vec![(398, 9997), (395, 9999)])
                );
                assert_eq!(ob.spread(), Some(1));
            } else {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Filled {
                            id: 1,
                            filled_qty: 2,
                            fills: vec![FillMetadata {
                                order_1: 1,
                                order_2: 0,
                                qty: 2,
                                price: 395,
                                taker_side: *ask_bid,
                                total_fill: false,
                            }],
                        },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(
                    result,
                    OrderEvent::Filled {
                        id: 3,
                        filled_qty: 1,
                        fills: vec![FillMetadata {
                            order_1: 3,
                            order_2: 0,
                            qty: 1,
                            price: 395,
                            taker_side: *ask_bid,
                            total_fill: false,
                        }]
                    }
                );
                assert_eq!(ob.min_ask(), Some(395));
                assert_eq!(ob.max_bid(), None);
                assert_eq!(
                    ob._asks(),
                    init_book(vec![(398, 9998), (395, 9999)])
                );
                assert_eq!(ob._bids(), init_book(vec![]));
                assert_eq!(ob.spread(), None);
            }
        }
    }

    #[test]
    fn crossing_limit_order_matching() {
        for (bid_ask, ask_bid) in &BID_ASK_COMBINATIONS {
            let (mut ob, results) = init_ob(vec![
                OrderType::Limit {
                    id: 0,
                    side: *bid_ask,
                    qty: 12,
                    price: 395,
                },
                OrderType::Limit {
                    id: 1,
                    side: *ask_bid,
                    qty: 2,
                    price: 399,
                },
                OrderType::Limit {
                    id: 2,
                    side: *bid_ask,
                    qty: 2,
                    price: 398,
                },
            ]);
            let result = ob.execute(OrderType::Limit {
                id: 3,
                side: *ask_bid,
                qty: 2,
                price: 397,
            });

            if *bid_ask == Side::Bid {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Placed { id: 1 },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(
                    result,
                    OrderEvent::Filled {
                        id: 3,
                        filled_qty: 2,
                        fills: vec![FillMetadata {
                            order_1: 3,
                            order_2: 2,
                            qty: 2,
                            price: 398,
                            taker_side: *ask_bid,
                            total_fill: true,
                        }]
                    }
                );
                assert_eq!(ob.min_ask(), Some(399));
                assert_eq!(ob.max_bid(), Some(395));
                assert_eq!(ob._asks(), init_book(vec![(399, 9998)]));
                assert_eq!(
                    ob._bids(),
                    init_book_holes(vec![(395, 9999)], vec![398])
                );
                assert_eq!(ob.spread(), Some(4));
            } else {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Filled {
                            id: 1,
                            filled_qty: 2,
                            fills: vec![FillMetadata {
                                order_1: 1,
                                order_2: 0,
                                qty: 2,
                                price: 395,
                                taker_side: *ask_bid,
                                total_fill: false,
                            }],
                        },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(
                    result,
                    OrderEvent::Filled {
                        id: 3,
                        filled_qty: 2,
                        fills: vec![FillMetadata {
                            order_1: 3,
                            order_2: 0,
                            qty: 2,
                            price: 395,
                            taker_side: *ask_bid,
                            total_fill: false,
                        }]
                    }
                );
                assert_eq!(ob.min_ask(), Some(395));
                assert_eq!(ob.max_bid(), None);
                assert_eq!(
                    ob._asks(),
                    init_book(vec![(395, 9999), (398, 9998)])
                );
                assert_eq!(ob._bids(), init_book(vec![]));
                assert_eq!(ob.spread(), None);
            }
        }
    }

    #[test]
    fn crossing_limit_order_over() {
        for (bid_ask, ask_bid) in &BID_ASK_COMBINATIONS {
            let (mut ob, results) = init_ob(vec![
                OrderType::Limit {
                    id: 0,
                    side: *bid_ask,
                    qty: 12,
                    price: 395,
                },
                OrderType::Limit {
                    id: 1,
                    side: *ask_bid,
                    qty: 2,
                    price: 399,
                },
                OrderType::Limit {
                    id: 2,
                    side: *bid_ask,
                    qty: 2,
                    price: 398,
                },
            ]);
            let result = ob.execute(OrderType::Limit {
                id: 3,
                side: *ask_bid,
                qty: 5,
                price: 397,
            });

            if *bid_ask == Side::Bid {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Placed { id: 1 },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(
                    result,
                    OrderEvent::PartiallyFilled {
                        id: 3,
                        filled_qty: 2,
                        fills: vec![FillMetadata {
                            order_1: 3,
                            order_2: 2,
                            qty: 2,
                            price: 398,
                            taker_side: *ask_bid,
                            total_fill: true,
                        }]
                    }
                );
                assert_eq!(ob.min_ask(), Some(397));
                assert_eq!(ob.max_bid(), Some(395));
                assert_eq!(
                    ob._asks(),
                    init_book(vec![(399, 9998), (397, 9996)])
                );
                assert_eq!(
                    ob._bids(),
                    init_book_holes(vec![(395, 9999)], vec![398])
                );
                assert_eq!(ob.spread(), Some(2));
            } else {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Filled {
                            id: 1,
                            filled_qty: 2,
                            fills: vec![FillMetadata {
                                order_1: 1,
                                order_2: 0,
                                qty: 2,
                                price: 395,
                                taker_side: *ask_bid,
                                total_fill: false,
                            }],
                        },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(
                    result,
                    OrderEvent::Filled {
                        id: 3,
                        filled_qty: 5,
                        fills: vec![FillMetadata {
                            order_1: 3,
                            order_2: 0,
                            qty: 5,
                            price: 395,
                            taker_side: *ask_bid,
                            total_fill: false,
                        }]
                    }
                );
                assert_eq!(ob.min_ask(), Some(395));
                assert_eq!(ob.max_bid(), None);
                assert_eq!(
                    ob._asks(),
                    init_book(vec![(395, 9999), (398, 9998)])
                );
                assert_eq!(ob._bids(), init_book(vec![]));
                assert_eq!(ob.spread(), None);
            }
        }
    }

    #[test]
    fn market_order_unfilled() {
        for (_, ask_bid) in &BID_ASK_COMBINATIONS {
            let (mut ob, _) = init_ob(vec![]);
            let result = ob.execute(OrderType::Market {
                id: 0,
                side: *ask_bid,
                qty: 5,
            });

            assert_eq!(result, OrderEvent::Unfilled { id: 0 });
        }
    }

    #[test]
    fn market_order_partially_filled() {
        for (bid_ask, ask_bid) in &BID_ASK_COMBINATIONS {
            let (mut ob, results) = init_ob(vec![
                OrderType::Limit {
                    id: 0,
                    side: *bid_ask,
                    qty: 12,
                    price: 395,
                },
                OrderType::Limit {
                    id: 1,
                    side: *ask_bid,
                    qty: 2,
                    price: 399,
                },
                OrderType::Limit {
                    id: 2,
                    side: *bid_ask,
                    qty: 2,
                    price: 398,
                },
            ]);
            let result = ob.execute(OrderType::Market {
                id: 3,
                side: *ask_bid,
                qty: 15,
            });

            if *bid_ask == Side::Bid {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Placed { id: 1 },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(
                    result,
                    OrderEvent::PartiallyFilled {
                        id: 3,
                        filled_qty: 14,
                        fills: vec![
                            FillMetadata {
                                order_1: 3,
                                order_2: 2,
                                qty: 2,
                                price: 398,
                                taker_side: *ask_bid,
                                total_fill: true,
                            },
                            FillMetadata {
                                order_1: 3,
                                order_2: 0,
                                qty: 12,
                                price: 395,
                                taker_side: *ask_bid,
                                total_fill: true,
                            }
                        ]
                    }
                );
                assert_eq!(ob.min_ask(), Some(399));
                assert_eq!(ob.max_bid(), None);
                assert_eq!(ob._asks(), init_book(vec![(399, 9998)]));
                assert_eq!(ob._bids(), init_book_holes(vec![], vec![395, 398]));
                assert_eq!(ob.spread(), None);
            } else {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Filled {
                            id: 1,
                            filled_qty: 2,
                            fills: vec![FillMetadata {
                                order_1: 1,
                                order_2: 0,
                                qty: 2,
                                price: 395,
                                taker_side: *ask_bid,
                                total_fill: false,
                            }],
                        },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(
                    result,
                    OrderEvent::PartiallyFilled {
                        id: 3,
                        filled_qty: 12,
                        fills: vec![
                            FillMetadata {
                                order_1: 3,
                                order_2: 0,
                                qty: 10,
                                price: 395,
                                taker_side: *ask_bid,
                                total_fill: true,
                            },
                            FillMetadata {
                                order_1: 3,
                                order_2: 2,
                                qty: 2,
                                price: 398,
                                taker_side: *ask_bid,
                                total_fill: true,
                            }
                        ]
                    }
                );
                assert_eq!(ob.min_ask(), None);
                assert_eq!(ob.max_bid(), None);
                assert_eq!(ob._asks(), init_book_holes(vec![], vec![395, 398]));
                assert_eq!(ob._bids(), init_book(vec![]));
                assert_eq!(ob.spread(), None);
            }
        }
    }

    #[test]
    fn market_order_filled() {
        for (bid_ask, ask_bid) in &BID_ASK_COMBINATIONS {
            let (mut ob, results) = init_ob(vec![
                OrderType::Limit {
                    id: 0,
                    side: *bid_ask,
                    qty: 12,
                    price: 395,
                },
                OrderType::Limit {
                    id: 1,
                    side: *ask_bid,
                    qty: 2,
                    price: 399,
                },
                OrderType::Limit {
                    id: 2,
                    side: *bid_ask,
                    qty: 2,
                    price: 398,
                },
            ]);
            let result = ob.execute(OrderType::Market {
                id: 3,
                side: *ask_bid,
                qty: 7,
            });

            if *bid_ask == Side::Bid {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Placed { id: 1 },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(
                    result,
                    OrderEvent::Filled {
                        id: 3,
                        filled_qty: 7,
                        fills: vec![
                            FillMetadata {
                                order_1: 3,
                                order_2: 2,
                                qty: 2,
                                price: 398,
                                taker_side: *ask_bid,
                                total_fill: true,
                            },
                            FillMetadata {
                                order_1: 3,
                                order_2: 0,
                                qty: 5,
                                price: 395,
                                taker_side: *ask_bid,
                                total_fill: false,
                            }
                        ]
                    }
                );
                assert_eq!(ob.min_ask(), Some(399));
                assert_eq!(ob.max_bid(), Some(395));
                assert_eq!(ob._asks(), init_book(vec![(399, 9998)]));
                assert_eq!(
                    ob._bids(),
                    init_book_holes(vec![(395, 9999)], vec![398])
                );
                assert_eq!(ob.spread(), Some(4));
            } else {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Filled {
                            id: 1,
                            filled_qty: 2,
                            fills: vec![FillMetadata {
                                order_1: 1,
                                order_2: 0,
                                qty: 2,
                                price: 395,
                                taker_side: *ask_bid,
                                total_fill: false,
                            }],
                        },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(
                    result,
                    OrderEvent::Filled {
                        id: 3,
                        filled_qty: 7,
                        fills: vec![FillMetadata {
                            order_1: 3,
                            order_2: 0,
                            qty: 7,
                            price: 395,
                            taker_side: *ask_bid,
                            total_fill: false,
                        }]
                    }
                );
                assert_eq!(ob.min_ask(), Some(395));
                assert_eq!(ob.max_bid(), None);
                assert_eq!(
                    ob._asks(),
                    init_book(vec![(395, 9999), (398, 9998)])
                );
                assert_eq!(ob._bids(), init_book(vec![]));
                assert_eq!(ob.spread(), None);
            }
        }
    }

    #[test]
    fn cancel_non_existing_order() {
        let (mut ob, _) = init_ob(vec![]);
        let result = ob.execute(OrderType::Cancel { id: 0 });
        assert_eq!(result, OrderEvent::Canceled { id: 0 });
        assert_eq!(ob.min_ask(), None);
        assert_eq!(ob.max_bid(), None);
        assert_eq!(ob._asks(), BTreeMap::new());
        assert_eq!(ob._bids(), BTreeMap::new());
        assert_eq!(ob.spread(), None);
    }

    #[test]
    fn cancel_resting_order() {
        for (bid_ask, _) in &BID_ASK_COMBINATIONS {
            let (mut ob, results) = init_ob(vec![OrderType::Limit {
                id: 0,
                side: *bid_ask,
                qty: 12,
                price: 395,
            }]);
            let result = ob.execute(OrderType::Cancel { id: 0 });
            assert_eq!(results, vec![OrderEvent::Placed { id: 0 }]);
            assert_eq!(result, OrderEvent::Canceled { id: 0 });
            assert_eq!(ob.min_ask(), None);
            assert_eq!(ob.max_bid(), None);
            if *bid_ask == Side::Bid {
                assert_eq!(ob._asks(), BTreeMap::new());
                assert_eq!(ob._bids(), init_book_holes(vec![], vec![395]));
            } else {
                assert_eq!(ob._asks(), init_book_holes(vec![], vec![395]));
                assert_eq!(ob._bids(), BTreeMap::new());
            }
            assert_eq!(ob.spread(), None);
        }
    }

    #[test]
    fn cancel_resting_order_of_many() {
        for (bid_ask, ask_bid) in &BID_ASK_COMBINATIONS {
            let (mut ob, results) = init_ob(vec![
                OrderType::Limit {
                    id: 0,
                    side: *bid_ask,
                    qty: 12,
                    price: 395,
                },
                OrderType::Limit {
                    id: 1,
                    side: *ask_bid,
                    qty: 2,
                    price: 399,
                },
                OrderType::Limit {
                    id: 2,
                    side: *bid_ask,
                    qty: 2,
                    price: 398,
                },
            ]);
            let result = ob.execute(OrderType::Cancel { id: 0 });
            if *bid_ask == Side::Bid {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Placed { id: 1 },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(result, OrderEvent::Canceled { id: 0 });
                assert_eq!(ob.min_ask(), Some(399));
                assert_eq!(ob.max_bid(), Some(398));
                assert_eq!(ob._asks(), init_book(vec![(399, 9998)]));
                assert_eq!(
                    ob._bids(),
                    init_book_holes(vec![(398, 9997)], vec![395])
                );
                assert_eq!(ob.spread(), Some(1));
            } else {
                assert_eq!(
                    results,
                    vec![
                        OrderEvent::Placed { id: 0 },
                        OrderEvent::Filled {
                            id: 1,
                            filled_qty: 2,
                            fills: vec![FillMetadata {
                                order_1: 1,
                                order_2: 0,
                                qty: 2,
                                price: 395,
                                taker_side: *ask_bid,
                                total_fill: false,
                            }],
                        },
                        OrderEvent::Placed { id: 2 }
                    ]
                );
                assert_eq!(result, OrderEvent::Canceled { id: 0 });
                assert_eq!(ob.min_ask(), Some(398));
                assert_eq!(ob.max_bid(), None);
                assert_eq!(
                    ob._asks(),
                    init_book_holes(vec![(398, 9998)], vec![395])
                );
                assert_eq!(ob._bids(), init_book(vec![]));
                assert_eq!(ob.spread(), None);
            }
        }
    }
}

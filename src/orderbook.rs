use std::collections::{BTreeMap, VecDeque};

use crate::models::{FillMetadata, OrderEvent, OrderEventResult, Side};
use crate::ordermap::OrderMap;

const DEFAULT_MAP_SIZE: usize = 10_000;
const DEFAULT_QUEUE_SIZE: usize = 10;

#[derive(Debug)]
pub struct OrderBook {
    min_ask: Option<u64>,
    max_bid: Option<u64>,
    asks: BTreeMap<u64, VecDeque<usize>>,
    bids: BTreeMap<u64, VecDeque<usize>>,
    orders: OrderMap,
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            min_ask: None,
            max_bid: None,
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            orders: OrderMap::new(DEFAULT_MAP_SIZE),
        }
    }

    pub fn spread(&self) -> Option<u64> {
        match (self.max_bid, self.min_ask) {
            (Some(b), Some(a)) => Some(a - b),
            _ => None,
        }
    }

    pub fn event(&mut self, event: OrderEvent) -> OrderEventResult {
        match event {
            OrderEvent::Market { id, side, qty } => {
                let (fills, partial) = self.market(id, side, qty);
                if fills.is_empty() {
                    OrderEventResult::Unfilled
                } else {
                    match partial {
                        false => OrderEventResult::Filled(fills),
                        true => OrderEventResult::PartiallyFilled(fills),
                    }
                }
            }
            OrderEvent::Limit {
                id,
                side,
                qty,
                price,
            } => {
                let (fills, partial) = self.limit(id, side, qty, price);
                if fills.is_empty() {
                    OrderEventResult::Placed(id)
                } else {
                    match partial {
                        false => OrderEventResult::Filled(fills),
                        true => OrderEventResult::PartiallyFilled(fills),
                    }
                }
            }
            OrderEvent::Cancel(id) => {
                self.cancel(id);
                OrderEventResult::Canceled(id)
            }
        }
    }

    fn cancel(&mut self, id: u128) -> bool {
        self.orders.delete(&id)
    }

    fn market(
        &mut self,
        id: u128,
        side: Side,
        qty: u64,
    ) -> (Vec<FillMetadata>, bool) {
        let mut partial = true;
        let mut update_bid_ask = false;
        let mut fills = Vec::new();
        let mut remaining_qty = qty;

        match side {
            Side::Bid => {
                for (ask_price, queue) in self.asks.iter_mut() {
                    if update_bid_ask {
                        self.min_ask = Some(*ask_price);
                        update_bid_ask = false;
                    }
                    if remaining_qty == 0 {
                        partial = false;
                        break;
                    }
                    let (new_fills, filled_qty) = Self::process_queue(
                        &mut self.orders,
                        queue,
                        remaining_qty,
                        id,
                    );
                    if queue.is_empty() {
                        update_bid_ask = true;
                    }
                    remaining_qty -= filled_qty;
                    fills.extend(new_fills);
                }
            }
            Side::Ask => {
                for (bid_price, queue) in self.bids.iter_mut().rev() {
                    if update_bid_ask {
                        self.max_bid = Some(*bid_price);
                        update_bid_ask = false;
                    }
                    if remaining_qty == 0 {
                        partial = false;
                        break;
                    }
                    let (new_fills, filled_qty) = Self::process_queue(
                        &mut self.orders,
                        queue,
                        remaining_qty,
                        id,
                    );
                    if queue.is_empty() {
                        update_bid_ask = true;
                    }
                    remaining_qty -= filled_qty;
                    fills.extend(new_fills);
                }
            }
        }

        (fills, partial)
    }

    fn limit(
        &mut self,
        id: u128,
        side: Side,
        qty: u64,
        price: u64,
    ) -> (Vec<FillMetadata>, bool) {
        let mut partial = true;
        let mut update_bid_ask = false;
        let mut fills: Vec<FillMetadata> = Vec::new();
        let mut remaining_qty: u64 = qty;

        match side {
            Side::Bid => {
                for (ask_price, queue) in self.asks.iter_mut() {
                    if update_bid_ask || self.min_ask.is_none() {
                        self.min_ask = Some(*ask_price);
                        update_bid_ask = false;
                    }
                    if remaining_qty == 0 || price < *ask_price {
                        partial = false;
                        break;
                    }
                    let (new_fills, filled_qty) = Self::process_queue(
                        &mut self.orders,
                        queue,
                        remaining_qty,
                        id,
                    );
                    if queue.is_empty() {
                        update_bid_ask = true;
                    }
                    remaining_qty -= filled_qty;
                    fills.extend(new_fills);
                }
                if remaining_qty > 0 {
                    let index = self.orders.insert(id, price, qty);
                    match self.max_bid {
                        None => {
                            self.max_bid = Some(price);
                        }
                        Some(b) if price > b => {
                            self.max_bid = Some(price);
                        }
                        _ => {}
                    };
                    self.bids
                        .entry(price)
                        .or_insert_with(|| {
                            VecDeque::with_capacity(DEFAULT_QUEUE_SIZE)
                        })
                        .push_back(index);
                }
            }
            Side::Ask => {
                for (bid_price, queue) in self.bids.iter_mut() {
                    if update_bid_ask || self.max_bid.is_none() {
                        self.max_bid = Some(*bid_price);
                        update_bid_ask = false;
                    }
                    if remaining_qty == 0 || price > *bid_price {
                        partial = false;
                        break;
                    }
                    let (new_fills, filled_qty) = Self::process_queue(
                        &mut self.orders,
                        queue,
                        remaining_qty,
                        id,
                    );
                    if queue.is_empty() {
                        update_bid_ask = true;
                    }
                    remaining_qty -= filled_qty;
                    fills.extend(new_fills);
                }
                if remaining_qty > 0 {
                    let index = self.orders.insert(id, price, qty);
                    if let Some(a) = self.min_ask {
                        if price < a {
                            self.min_ask = Some(price);
                        }
                    }
                    match self.min_ask {
                        None => {
                            self.min_ask = Some(price);
                        }
                        Some(a) if price < a => {
                            self.min_ask = Some(price);
                        }
                        _ => {}
                    };
                    self.asks
                        .entry(price)
                        .or_insert_with(|| {
                            VecDeque::with_capacity(DEFAULT_QUEUE_SIZE)
                        })
                        .push_back(index);
                }
            }
        }

        (fills, partial)
    }

    fn process_queue(
        orders: &mut OrderMap,
        opposite_orders: &mut VecDeque<usize>,
        quantity_still_to_trade: u64,
        id: u128,
    ) -> (Vec<FillMetadata>, u64) {
        let mut fills: Vec<FillMetadata> = Vec::new();
        let mut qty_to_fill = quantity_still_to_trade;
        let mut filled_qty = 0;
        let mut filled_index = None;

        for (index, head_order_idx) in opposite_orders.iter_mut().enumerate() {
            if qty_to_fill == 0 {
                break;
            }
            let head_order = &mut orders[*head_order_idx];
            let traded_price = head_order.price;
            let available_qty = head_order.qty;
            if available_qty == 0 {
                filled_index = Some(index);
                continue;
            }
            let traded_quantity: u64;

            if qty_to_fill >= available_qty {
                traded_quantity = available_qty;
                qty_to_fill -= available_qty;
                filled_index = Some(index);
            } else {
                traded_quantity = qty_to_fill;
                qty_to_fill = 0u64;
            }
            head_order.qty -= traded_quantity;
            let fill: FillMetadata;
            fill = FillMetadata {
                order_1: id,
                order_2: head_order.id,
                qty: traded_quantity,
                price: traded_price,
            };
            fills.push(fill);
            filled_qty += traded_quantity;
        }
        if let Some(index) = filled_index {
            opposite_orders.drain(0..index + 1);
        }

        (fills, filled_qty)
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{OrderEvent, OrderEventResult, Side};
    use crate::orderbook::{OrderBook, DEFAULT_QUEUE_SIZE};
    use std::collections::{BTreeMap, VecDeque};

    fn init_ob(events: Vec<OrderEvent>) -> (OrderBook, Vec<OrderEventResult>) {
        let mut ob = OrderBook::default();
        let mut results = Vec::new();
        for e in events {
            results.push(ob.event(e));
        }
        (ob, results)
    }

    fn init_book(orders: Vec<(u64, usize)>) -> BTreeMap<u64, VecDeque<usize>> {
        let mut bk = BTreeMap::new();
        for (p, i) in orders {
            bk.entry(p)
                .or_insert_with(|| VecDeque::with_capacity(DEFAULT_QUEUE_SIZE))
                .push_back(i);
        }
        bk
    }

    #[test]
    fn empty_book() {
        let (ob, results) = init_ob(Vec::new());
        assert_eq!(results, Vec::new());
        assert_eq!(ob.min_ask, None);
        assert_eq!(ob.max_bid, None);
        assert_eq!(ob.asks, BTreeMap::new());
        assert_eq!(ob.bids, BTreeMap::new());
        assert_eq!(ob.spread(), None);
    }

    #[test]
    fn one_resting_order() {
        let (ob, results) = init_ob(vec![OrderEvent::Limit {
            id: 0,
            side: Side::Bid,
            qty: 12,
            price: 395,
        }]);
        assert_eq!(results, vec![OrderEventResult::Placed(0)]);
        assert_eq!(ob.min_ask, None);
        assert_eq!(ob.max_bid, Some(395));
        assert_eq!(ob.asks, BTreeMap::new());
        assert_eq!(ob.bids, init_book(vec![(395, 9999)]));
        assert_eq!(ob.spread(), None);
    }

    #[test]
    fn two_resting_orders() {
        let (ob, results) = init_ob(vec![
            OrderEvent::Limit {
                id: 0,
                side: Side::Bid,
                qty: 12,
                price: 395,
            },
            OrderEvent::Limit {
                id: 1,
                side: Side::Ask,
                qty: 2,
                price: 398,
            },
        ]);
        assert_eq!(
            results,
            vec![OrderEventResult::Placed(0), OrderEventResult::Placed(1)]
        );
        assert_eq!(ob.min_ask, Some(398));
        assert_eq!(ob.max_bid, Some(395));
        assert_eq!(ob.asks, init_book(vec![(398, 9998)]));
        assert_eq!(ob.bids, init_book(vec![(395, 9999)]));
        assert_eq!(ob.spread(), Some(3));
    }

    #[test]
    fn two_resting_orders_stacked() {
        let (ob, results) = init_ob(vec![
            OrderEvent::Limit {
                id: 0,
                side: Side::Bid,
                qty: 12,
                price: 395,
            },
            OrderEvent::Limit {
                id: 1,
                side: Side::Bid,
                qty: 2,
                price: 398,
            },
        ]);
        assert_eq!(
            results,
            vec![OrderEventResult::Placed(0), OrderEventResult::Placed(1)]
        );
        assert_eq!(ob.min_ask, None);
        assert_eq!(ob.max_bid, Some(398));
        assert_eq!(ob.asks, BTreeMap::new());
        assert_eq!(ob.bids, init_book(vec![(398, 9998), (395, 9999)]));
        assert_eq!(ob.spread(), None);
    }

    #[test]
    fn three_resting_orders_stacked() {
        let (ob, results) = init_ob(vec![
            OrderEvent::Limit {
                id: 0,
                side: Side::Bid,
                qty: 12,
                price: 395,
            },
            OrderEvent::Limit {
                id: 1,
                side: Side::Ask,
                qty: 2,
                price: 399,
            },
            OrderEvent::Limit {
                id: 1,
                side: Side::Bid,
                qty: 2,
                price: 398,
            },
        ]);
        assert_eq!(
            results,
            vec![
                OrderEventResult::Placed(0),
                OrderEventResult::Placed(1),
                OrderEventResult::Placed(1)
            ]
        );
        assert_eq!(ob.min_ask, Some(399));
        assert_eq!(ob.max_bid, Some(398));
        assert_eq!(ob.asks, init_book(vec![(399, 9998)]));
        assert_eq!(ob.bids, init_book(vec![(398, 9997), (395, 9999)]));
        assert_eq!(ob.spread(), Some(1));
    }

    #[test]
    fn crossing_limit_order_partial() {
        let (ob, results) = init_ob(vec![
            OrderEvent::Limit {
                id: 0,
                side: Side::Bid,
                qty: 12,
                price: 395,
            },
            OrderEvent::Limit {
                id: 1,
                side: Side::Ask,
                qty: 2,
                price: 399,
            },
            OrderEvent::Limit {
                id: 1,
                side: Side::Bid,
                qty: 2,
                price: 398,
            },
        ]);
        let result = ob.event(OrderEvent::Limit {
            id: 3,
            side: Side::Ask,
            qty: 1,
            price: 397,
        });

        assert_eq!(
            results,
            vec![
                OrderEventResult::Placed(0),
                OrderEventResult::Placed(1),
                OrderEventResult::Placed(1)
            ]
        );
        assert_eq!(result, OrderEventResult::Placed(3));
        assert_eq!(ob.min_ask, Some(399));
        assert_eq!(ob.max_bid, Some(398));
        assert_eq!(ob.asks, init_book(vec![(399, 9998)]));
        assert_eq!(ob.bids, init_book(vec![(398, 9997), (395, 9999)]));
        assert_eq!(ob.spread(), Some(1));
    }
}

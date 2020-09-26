use std::collections::BTreeMap;

use crate::arena::OrderArena;
use crate::models::{FillMetadata, OrderEvent, OrderType, Side};

const DEFAULT_ARENA_CAPACITY: usize = 10_000;
const DEFAULT_QUEUE_CAPACITY: usize = 10;

#[derive(Debug)]
pub struct OrderBook {
    pub min_ask: Option<u64>,
    pub max_bid: Option<u64>,
    pub asks: BTreeMap<u64, Vec<usize>>,
    pub bids: BTreeMap<u64, Vec<usize>>,
    pub arena: OrderArena,
    default_queue_capacity: usize,
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new(DEFAULT_ARENA_CAPACITY, DEFAULT_QUEUE_CAPACITY)
    }
}

impl OrderBook {
    pub fn new(arena_capacity: usize, queue_capacity: usize) -> Self {
        Self {
            min_ask: None,
            max_bid: None,
            asks: BTreeMap::new(),
            bids: BTreeMap::new(),
            arena: OrderArena::new(arena_capacity),
            default_queue_capacity: queue_capacity,
        }
    }

    pub fn spread(&self) -> Option<u64> {
        match (self.max_bid, self.min_ask) {
            (Some(b), Some(a)) => Some(a - b),
            _ => None,
        }
    }

    pub fn event(&mut self, event: OrderType) -> OrderEvent {
        match event {
            OrderType::Market { id, side, qty } => {
                let (fills, partial, filled_qty) = self.market(id, side, qty);
                if fills.is_empty() {
                    OrderEvent::Unfilled
                } else {
                    match partial {
                        false => OrderEvent::Filled {
                            id,
                            filled_qty,
                            fills,
                        },
                        true => OrderEvent::PartiallyFilled {
                            id,
                            filled_qty,
                            fills,
                        },
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
                    OrderEvent::Placed(id)
                } else {
                    match partial {
                        false => OrderEvent::Filled {
                            id,
                            filled_qty,
                            fills,
                        },
                        true => OrderEvent::PartiallyFilled {
                            id,
                            filled_qty,
                            fills,
                        },
                    }
                }
            }
            OrderType::Cancel(id) => {
                self.cancel(id);
                OrderEvent::Canceled(id)
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
        let mut partial = false;
        let remaining_qty;
        let mut fills = Vec::new();

        match side {
            Side::Bid => {
                remaining_qty = self.match_with_asks(id, qty, &mut fills, None);
                if remaining_qty > 0 {
                    partial = true;
                }
            }
            Side::Ask => {
                remaining_qty = self.match_with_bids(id, qty, &mut fills, None);
                if remaining_qty > 0 {
                    partial = true;
                }
            }
        }

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
                    match self.max_bid {
                        None => {
                            self.max_bid = Some(price);
                        }
                        Some(b) if price > b => {
                            self.max_bid = Some(price);
                        }
                        _ => {}
                    };
                    let queue_capacity = self.default_queue_capacity;
                    self.bids
                        .entry(price)
                        .or_insert_with(|| Vec::with_capacity(queue_capacity))
                        .push(index);
                    match self.max_bid {
                        None => self.max_bid = Some(price),
                        Some(p) => {
                            if p < price {
                                self.max_bid = Some(price);
                            }
                        },
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
                    match self.min_ask {
                        None => {
                            self.min_ask = Some(price);
                        }
                        Some(a) if price < a => {
                            self.min_ask = Some(price);
                        }
                        _ => {}
                    };
                    let queue_capacity = self.default_queue_capacity;
                    self.asks
                        .entry(price)
                        .or_insert_with(|| Vec::with_capacity(queue_capacity))
                        .push(index);
                    match self.min_ask {
                        None => self.min_ask = Some(price),
                        Some(p) => {
                            if p > price {
                                self.min_ask = Some(price);
                            }
                        },
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
        self.min_ask = match cur_asks.next() {
            None => None,
            Some((p, _)) => Some(*p),
        };
    }

    fn update_max_bid(&mut self) {
        let mut cur_bids =
            self.bids.iter().rev().filter(|(_, q)| !q.is_empty());
        self.max_bid = match cur_bids.next() {
            None => None,
            Some((p, _)) => Some(*p),
        };
    }

    fn process_queue(
        arena: &mut OrderArena,
        opposite_orders: &mut Vec<usize>,
        remaining_qty: u64,
        id: u128,
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

            if qty_to_fill >= available_qty {
                traded_quantity = available_qty;
                qty_to_fill -= available_qty;
                filled_index = Some(index);
            } else {
                traded_quantity = qty_to_fill;
                qty_to_fill = 0;
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

        filled_qty
    }
}

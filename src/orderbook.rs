use std::collections::{BTreeMap, VecDeque};

use crate::models::{FillMetadata, OrderEvent, OrderType, Side};
use crate::ordermap::OrderMap;

const DEFAULT_MAP_SIZE: usize = 10_000;
const DEFAULT_QUEUE_SIZE: usize = 10;

#[derive(Debug)]
pub struct OrderBook {
    pub min_ask: Option<u64>,
    pub max_bid: Option<u64>,
    pub asks: BTreeMap<u64, VecDeque<usize>>,
    pub bids: BTreeMap<u64, VecDeque<usize>>,
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

    pub fn event(&mut self, event: OrderType) -> OrderEvent {
        match event {
            OrderType::Market { id, side, qty } => {
                let (fills, partial, filled_qty) = self.market(id, side, qty);
                if fills.is_empty() {
                    OrderEvent::Unfilled
                } else {
                    match partial {
                        false => OrderEvent::Filled(filled_qty, fills),
                        true => OrderEvent::PartiallyFilled(filled_qty, fills),
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
                        false => OrderEvent::Filled(filled_qty, fills),
                        true => OrderEvent::PartiallyFilled(filled_qty, fills),
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
        self.orders.delete(&id)
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
                    let index = self.orders.insert(id, price, remaining_qty);
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
                remaining_qty =
                    self.match_with_bids(id, qty, &mut fills, Some(price));
                if remaining_qty > 0 {
                    partial = true;
                    let index = self.orders.insert(id, price, remaining_qty);
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
            let (new_fills, filled_qty) =
                Self::process_queue(&mut self.orders, queue, remaining_qty, id);
            if queue.is_empty() {
                update_bid_ask = true;
            }
            remaining_qty -= filled_qty;
            fills.extend(new_fills);
        }

        let mut cur_asks = self.asks.iter().filter(|(_, q)| !q.is_empty());
        self.min_ask = match cur_asks.next() {
            None => None,
            Some((p, _)) => Some(*p),
        };

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
            let (new_fills, filled_qty) =
                Self::process_queue(&mut self.orders, queue, remaining_qty, id);
            if queue.is_empty() {
                update_bid_ask = true;
            }
            remaining_qty -= filled_qty;
            fills.extend(new_fills);
        }

        let mut cur_bids =
            self.bids.iter().rev().filter(|(_, q)| !q.is_empty());
        self.max_bid = match cur_bids.next() {
            None => None,
            Some((p, _)) => Some(*p),
        };

        remaining_qty
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

use std::collections::{BTreeMap, VecDeque};

use crate::models::{FillMetadata, OrderEvent, OrderEventResult, Side};
use crate::ordermap::OrderMap;

#[derive(Debug)]
pub struct OrderBook {
    min_ask: Option<u64>,
    max_bid: Option<u64>,
    asks: BTreeMap<u64, VecDeque<usize>>,
    bids: BTreeMap<u64, VecDeque<usize>>,
    orders: OrderMap,
}

const DEFAULT_MAP_SIZE: usize = 10_000;
const DEFAULT_QUEUE_SIZE: usize = 10;

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
            },
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
            },
        }
    }

    fn cancel(&mut self, id: u128) -> bool {
        self.orders.delete(&id)
    }

    fn market(&mut self, id: u128, side: Side, qty: u64) -> (Vec<FillMetadata>, bool) {
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
                    if update_bid_ask {
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
                    let index: usize = self.orders.insert(id, price, qty);
                    self.bids
                        .entry(price)
                        .or_insert_with(|| VecDeque::with_capacity(DEFAULT_QUEUE_SIZE))
                        .push_back(index);
                }
            }
            Side::Ask => {
                for (bid_price, queue) in self.bids.iter_mut() {
                    if update_bid_ask {
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
                    let index: usize = self.orders.insert(id, price, qty);
                    self.asks
                        .entry(price)
                        .or_insert_with(|| VecDeque::with_capacity(DEFAULT_QUEUE_SIZE))
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
    use crate::order_book::{FillResult, LimitOrder, OrderBook, Side, Symbol};
    use crate::VecDeque;
    use u128::u128;

    fn assert_order(expected: &LimitOrder, actual: &LimitOrder) {
        assert_eq!(expected.amount, actual.amount);
        assert_eq!(expected.price, actual.price);
        assert_eq!(expected.side, actual.side);
        if !expected.id.is_nil() {
            assert_eq!(expected.id, actual.id);
        }
    }

    fn assert_orders(expected: Vec<LimitOrder>, actual: Vec<LimitOrder>) {
        assert_eq!(expected.len(), actual.len());
        for i in 0..actual.len() {
            assert_order(&expected[i], &actual[i]);
        }
    }

    fn assert_order_queue(expected: &VecDeque<LimitOrder>, actual: &VecDeque<LimitOrder>) {
        assert_eq!(expected.len(), actual.len());
        for i in 0..expected.len() {
            assert_order(expected.get(i).unwrap(), actual.get(i).unwrap());
        }
    }

    fn assert_order_book(
        expected: Vec<VecDeque<LimitOrder>>,
        actual: Vec<VecDeque<LimitOrder>>,
    ) {
        assert_eq!(expected.len(), actual.len());
        for i in 0..expected.len() {
            assert_order_queue(&expected[i], &actual[i])
        }
    }

    #[test]
    fn test_orders() {
        // Test structure: add all the orders,
        // assert book looks as expected.
        // remove all specified orders
        // assert book looks as expected.
        struct TestCase {
            add: Vec<LimitOrder>,
            expected_after_add: Vec<VecDeque<LimitOrder>>,
            remove: Vec<LimitOrder>,
            expected_after_remove: Vec<VecDeque<LimitOrder>>,
        };
        let test_cases = vec![
            // Single add remove
            TestCase {
                add: vec![LimitOrder {
                    id: u128::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 5,
                }],
                expected_after_add: vec![VecDeque::from(vec![LimitOrder {
                    id: u128::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 5,
                }])],
                remove: vec![LimitOrder {
                    id: u128::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 5,
                }],
                expected_after_remove: Vec::new(),
            },
            // Same price should queue
            TestCase {
                add: vec![
                    LimitOrder {
                        id: u128::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    },
                    LimitOrder {
                        id: u128::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    },
                ],
                expected_after_add: vec![VecDeque::from(vec![
                    LimitOrder {
                        id: u128::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    },
                    LimitOrder {
                        id: u128::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    },
                ])],
                remove: vec![LimitOrder {
                    id: u128::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 5,
                }],
                expected_after_remove: vec![VecDeque::from(vec![LimitOrder {
                    id: u128::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 5,
                }])],
            },
            // Maintain sort
            TestCase {
                add: vec![
                    LimitOrder {
                        id: u128::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 4,
                    },
                    LimitOrder {
                        id: u128::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    },
                ],
                expected_after_add: vec![
                    VecDeque::from(vec![LimitOrder {
                        id: u128::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    }]),
                    VecDeque::from(vec![LimitOrder {
                        id: u128::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 4,
                    }]),
                ],
                remove: Vec::new(),
                expected_after_remove: vec![
                    VecDeque::from(vec![LimitOrder {
                        id: u128::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    }]),
                    VecDeque::from(vec![LimitOrder {
                        id: u128::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 4,
                    }]),
                ],
            },
        ];
        for tc in test_cases.iter() {
            let mut buy_ob = OrderBook::new(Side::Buy);
            for &to_add in tc.add.iter() {
                let result = buy_ob.add_order(to_add);
                assert!(!result.is_err());
            }
            assert_order_book(buy_ob.get_book(), tc.expected_after_add.clone());
            for &to_remove in tc.remove.iter() {
                let result = buy_ob.remove_order(to_remove);
                assert!(!result.is_err());
            }
            assert_order_book(buy_ob.get_book(), tc.expected_after_remove.clone());
        }
    }

    fn create_order_book(side: Side, orders: Vec<LimitOrder>) -> OrderBook {
        let mut buy_ob = OrderBook::new(Side::Buy);
        for &order in orders.iter() {
            let result = buy_ob.add_order(order);
            assert!(!result.is_err());
        }
        return buy_ob;
    }

    #[test]
    fn test_order_fill() {
        let mut buy_ob = create_order_book(
            Side::Buy,
            vec![
                LimitOrder {
                    id: u128::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 4,
                },
                LimitOrder {
                    id: u128::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 5,
                },
            ],
        );

        let result = buy_ob.fill_order_helper(LimitOrder {
            id: u128::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            amount: 10,
            symbol: Symbol::AAPL,
            side: Side::Buy,
            price: 5,
        });
        // Must be opposite side
        assert!(result.is_err());

        // Sell for 3, should take any bids >= 3, best price first
        let result = buy_ob.fill_order_helper(LimitOrder {
            id: u128::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
            amount: 10,
            symbol: Symbol::AAPL,
            side: Side::Sell,
            price: 3,
        });
        assert!(!result.is_err());
        assert_orders(
            vec![LimitOrder {
                id: u128::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                amount: 10,
                symbol: Symbol::AAPL,
                side: Side::Buy,
                price: 5,
            }],
            result.unwrap(),
        );
        // Only the 4 should be left in the book
        assert_order_book(
            vec![VecDeque::from(vec![LimitOrder {
                id: u128::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                amount: 10,
                symbol: Symbol::AAPL,
                side: Side::Buy,
                price: 4,
            }])],
            buy_ob.get_book(),
        )
    }

    #[test]
    fn test_order_fill_split() {
        // 7 -> [6]
        // 5 -> [11]
        // 4 -> [10]
        // 3 -> [6, 3]
        let mut buy_ob = create_order_book(
            Side::Buy,
            vec![
                LimitOrder {
                    id: u128::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 4,
                },
                LimitOrder {
                    id: u128::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                    amount: 11,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 5,
                },
                LimitOrder {
                    id: u128::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
                    amount: 6,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 3,
                },
                LimitOrder {
                    id: u128::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
                    amount: 6,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 7,
                },
                LimitOrder {
                    id: u128::parse_str("00000000-0000-0000-0000-000000000004").unwrap(),
                    amount: 3,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 3,
                },
            ],
        );

        // Sell for 3, should take any bids >= 3, best price first
        // This order should eat the whole book except for the last buy
        // which it splits.
        let result = buy_ob.fill_order_helper(LimitOrder {
            id: u128::parse_str("00000000-0000-0000-0000-000000000005").unwrap(),
            amount: 35,
            symbol: Symbol::AAPL,
            side: Side::Sell,
            price: 3,
        });
        assert!(!result.is_err());
        // We ate 35 shares of the total 36 on the book.
        assert_order_book(
            vec![VecDeque::from(vec![LimitOrder {
                id: u128::nil(),
                amount: 1,
                symbol: Symbol::AAPL,
                side: Side::Buy,
                price: 3,
            }])],
            buy_ob.get_book(),
        )
    }

    #[test]
    fn test_average_price() {
        let orders = vec![
            LimitOrder {
                id: u128::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                amount: 10,
                symbol: Symbol::AAPL,
                side: Side::Buy,
                price: 4,
            },
            LimitOrder {
                id: u128::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                amount: 11,
                symbol: Symbol::AAPL,
                side: Side::Buy,
                price: 5,
            },
        ];
        let ob = OrderBook::new(Side::Buy);
        assert_eq!(ob.average_price(orders), 4.523809523809524);
    }

}

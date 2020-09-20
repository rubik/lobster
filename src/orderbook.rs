use log::debug;
use std::collections::VecDeque;
use uuid::Uuid;

#[derive(Debug, PartialEq)]
pub enum Side {
    Bid,
    Ask,
}

#[derive(Debug)]
pub struct LimitOrder {
    pub id: uuid::Uuid,
    pub amount: u64,
    pub price: u64,
}

#[derive(Debug)]
pub struct FillResult {
    pub avg_price: f64,
}

pub struct OrderBook {
    book: BTreeMap<u64, VecDeque<LimitOrder>>,
    cache: HashMap<uuid::Uuid, LimitOrder>,
}

impl OrderBook {
    pub fn new(side: Side) -> Self {
        Self {
            book: Vec::new(),
            cache: HashMap::new(),
        }
    }

    pub fn find_order(&self, u: uuid::Uuid) -> Option<(u64, usize)> {
        if let Some(ref order) = self.cache.get(u) {
            if let Some(ref queue) = self.book.get(order.price) {
                for (i, &o) in queue.iter() {
                    if o.id == u {
                        return Some((order.price, i));
                    }
                }
            } else {
                // This is an inconsistent state and should not be reachable. In
                // case everything goes wrong and we get here, we try to
                // partially fix things by deleting this stale order from the
                // cache.
                self.cache.remove(u);
            }
        }
        None
    }

    pub fn remove_order(&mut self, t: LimitOrder) -> Result<(), Box<std::error::Error>> {
        let order_indices = self.find_order(t);
        match order_indices {
            None => Err("no such order"),
            Some(queue_index, order_index) =>
                match self.book[queue_index].remove(order_index) {
                    None => Err("error removing"),
                    Some(_) => Ok(()),
                        if self.book[queue_index.unwrap()].len() == 0 {
                            debug!("no more orders at price point {}", t.price);
                            self.book.remove(queue_index.unwrap());
                        }
    }

    pub fn add_order(&mut self, order: LimitOrder) -> Result<LimitOrder, &'static str> {
        debug!("adding order {:?}", order);
        // If we find an entry at that price point, add it to the queue
        // Otherwise create a queue at that price point.
        let mut queue_index = None;
        let mut insert_index = None;

        for (index, order_queue) in self.book.iter().enumerate() {
            debug!("index {:?} order queue {:?}", index, order_queue);
            if order_queue.front().unwrap().price == t.price {
                queue_index = Some(index);
                break;
            } else if order_queue.front().unwrap().price < t.price && self.side == Side::Buy {
                insert_index = Some(index);
                break;
            } else if order_queue.front().unwrap().price > t.price && self.side == Side::Sell {
                insert_index = Some(index);
                break;
            }
        }

        match queue_index {
            Some(queue_index) => {
                // Existing orders at that price
                self.book[queue_index].push_back(order);
            }
            None => {
                // No existing orders at the price, create a new queue
                let mut orders: VecDeque<LimitOrder> = VecDeque::new();
                orders.push_back(order);
                // Put the queue in the right place
                match insert_index {
                    Some(insert_index) => {
                        // We know the spot to put this new queue
                        self.book.insert(insert_index, orders);
                    }
                    None => {
                        // Order book must be empty, just push the queue into the first spot
                        self.book.push(orders);
                    }
                }
            }
        };
        Ok(order)
    }

    pub fn valid_price(&self, to_fill_price: u32, candidate_order_price: u32) -> bool {
        if self.side == Side::Buy {
            return to_fill_price <= candidate_order_price;
        }
        return to_fill_price >= candidate_order_price;
    }

    // Returns orders on the other side that were used to fill the order.
    // Removes any orders that were used to fill from the book.
    // If sum(orders returns) > to_fill, then the last order was only partially used to fill.
    pub fn fill_order_helper(
        &mut self,
        to_fill: LimitOrder,
    ) -> Result<Vec<LimitOrder>, &'static str> {
        if to_fill.side == Side::Buy && self.side != Side::Sell {
            return Err("cannot fill buy order with sell book");
        }
        if to_fill.side == Side::Sell && self.side != Side::Buy {
            return Err("cannot fill sell order with buy  book");
        }

        debug!("orderbook size {}", self.book.len());
        if self.book.len() == 0 {
            return Err(ERR_CANT_FILL_SIZE);
        }

        // If the current price is no good break
        if !self.valid_price(to_fill.price, self.book[0].front().unwrap().price) {
            debug!("nothing available in book at valid price");
            return Err(ERR_CANT_FILL_PRICE);
        }

        let mut remaining: i32 = to_fill.amount as i32;
        let mut orders = Vec::new();

        // Drain each queue one by one as needed
        while self.valid_price(to_fill.price, self.book[0].front().unwrap().price) {
            let order = self.book[0].pop_front();
            match order {
                Some(order) => {
                    orders.push(order);
                    debug!("selecting order {:?}", order);
                    remaining = remaining - order.amount as i32;
                }
                None => {
                    debug!("drained the whole queue at current price, moving to next price");
                }
            }
            if self.book[0].len() == 0 {
                self.book.remove(0);
            }
            if remaining <= 0 {
                debug!("filled the order");
                break;
            }
            if self.book.len() == 0 {
                debug!("drained the whole book without filling the order");
                // Add all the order back if we fail to fill
                for &i in orders.iter() {
                    let result = self.add_order(i);
                    if result.is_err() {
                        panic!(result);
                    }
                }
                return Err(ERR_CANT_FILL_SIZE);
            }
        }

        if remaining > 0 {
            return Err(ERR_CANT_FILL_PRICE);
        }

        if remaining == 0 {
            // Exact fill
            return Ok(orders);
        }

        // We have some set of orders which covers the requested amount, but not exactly.
        // Add a new order which is this leftover to the book.
        let last_order = orders[orders.len() - 1];
        self.add_order(LimitOrder {
            id: last_order.id,
            price: last_order.price,
            side: last_order.side,
            amount: remaining.abs() as u32,
            symbol: last_order.symbol,
        });
        return Ok(orders);
    }

    pub fn average_price(&self, orders: Vec<LimitOrder>) -> f64 {
        let total_shares = orders.iter().fold(0, |sum, order| sum + order.amount);
        orders
            .iter()
            .fold(0, |sum, order| sum + order.price * order.amount) as f64
            / total_shares as f64
    }

    pub fn fill_order(&mut self, to_fill: LimitOrder) -> Result<FillResult, &'static str> {
        let orders_used = self.fill_order_helper(to_fill)?;
        let extra = orders_used.iter().fold(0, |sum, order| sum + order.amount) - to_fill.amount;
        if extra > 0 {
            // If the orders used is > then the desired fill, we had to split an order.
            // Include the appropriate portion of the split order in the average price calculation.
            let (orders_used, extra_order) = orders_used.split_at(orders_used.len() - 1);
            let mut orders_used = orders_used.clone().to_vec();
            let mut extra_order = extra_order[0].clone();
            extra_order.amount = extra;
            orders_used.push(extra_order);
            return Ok(FillResult {
                avg_price: self.average_price(orders_used),
            });
        }
        Ok(FillResult {
            avg_price: self.average_price(orders_used),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::order_book::{FillResult, LimitOrder, OrderBook, Side, Symbol};
    use crate::VecDeque;
    use uuid::Uuid;

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
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 5,
                }],
                expected_after_add: vec![VecDeque::from(vec![LimitOrder {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 5,
                }])],
                remove: vec![LimitOrder {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
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
                        id: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    },
                    LimitOrder {
                        id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    },
                ],
                expected_after_add: vec![VecDeque::from(vec![
                    LimitOrder {
                        id: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    },
                    LimitOrder {
                        id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    },
                ])],
                remove: vec![LimitOrder {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 5,
                }],
                expected_after_remove: vec![VecDeque::from(vec![LimitOrder {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
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
                        id: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 4,
                    },
                    LimitOrder {
                        id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    },
                ],
                expected_after_add: vec![
                    VecDeque::from(vec![LimitOrder {
                        id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    }]),
                    VecDeque::from(vec![LimitOrder {
                        id: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 4,
                    }]),
                ],
                remove: Vec::new(),
                expected_after_remove: vec![
                    VecDeque::from(vec![LimitOrder {
                        id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                        amount: 10,
                        symbol: Symbol::AAPL,
                        side: Side::Buy,
                        price: 5,
                    }]),
                    VecDeque::from(vec![LimitOrder {
                        id: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
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
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 4,
                },
                LimitOrder {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 5,
                },
            ],
        );

        let result = buy_ob.fill_order_helper(LimitOrder {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
            amount: 10,
            symbol: Symbol::AAPL,
            side: Side::Buy,
            price: 5,
        });
        // Must be opposite side
        assert!(result.is_err());

        // Sell for 3, should take any bids >= 3, best price first
        let result = buy_ob.fill_order_helper(LimitOrder {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
            amount: 10,
            symbol: Symbol::AAPL,
            side: Side::Sell,
            price: 3,
        });
        assert!(!result.is_err());
        assert_orders(
            vec![LimitOrder {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
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
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
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
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                    amount: 10,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 4,
                },
                LimitOrder {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
                    amount: 11,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 5,
                },
                LimitOrder {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap(),
                    amount: 6,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 3,
                },
                LimitOrder {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap(),
                    amount: 6,
                    symbol: Symbol::AAPL,
                    side: Side::Buy,
                    price: 7,
                },
                LimitOrder {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000004").unwrap(),
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
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000005").unwrap(),
            amount: 35,
            symbol: Symbol::AAPL,
            side: Side::Sell,
            price: 3,
        });
        assert!(!result.is_err());
        // We ate 35 shares of the total 36 on the book.
        assert_order_book(
            vec![VecDeque::from(vec![LimitOrder {
                id: Uuid::nil(),
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
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000010").unwrap(),
                amount: 10,
                symbol: Symbol::AAPL,
                side: Side::Buy,
                price: 4,
            },
            LimitOrder {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap(),
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

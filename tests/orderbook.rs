use lobster::{FillMetadata, OrderBook, OrderEvent, OrderType, Side};
use std::collections::BTreeMap;

const DEFAULT_QUEUE_SIZE: usize = 10;
const BID_ASK_COMBINATIONS: [(Side, Side); 2] =
    [(Side::Bid, Side::Ask), (Side::Ask, Side::Bid)];

fn init_ob(events: Vec<OrderType>) -> (OrderBook, Vec<OrderEvent>) {
    let mut ob = OrderBook::default();
    let mut results = Vec::new();
    for e in events {
        results.push(ob.event(e));
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
    assert_eq!(ob.min_ask, None);
    assert_eq!(ob.max_bid, None);
    assert_eq!(ob.asks, BTreeMap::new());
    assert_eq!(ob.bids, BTreeMap::new());
    assert_eq!(ob.spread(), None);
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
        assert_eq!(results, vec![OrderEvent::Placed(0)]);
        if *bid_ask == Side::Bid {
            assert_eq!(ob.min_ask, None);
            assert_eq!(ob.max_bid, Some(395));
            assert_eq!(ob.asks, BTreeMap::new());
            assert_eq!(ob.bids, init_book(vec![(395, 9999)]));
            assert_eq!(ob.spread(), None);
        } else {
            assert_eq!(ob.min_ask, Some(395));
            assert_eq!(ob.max_bid, None);
            assert_eq!(ob.asks, init_book(vec![(395, 9999)]));
            assert_eq!(ob.bids, BTreeMap::new());
            assert_eq!(ob.spread(), None);
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
                vec![OrderEvent::Placed(0), OrderEvent::Placed(1)]
            );
            assert_eq!(ob.min_ask, Some(398));
            assert_eq!(ob.max_bid, Some(395));
            assert_eq!(ob.asks, init_book(vec![(398, 9998)]));
            assert_eq!(ob.bids, init_book(vec![(395, 9999)]));
            assert_eq!(ob.spread(), Some(3));
        } else {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Filled {
                        id: 1,
                        filled_qty: 2,
                        fills: vec![FillMetadata {
                            order_1: 1,
                            order_2: 0,
                            qty: 2,
                            price: 395,
                        }],
                    }
                ]
            );
            assert_eq!(ob.min_ask, Some(395));
            assert_eq!(ob.max_bid, None);
            assert_eq!(ob.asks, init_book(vec![(395, 9999)]));
            assert_eq!(ob.bids, init_book(vec![]));
            assert_eq!(ob.spread(), None);
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
        assert_eq!(results, vec![OrderEvent::Placed(0), OrderEvent::Placed(1)]);
        if *bid_ask == Side::Bid {
            assert_eq!(ob.min_ask, None);
            assert_eq!(ob.max_bid, Some(395));
            assert_eq!(ob.asks, BTreeMap::new());
            assert_eq!(ob.bids, init_book(vec![(395, 9999), (395, 9998)]));
            assert_eq!(ob.spread(), None);
        } else {
            assert_eq!(ob.min_ask, Some(395));
            assert_eq!(ob.max_bid, None);
            assert_eq!(ob.asks, init_book(vec![(395, 9999), (395, 9998)]));
            assert_eq!(ob.bids, BTreeMap::new());
            assert_eq!(ob.spread(), None);
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
        assert_eq!(results, vec![OrderEvent::Placed(0), OrderEvent::Placed(1)]);
        if *bid_ask == Side::Bid {
            assert_eq!(ob.min_ask, None);
            assert_eq!(ob.max_bid, Some(398));
            assert_eq!(ob.asks, BTreeMap::new());
            assert_eq!(ob.bids, init_book(vec![(398, 9998), (395, 9999)]));
            assert_eq!(ob.spread(), None);
        } else {
            assert_eq!(ob.min_ask, Some(395));
            assert_eq!(ob.max_bid, None);
            assert_eq!(ob.asks, init_book(vec![(398, 9998), (395, 9999)]));
            assert_eq!(ob.bids, BTreeMap::new());
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
                    OrderEvent::Placed(0),
                    OrderEvent::Placed(1),
                    OrderEvent::Placed(2)
                ]
            );
            assert_eq!(ob.min_ask, Some(399));
            assert_eq!(ob.max_bid, Some(398));
            assert_eq!(ob.asks, init_book(vec![(399, 9998)]));
            assert_eq!(ob.bids, init_book(vec![(398, 9997), (395, 9999)]));
            assert_eq!(ob.spread(), Some(1));
        } else {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Filled {
                        id: 1,
                        filled_qty: 2,
                        fills: vec![FillMetadata {
                            order_1: 1,
                            order_2: 0,
                            qty: 2,
                            price: 395,
                        },],
                    },
                    OrderEvent::Placed(2)
                ]
            );
            assert_eq!(ob.min_ask, Some(395));
            assert_eq!(ob.max_bid, None);
            assert_eq!(ob.asks, init_book(vec![(398, 9998), (395, 9999)]));
            assert_eq!(ob.bids, init_book(vec![]));
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
        let result = ob.event(OrderType::Limit {
            id: 3,
            side: *ask_bid,
            qty: 1,
            price: 397,
        });

        if *bid_ask == Side::Bid {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Placed(1),
                    OrderEvent::Placed(2)
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
                        price: 398
                    }]
                }
            );
            assert_eq!(ob.min_ask, Some(399));
            assert_eq!(ob.max_bid, Some(398));
            assert_eq!(ob.asks, init_book(vec![(399, 9998)]));
            assert_eq!(ob.bids, init_book(vec![(398, 9997), (395, 9999)]));
            assert_eq!(ob.spread(), Some(1));
        } else {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Filled {
                        id: 1,
                        filled_qty: 2,
                        fills: vec![FillMetadata {
                            order_1: 1,
                            order_2: 0,
                            qty: 2,
                            price: 395,
                        },],
                    },
                    OrderEvent::Placed(2)
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
                        price: 395
                    }]
                }
            );
            assert_eq!(ob.min_ask, Some(395));
            assert_eq!(ob.max_bid, None);
            assert_eq!(ob.asks, init_book(vec![(398, 9998), (395, 9999)]));
            assert_eq!(ob.bids, init_book(vec![]));
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
        let result = ob.event(OrderType::Limit {
            id: 3,
            side: *ask_bid,
            qty: 2,
            price: 397,
        });

        if *bid_ask == Side::Bid {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Placed(1),
                    OrderEvent::Placed(2)
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
                        price: 398
                    }]
                }
            );
            assert_eq!(ob.min_ask, Some(399));
            assert_eq!(ob.max_bid, Some(395));
            assert_eq!(ob.asks, init_book(vec![(399, 9998)]));
            assert_eq!(ob.bids, init_book_holes(vec![(395, 9999)], vec![398]));
            assert_eq!(ob.spread(), Some(4));
        } else {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Filled {
                        id: 1,
                        filled_qty: 2,
                        fills: vec![FillMetadata {
                            order_1: 1,
                            order_2: 0,
                            qty: 2,
                            price: 395,
                        },],
                    },
                    OrderEvent::Placed(2)
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
                        price: 395
                    }]
                }
            );
            assert_eq!(ob.min_ask, Some(395));
            assert_eq!(ob.max_bid, None);
            assert_eq!(ob.asks, init_book(vec![(395, 9999), (398, 9998)]));
            assert_eq!(ob.bids, init_book(vec![]));
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
        let result = ob.event(OrderType::Limit {
            id: 3,
            side: *ask_bid,
            qty: 5,
            price: 397,
        });

        if *bid_ask == Side::Bid {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Placed(1),
                    OrderEvent::Placed(2)
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
                        price: 398
                    }]
                }
            );
            assert_eq!(ob.min_ask, Some(397));
            assert_eq!(ob.max_bid, Some(395));
            assert_eq!(ob.asks, init_book(vec![(399, 9998), (397, 9996)]));
            assert_eq!(ob.bids, init_book_holes(vec![(395, 9999)], vec![398]));
            assert_eq!(ob.spread(), Some(2));
        } else {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Filled {
                        id: 1,
                        filled_qty: 2,
                        fills: vec![FillMetadata {
                            order_1: 1,
                            order_2: 0,
                            qty: 2,
                            price: 395,
                        },],
                    },
                    OrderEvent::Placed(2)
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
                        price: 395
                    }]
                }
            );
            assert_eq!(ob.min_ask, Some(395));
            assert_eq!(ob.max_bid, None);
            assert_eq!(ob.asks, init_book(vec![(395, 9999), (398, 9998)]));
            assert_eq!(ob.bids, init_book(vec![]));
            assert_eq!(ob.spread(), None);
        }
    }
}

#[test]
fn market_order_unfilled() {
    for (_, ask_bid) in &BID_ASK_COMBINATIONS {
        let (mut ob, _) = init_ob(vec![]);
        let result = ob.event(OrderType::Market {
            id: 0,
            side: *ask_bid,
            qty: 5,
        });

        assert_eq!(result, OrderEvent::Unfilled);
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
        let result = ob.event(OrderType::Market {
            id: 3,
            side: *ask_bid,
            qty: 15,
        });

        if *bid_ask == Side::Bid {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Placed(1),
                    OrderEvent::Placed(2)
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
                            price: 398
                        },
                        FillMetadata {
                            order_1: 3,
                            order_2: 0,
                            qty: 12,
                            price: 395
                        }
                    ]
                }
            );
            assert_eq!(ob.min_ask, Some(399));
            assert_eq!(ob.max_bid, None);
            assert_eq!(ob.asks, init_book(vec![(399, 9998)]));
            assert_eq!(ob.bids, init_book_holes(vec![], vec![395, 398]));
            assert_eq!(ob.spread(), None);
        } else {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Filled {
                        id: 1,
                        filled_qty: 2,
                        fills: vec![FillMetadata {
                            order_1: 1,
                            order_2: 0,
                            qty: 2,
                            price: 395,
                        },],
                    },
                    OrderEvent::Placed(2)
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
                            price: 395
                        },
                        FillMetadata {
                            order_1: 3,
                            order_2: 2,
                            qty: 2,
                            price: 398
                        }
                    ]
                }
            );
            assert_eq!(ob.min_ask, None);
            assert_eq!(ob.max_bid, None);
            assert_eq!(ob.asks, init_book_holes(vec![], vec![395, 398]));
            assert_eq!(ob.bids, init_book(vec![]));
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
        let result = ob.event(OrderType::Market {
            id: 3,
            side: *ask_bid,
            qty: 7,
        });

        if *bid_ask == Side::Bid {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Placed(1),
                    OrderEvent::Placed(2)
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
                            price: 398
                        },
                        FillMetadata {
                            order_1: 3,
                            order_2: 0,
                            qty: 5,
                            price: 395
                        }
                    ]
                }
            );
            assert_eq!(ob.min_ask, Some(399));
            assert_eq!(ob.max_bid, Some(395));
            assert_eq!(ob.asks, init_book(vec![(399, 9998)]));
            assert_eq!(ob.bids, init_book_holes(vec![(395, 9999)], vec![398]));
            assert_eq!(ob.spread(), Some(4));
        } else {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Filled {
                        id: 1,
                        filled_qty: 2,
                        fills: vec![FillMetadata {
                            order_1: 1,
                            order_2: 0,
                            qty: 2,
                            price: 395,
                        },],
                    },
                    OrderEvent::Placed(2)
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
                        price: 395
                    },]
                }
            );
            assert_eq!(ob.min_ask, Some(395));
            assert_eq!(ob.max_bid, None);
            assert_eq!(ob.asks, init_book(vec![(395, 9999), (398, 9998)]));
            assert_eq!(ob.bids, init_book(vec![]));
            assert_eq!(ob.spread(), None);
        }
    }
}

#[test]
fn cancel_non_existing_order() {
    let (mut ob, _) = init_ob(vec![]);
    let result = ob.event(OrderType::Cancel(0));
    assert_eq!(result, OrderEvent::Canceled(0));
    assert_eq!(ob.min_ask, None);
    assert_eq!(ob.max_bid, None);
    assert_eq!(ob.asks, BTreeMap::new());
    assert_eq!(ob.bids, BTreeMap::new());
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
        let result = ob.event(OrderType::Cancel(0));
        assert_eq!(results, vec![OrderEvent::Placed(0)]);
        assert_eq!(result, OrderEvent::Canceled(0));
        assert_eq!(ob.min_ask, None);
        assert_eq!(ob.max_bid, None);
        if *bid_ask == Side::Bid {
            assert_eq!(ob.asks, BTreeMap::new());
            assert_eq!(ob.bids, init_book_holes(vec![], vec![395]));
        } else {
            assert_eq!(ob.asks, init_book_holes(vec![], vec![395]));
            assert_eq!(ob.bids, BTreeMap::new());
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
        let result = ob.event(OrderType::Cancel(0));
        if *bid_ask == Side::Bid {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Placed(1),
                    OrderEvent::Placed(2)
                ]
            );
            assert_eq!(result, OrderEvent::Canceled(0));
            assert_eq!(ob.min_ask, Some(399));
            assert_eq!(ob.max_bid, Some(398));
            assert_eq!(ob.asks, init_book(vec![(399, 9998)]));
            assert_eq!(ob.bids, init_book_holes(vec![(398, 9997)], vec![395]));
            assert_eq!(ob.spread(), Some(1));
        } else {
            assert_eq!(
                results,
                vec![
                    OrderEvent::Placed(0),
                    OrderEvent::Filled {
                        id: 1,
                        filled_qty: 2,
                        fills: vec![FillMetadata {
                            order_1: 1,
                            order_2: 0,
                            qty: 2,
                            price: 395,
                        },],
                    },
                    OrderEvent::Placed(2)
                ]
            );
            assert_eq!(result, OrderEvent::Canceled(0));
            assert_eq!(ob.min_ask, Some(398));
            assert_eq!(ob.max_bid, None);
            assert_eq!(ob.asks, init_book_holes(vec![(398, 9998)], vec![395]));
            assert_eq!(ob.bids, init_book(vec![]));
            assert_eq!(ob.spread(), None);
        }
    }
}

use lobster::{FillMetadata, OrderAction, OrderBook, OrderEvent, Side};
use std::collections::{BTreeMap, VecDeque};

const DEFAULT_QUEUE_SIZE: usize = 10;

fn init_ob(events: Vec<OrderAction>) -> (OrderBook, Vec<OrderEvent>) {
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

fn init_book_holes(
    orders: Vec<(u64, usize)>,
    holes: Vec<u64>,
) -> BTreeMap<u64, VecDeque<usize>> {
    let mut bk = init_book(orders);
    for h in holes {
        bk.insert(h, VecDeque::new());
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
    let (ob, results) = init_ob(vec![OrderAction::Limit {
        id: 0,
        side: Side::Bid,
        qty: 12,
        price: 395,
    }]);
    assert_eq!(results, vec![OrderEvent::Placed(0)]);
    assert_eq!(ob.min_ask, None);
    assert_eq!(ob.max_bid, Some(395));
    assert_eq!(ob.asks, BTreeMap::new());
    assert_eq!(ob.bids, init_book(vec![(395, 9999)]));
    assert_eq!(ob.spread(), None);
}

#[test]
fn two_resting_orders() {
    let (ob, results) = init_ob(vec![
        OrderAction::Limit {
            id: 0,
            side: Side::Bid,
            qty: 12,
            price: 395,
        },
        OrderAction::Limit {
            id: 1,
            side: Side::Ask,
            qty: 2,
            price: 398,
        },
    ]);
    assert_eq!(
        results,
        vec![OrderEvent::Placed(0), OrderEvent::Placed(1)]
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
        OrderAction::Limit {
            id: 0,
            side: Side::Bid,
            qty: 12,
            price: 395,
        },
        OrderAction::Limit {
            id: 1,
            side: Side::Bid,
            qty: 2,
            price: 398,
        },
    ]);
    assert_eq!(
        results,
        vec![OrderEvent::Placed(0), OrderEvent::Placed(1)]
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
        OrderAction::Limit {
            id: 0,
            side: Side::Bid,
            qty: 12,
            price: 395,
        },
        OrderAction::Limit {
            id: 1,
            side: Side::Ask,
            qty: 2,
            price: 399,
        },
        OrderAction::Limit {
            id: 2,
            side: Side::Bid,
            qty: 2,
            price: 398,
        },
    ]);
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
}

#[test]
fn crossing_limit_order_partial() {
    let (mut ob, results) = init_ob(vec![
        OrderAction::Limit {
            id: 0,
            side: Side::Bid,
            qty: 12,
            price: 395,
        },
        OrderAction::Limit {
            id: 1,
            side: Side::Ask,
            qty: 2,
            price: 399,
        },
        OrderAction::Limit {
            id: 2,
            side: Side::Bid,
            qty: 2,
            price: 398,
        },
    ]);
    let result = ob.event(OrderAction::Limit {
        id: 3,
        side: Side::Ask,
        qty: 1,
        price: 397,
    });

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
        OrderEvent::Filled(
            1,
            vec![FillMetadata {
                order_1: 3,
                order_2: 2,
                qty: 1,
                price: 398
            }]
        )
    );
    assert_eq!(ob.min_ask, Some(399));
    assert_eq!(ob.max_bid, Some(398));
    assert_eq!(ob.asks, init_book(vec![(399, 9998)]));
    assert_eq!(ob.bids, init_book(vec![(398, 9997), (395, 9999)]));
    assert_eq!(ob.spread(), Some(1));
}

#[test]
fn crossing_limit_order_matching() {
    let (mut ob, results) = init_ob(vec![
        OrderAction::Limit {
            id: 0,
            side: Side::Bid,
            qty: 12,
            price: 395,
        },
        OrderAction::Limit {
            id: 1,
            side: Side::Ask,
            qty: 2,
            price: 399,
        },
        OrderAction::Limit {
            id: 2,
            side: Side::Bid,
            qty: 2,
            price: 398,
        },
    ]);
    let result = ob.event(OrderAction::Limit {
        id: 3,
        side: Side::Ask,
        qty: 2,
        price: 397,
    });

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
        OrderEvent::Filled(
            2,
            vec![FillMetadata {
                order_1: 3,
                order_2: 2,
                qty: 2,
                price: 398
            }]
        )
    );
    assert_eq!(ob.min_ask, Some(399));
    assert_eq!(ob.max_bid, Some(395));
    assert_eq!(ob.asks, init_book(vec![(399, 9998)]));
    assert_eq!(ob.bids, init_book_holes(vec![(395, 9999)], vec![398]));
    assert_eq!(ob.spread(), Some(4));
}

#[test]
fn crossing_limit_order_over() {
    let (mut ob, results) = init_ob(vec![
        OrderAction::Limit {
            id: 0,
            side: Side::Bid,
            qty: 12,
            price: 395,
        },
        OrderAction::Limit {
            id: 1,
            side: Side::Ask,
            qty: 2,
            price: 399,
        },
        OrderAction::Limit {
            id: 2,
            side: Side::Bid,
            qty: 2,
            price: 398,
        },
    ]);
    let result = ob.event(OrderAction::Limit {
        id: 3,
        side: Side::Ask,
        qty: 5,
        price: 397,
    });

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
        OrderEvent::PartiallyFilled(
            2,
            vec![FillMetadata {
                order_1: 3,
                order_2: 2,
                qty: 2,
                price: 398
            }]
        )
    );
    assert_eq!(ob.min_ask, Some(397));
    assert_eq!(ob.max_bid, Some(395));
    assert_eq!(ob.asks, init_book(vec![(399, 9998), (397, 9996)]));
    assert_eq!(ob.bids, init_book_holes(vec![(395, 9999)], vec![398]));
    assert_eq!(ob.spread(), Some(2));
}

#[test]
fn market_order_unfilled() {
    let (mut ob, _) = init_ob(vec![]);
    let result = ob.event(OrderAction::Market {
        id: 0,
        side: Side::Ask,
        qty: 5,
    });

    assert_eq!(result, OrderEvent::Unfilled);
}

#[test]
fn market_order_partially_filled() {
    let (mut ob, results) = init_ob(vec![
        OrderAction::Limit {
            id: 0,
            side: Side::Bid,
            qty: 12,
            price: 395,
        },
        OrderAction::Limit {
            id: 1,
            side: Side::Ask,
            qty: 2,
            price: 399,
        },
        OrderAction::Limit {
            id: 2,
            side: Side::Bid,
            qty: 2,
            price: 398,
        },
    ]);
    let result = ob.event(OrderAction::Market {
        id: 3,
        side: Side::Ask,
        qty: 15,
    });

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
        OrderEvent::PartiallyFilled(
            14,
            vec![FillMetadata {
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
            }]
        )
    );
    assert_eq!(ob.min_ask, Some(399));
    assert_eq!(ob.max_bid, None);
    assert_eq!(ob.asks, init_book(vec![(399, 9998)]));
    assert_eq!(ob.bids, init_book_holes(vec![], vec![395, 398]));
    assert_eq!(ob.spread(), None);
}

#[test]
fn market_order_filled() {
    let (mut ob, results) = init_ob(vec![
        OrderAction::Limit {
            id: 0,
            side: Side::Bid,
            qty: 12,
            price: 395,
        },
        OrderAction::Limit {
            id: 1,
            side: Side::Ask,
            qty: 2,
            price: 399,
        },
        OrderAction::Limit {
            id: 2,
            side: Side::Bid,
            qty: 2,
            price: 398,
        },
    ]);
    let result = ob.event(OrderAction::Market {
        id: 3,
        side: Side::Ask,
        qty: 7,
    });

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
        OrderEvent::Filled(
            7,
            vec![FillMetadata {
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
            }]
        )
    );
    assert_eq!(ob.min_ask, Some(399));
    assert_eq!(ob.max_bid, Some(395));
    assert_eq!(ob.asks, init_book(vec![(399, 9998)]));
    assert_eq!(ob.bids, init_book_holes(vec![(395, 9999)], vec![398]));
    assert_eq!(ob.spread(), Some(4));
}

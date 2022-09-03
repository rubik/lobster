//! Lobster implements a single-threaded order book. To use Lobster, create an
//! order book instance with default parameters, and send orders for execution:
//!
//! ```rust
//! use lobster::{FillMetadata, OrderBook, OrderEvent, OrderType, Side};
//!
//! let mut ob = OrderBook::default();
//! let event = ob.execute(OrderType::Market { id: 0, qty: 1, side: Side::Bid });
//! assert_eq!(event, OrderEvent::Unfilled { id: 0 });
//!
//! let event = ob.execute(OrderType::Limit { id: 1, price: 120, qty: 3, side: Side::Ask });
//! assert_eq!(event, OrderEvent::Placed { id: 1 });
//!
//! let event = ob.execute(OrderType::Market { id: 2, qty: 4, side: Side::Bid });
//! assert_eq!(
//!     event,
//!     OrderEvent::PartiallyFilled {
//!         id: 2,
//!         filled_qty: 3,
//!         fills: vec![
//!             FillMetadata {
//!                 order_1: 2,
//!                 order_2: 1,
//!                 qty: 3,
//!                 price: 120,
//!                 taker_side: Side::Bid,
//!                 total_fill: true,
//!             }
//!         ],
//!     },
//! );
//! ```
//!
//! Lobster only deals in integer price points and quantities. Prices and
//! quantities are represented as unsigned 64-bit integers. If the traded
//! instrument supports fractional prices and quantities, the conversion needs to
//! be handled by the user. At this time, Lobster does not support negative prices.

#![warn(missing_docs, missing_debug_implementations, rustdoc::broken_intra_doc_links)]

mod arena;
mod models;
mod orderbook;

pub use models::{
    BookDepth, BookLevel, FillMetadata, OrderEvent, OrderType, Side, Trade,
};
pub use orderbook::OrderBook;

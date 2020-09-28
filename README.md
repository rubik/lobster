<div align="center">
  <img alt="Lobster logo" src="https://github.com/rubik/lobster/raw/master/images/logo.png" height="130" />
</div>

<div align="center">
  <h1>Lobster</h1>
  <p>A fast in-memory limit order book (LOB).</p>
  <a target="_blank" href="https://travis-ci.org/rubik/lobster">
    <img src="https://img.shields.io/travis/rubik/lobster?style=for-the-badge" alt="Build">
  </a>
  <a target="_blank" href="https://coveralls.io/github/rubik/lobster">
    <img src="https://img.shields.io/coveralls/github/rubik/lobster?style=for-the-badge" alt="Code Coverage">
  </a>
  <a target="_blank" href="https://crates.io/crates/lobster">
   <img src="https://img.shields.io/crates/d/lobster?style=for-the-badge" alt="Downloads (all time)">
  <a>
  <a href="https://github.com/rubik/lobster/blob/master/LICENSE">
    <img src="https://img.shields.io/crates/l/lobster?style=for-the-badge" alt="ISC License">
  </a>
  <br>
  <br>
</div>


# Quickstart
To use Lobster, create an order book instance with default parameters, and send
orders for execution:

```rust
let mut ob = OrderBook::default();
let event = ob.execute(OrderType::Market { id: 1, price: 120, qty: 3 });
```

Lobster only deals in integer price points and quantities. Prices and
quantities are represented as unsigned 64-bit integers. If the traded
instrument supports fractional prices and quantities, the conversion needs to
be handled by the user. At this time, Lobster does not support negative prices.

# Quantcup
The winning quantcup submission is at the moment about 11x faster than Lobster.
While Lobster can surely be improved significantly, some design choices
by necessity make it slower. Here's a non-exhaustive list:

1. The Quantcup solution holds all the price points in memory, whereas Lobster
  uses two BTreeMap structs. The performance boost of holding all the price
  points in a contiguous data structure on the stack is massive, but it's not
  very practical: the array is indexed by the price, so it can be huge
  (imagine implementing an order book for forex markets with integer price
  points at all non-fractional values); in reality limit orders can be made at
  any price, and in most markets there is no upper bound, so the static array
  solution is not viable.

2. The Quantcup solution does not update the max bid/min ask values when
   canceling orders. So if an order at the best bid/ask is canceled, that
   solution will not be correct. To be fair, this is pretty trivial to fix, but
   nonetheless it wasn't done in the benchmarked winning solution.

3. The Quantcup solution does not accept external order IDs; instead it
   provides them as integer indices from a static array. This has obvious
   practical consequences: the order book can handle a maximum number of open
   orders, and that number is chosen at compile time. Furthermore, if order IDs
   are not known to the sender before executing the order, it's difficult to
   broadcast the events to the right sender. Lobster supports unsigned 128-bit
   integers as order IDs, which can thus contain v4 UUIDs.

# Todo
1. Remove `OrderBook::update_min_ask` and `OrderBook::update_max_bid` and
   instead update the min ask/max bid values only when needed. Currently those
   methods are called on every order execution.
2. Experiment with replacing `BTreeMap`s with Trie from
   [`qp_trie`](https://github.com/sdleffler/qp-trie-rs).

<div>
  <small>
    Logo made by <a href="https://www.flaticon.com/authors/turkkub"
    title="turkkub">turkkub</a> from <a href="https://www.flaticon.com/"
    title="Flaticon">www.flaticon.com</a>.
  </small>
</div>

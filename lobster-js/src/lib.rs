extern crate console_error_panic_hook;

use wasm_bindgen::prelude::*;

// use gloo_utils::format::JsValueSerdeExt;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use lobster::{OrderBook, OrderType, Side};
use serde_wasm_bindgen::{ from_value, to_value };
use serde::{ Serialize, Serializer };


#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}


#[wasm_bindgen]
pub struct Lobster {
    orderbook: OrderBook
}



#[wasm_bindgen]
impl Lobster {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Lobster {
        Lobster {
            orderbook:  OrderBook::default()
        }
    }
    
    pub fn exec(&mut self, order: JsValue) -> JsValue {
        let order: OrderType = from_value(order).unwrap();
        let event = self.orderbook.execute(order);
        to_value(&event).unwrap()
    }
}
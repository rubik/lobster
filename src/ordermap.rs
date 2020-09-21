use std::collections::HashMap;
use std::ops::{Index, IndexMut};

use crate::models::LimitOrder;

#[derive(Debug)]
pub(crate) struct OrderMap {
    orders: Vec<LimitOrder>,
    free: Vec<usize>,
    order_map: HashMap<u128, usize>,
}

impl OrderMap {
    pub fn new(capacity: usize) -> Self {
        let mut list = Self {
            orders: Vec::with_capacity(capacity),
            free: Vec::with_capacity(capacity),
            order_map: HashMap::with_capacity(capacity),
        };

        // Preallocate
        for i in 0..capacity {
            list.orders.push(LimitOrder::new(0, 0, 0));
            list.free.push(i);
        }
        list
    }

    pub fn insert(&mut self, id: u128, price: u64, qty: u64) -> usize {
        match self.free.pop() {
            None => {
                self.orders.push(LimitOrder::new(id, price, qty));
                let index = self.orders.len() - 1;
                self.order_map.insert(id, index);
                index
            }
            Some(index) => {
                let ord = &mut self.orders[index];
                ord.id = id;
                ord.qty = qty;
                ord.price = price;
                self.order_map.insert(id, index);
                index
            }
        }
    }

    pub fn delete(&mut self, id: &u128) -> bool {
        if let Some(idx) = self.order_map.remove(id) {
            if let Some(mut ord) = self.orders.get_mut(idx) {
                self.free.push(idx);
                ord.qty = 0;
                return true;
            }
        }
        false
    }
}

impl Index<usize> for OrderMap {
    type Output = LimitOrder;

    #[inline]
    fn index(&self, index: usize) -> &LimitOrder {
        &self.orders[index]
    }
}

impl IndexMut<usize> for OrderMap {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut LimitOrder {
        &mut self.orders[index]
    }
}

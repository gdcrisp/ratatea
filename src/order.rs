use crate::order_item::OrderItem;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Order {
    pub items: Vec<OrderItem>,
    pub name: String,
}
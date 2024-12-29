use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum OrderItem {
    ClassicMilkTea,
    TaroMilkTea,
    MatchaMilkTea,
    ThaiMilkTea,
    Espresso,
    Latte,
}

impl OrderItem {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ClassicMilkTea" => Some(OrderItem::ClassicMilkTea),
            "TaroMilkTea" => Some(OrderItem::TaroMilkTea),
            "MatchaMilkTea" => Some(OrderItem::MatchaMilkTea),
            "ThaiMilkTea" => Some(OrderItem::ThaiMilkTea),
            "Espresso" => Some(OrderItem::Espresso),
            "Latte" => Some(OrderItem::Latte),
            _ => None,
        }
    }
}
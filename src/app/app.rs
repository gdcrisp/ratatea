use crate::order::Order;
use crate::order_item::OrderItem;
use rusqlite::{params, Connection};
use std::error::Error;

pub struct App {
    pub db: Connection,
    pub options: Vec<OrderItem>,
    pub cart: Vec<OrderItem>,
    pub orders: Vec<Order>,
    pub cursor: usize,
    pub page: usize,
    pub gradient_index: usize,
    pub magnify_index: usize,
    pub input: String,
    pub selection_effect: Option<(usize, usize)>,
    pub users: Vec<String>,
    pub selected_user: Option<String>,
}

impl App {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let db = Connection::open("bubble_tea.db")?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS options (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS cart (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS orders (
                id INTEGER PRIMARY KEY,
                items TEXT NOT NULL,
                name TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            );",
        )?;
        db.execute(
            "ALTER TABLE orders ADD COLUMN name TEXT",
            [],
        ).ok();

        let users = {
            let mut stmt = db.prepare("SELECT name FROM users")?;
            let users_iter = stmt.query_map([], |row| {
                let name: String = row.get(0)?;
                Ok(name)
            })?;
            users_iter.collect::<Result<Vec<String>, _>>()?
        };

        Ok(Self {
            db,
            options: vec![
                OrderItem::ClassicMilkTea,
                OrderItem::TaroMilkTea,
                OrderItem::MatchaMilkTea,
                OrderItem::ThaiMilkTea,
                OrderItem::Espresso,
                OrderItem::Latte,
            ],
            cart: vec![],
            orders: vec![],
            cursor: 0,
            page: 0,
            gradient_index: 0,
            magnify_index: 0,
            input: String::new(),
            selection_effect: None,
            users,
            selected_user: None,
        })
    }

    pub fn load_data(&mut self) -> Result<(), Box<dyn Error>> {
        let mut stmt = self.db.prepare("SELECT items, name FROM orders")?;
        let orders_iter = stmt.query_map([], |row| {
            let items: String = row.get(0)?;
            let name: String = row.get(1)?;
            let items: Vec<OrderItem> = serde_json::from_str(&items)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok(Order { items, name })
        })?;
        self.orders = orders_iter.collect::<Result<Vec<Order>, _>>()?;

        Ok(())
    }

    pub fn current_list(&self) -> Vec<String> {
        match self.page {
            0 => self.users.clone(),
            2 => self.options.iter().map(|item| format!("{:?}", item)).collect(),
            3 => self.cart.iter().map(|item| format!("{:?}", item)).collect(),
            4 => self.orders.iter().map(|order| {
                format!("{}: {}", order.name, order.items.iter().map(|item| format!("{:?}", item)).collect::<Vec<_>>().join(", "))
            }).collect::<Vec<_>>(),
            _ => vec![],
        }
    }
}
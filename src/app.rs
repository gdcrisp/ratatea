use crate::order::Order;
use crate::order_item::OrderItem;
use crate::utils::hsv_to_rgb;
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use rusqlite::{params, Connection};
use serde_json;
use std::error::Error;
use std::io::Stdout;

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

    pub fn next_item(&mut self) {
        match self.current_list().len() {
            0 => {}
            len => {
                self.cursor = (self.cursor + 1) % len;
                self.selection_effect = None;
            }
        }
    }

    pub fn prev_item(&mut self) {
        match self.current_list().len() {
            0 => {}
            len => {
                self.cursor = if self.cursor == 0 {
                    len - 1
                } else {
                    self.cursor - 1
                };
                self.selection_effect = None;
            }
        }
    }

    pub fn select_item(&mut self) -> Result<(), Box<dyn Error>> {
        if self.page == 2 {
            if let Some(item) = self.options.get(self.cursor) {
                self.cart.push(item.clone());
                self.db.execute(
                    "INSERT INTO cart (name) VALUES (?1)",
                    params![serde_json::to_string(item)?],
                )?;
                self.selection_effect = Some((self.cursor, 20));
            }
        } else if self.page == 3 {
            if !self.cart.is_empty() {
                let item = self.cart.remove(self.cursor);
                self.db.execute(
                    "DELETE FROM cart WHERE name = ?1",
                    params![serde_json::to_string(&item)?],
                )?;
                if self.cursor >= self.cart.len() {
                    self.cursor = self.cart.len().saturating_sub(1);
                }
                self.selection_effect = Some((self.cursor, 20));
            }
        }
        Ok(())
    }

    pub fn next_page(&mut self) {
        self.page = match self.page {
            0 => 1,
            1 => 2,
            2 => 3,
            3 => 4,
            4 => 0,
            _ => 0,
        };
        self.cursor = 0;
        if self.page == 1 {
            self.input.clear();
        }
    }

 pub fn add_user(&mut self) {
    let user = self.input.trim().to_string();
    if !user.is_empty() && !self.users.contains(&user) {
        self.users.push(user.clone());
        self.db.execute(
            "INSERT INTO users (name) VALUES (?1)",
            params![user],
        ).unwrap();
        self.input.clear();
    }
    }

    pub fn add_order(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.cart.is_empty() && self.selected_user.is_some() {
            let items_json = serde_json::to_string(&self.cart)?;
            self.db.execute(
                "INSERT INTO orders (items, name) VALUES (?1, ?2)",
                params![items_json, self.selected_user.clone().unwrap()],
            )?;
            self.cart.clear();
            self.db.execute("DELETE FROM cart", [])?;
        } else {
            println!("Please select a user before placing an order.");
        }
        Ok(())
    }

    pub fn remove_order(&mut self) -> Result<(), Box<dyn Error>> {
        if self.page == 4 && !self.orders.is_empty() {
            let order = self.orders.remove(self.cursor);
            let items_json = serde_json::to_string(&order.items)?;
            self.db
                .execute("DELETE FROM orders WHERE items = ?1", params![items_json])?;
            if self.cursor >= self.orders.len() {
                self.cursor = self.orders.len().saturating_sub(1);
            }
        }
        Ok(())
    }

    pub fn current_list(&self) -> Vec<String> {
        match self.page {
            0 => self.users.clone(),
            2 => self.options.iter().map(|item| format!("{:?}", item)).collect(), // Menu page
            3 => self.cart.iter().map(|item| format!("{:?}", item)).collect(), // Cart page
            4 => self.orders.iter().map(|order| {
                format!("{}: {}", order.name, order.items.iter().map(|item| format!("{:?}", item)).collect::<Vec<_>>().join(", "))
            }).collect::<Vec<_>>(),
            _ => vec![],
        }
    }
    pub fn render_user_list(&self, f: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
        let items = self.map_items(&self.users);
        let content = Paragraph::new(items).block(Block::default().borders(Borders::ALL));
        f.render_widget(content, area);
    }

    pub fn select_user(&mut self) {
        if let Some(user) = self.users.get(self.cursor) {
            self.selected_user = Some(user.clone());
            self.page = 2;
        }
    }
    pub fn update_gradient(&mut self) {
        self.gradient_index = (self.gradient_index + 1) % 360;
        if self.gradient_index % 4 == 0 {
            self.magnify_index = (self.magnify_index + 1)
                % self.current_list().get(self.cursor).map_or(1, |s| s.len());
        }
    }

    pub fn render_gradient_text(&self, text: &str, index: usize) -> Vec<Span> {
        text.chars()
            .enumerate()
            .map(|(i, c)| {
                let hue = ((index + i * 3) % 360) as f64;
                let (r, g, b) = hsv_to_rgb(hue, 0.6, 0.8);
                let style = match i {
                    _ if i == self.magnify_index => Style::default()
                        .fg(Color::Rgb(r, g, b))
                        .add_modifier(Modifier::BOLD),
                    _ if i == (self.magnify_index + 1) % text.len()
                        || i == (self.magnify_index + text.len() - 1) % text.len() =>
                    {
                        Style::default()
                            .fg(Color::Rgb(r, g, b))
                            .add_modifier(Modifier::ITALIC)
                    }
                    _ => Style::default().fg(Color::Rgb(r, g, b)),
                };
                Span::styled(c.to_string(), style)
            })
            .collect()
    }

    pub fn render_selection_effect(
        &self,
        f: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        text: &str,
    ) {
        let border: Vec<Span> = "-"
            .repeat(text.len() + 4)
            .chars()
            .enumerate()
            .map(|(i, c)| {
                let hue = ((self.gradient_index + i * 3) % 360) as f64;
                let (r, g, b) = hsv_to_rgb(hue, 0.6, 0.8);
                Span::styled(
                    c.to_string(),
                    Style::default()
                        .fg(Color::Rgb(r, g, b))
                        .add_modifier(Modifier::BOLD),
                )
            })
            .collect();

        let border_spans = Spans::from(border);
        let content = Paragraph::new(vec![
            border_spans.clone(),
            Spans::from(Span::styled(
                format!("| {} |", text),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            border_spans,
        ])
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(content, area);
    }
    fn map_items(&self, items: &[String]) -> Vec<Spans> {
        items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if i == self.cursor {
                    Spans::from(self.render_gradient_text(item, self.gradient_index))
                } else {
                    Spans::from(Span::styled(
                        item.clone(),
                        Style::default().fg(Color::Magenta),
                    ))
                }
            })
            .collect()
    }
    pub fn render_list(&self, f: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
        let items = self.map_items(&self.current_list());
        let content = Paragraph::new(items).block(Block::default().borders(Borders::ALL));
        f.render_widget(content, area);

        if let Some((effect_index, _)) = self.selection_effect {
            if effect_index == self.cursor && !(self.page == 3 && self.current_list().len() < 1) {
                self.render_selection_effect(f, area, &self.current_list()[effect_index]);
            }
        }
    }
}

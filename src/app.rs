use crate::order_item::OrderItem;
use crate::order::Order;
use crate::utils::hsv_to_rgb;
use rusqlite::{params, Connection};
use serde_json;
use std::error::Error;
use ratatui::{layout::Rect, text::{Span, Spans}, widgets::{Block, Borders, Paragraph}, Frame, style::{Color, Modifier, Style}, backend::CrosstermBackend};
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
                items TEXT NOT NULL
            );",
        )?;
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
        })
    }

    pub fn load_data(&mut self) -> Result<(), Box<dyn Error>> {
        let mut stmt = self.db.prepare("SELECT name FROM cart")?;
        let cart_iter = stmt.query_map([], |row| {
            let name: String = row.get(0)?;
            let item: OrderItem = serde_json::from_str(&name)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok(item)
        })?;
        self.cart = cart_iter.collect::<Result<Vec<OrderItem>, _>>()?;

        let mut stmt = self.db.prepare("SELECT items FROM orders")?;
        let orders_iter = stmt.query_map([], |row| {
            let items: String = row.get(0)?;
            let items: Vec<OrderItem> = serde_json::from_str(&items)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            Ok(Order { items })
        })?;
        self.orders = orders_iter.collect::<Result<Vec<Order>, _>>()?;

        Ok(())
    }

    pub fn next_item(&mut self) {
        match self.current_list().len() {
            0 => {}
            len => {
                self.cursor = (self.cursor + 1) % len;
                self.selection_effect = None; // Clear selection effect
            }
        }
    }

    pub fn prev_item(&mut self) {
        match self.current_list().len() {
            0 => {}
            len => {
                self.cursor = if self.cursor == 0 { len - 1 } else { self.cursor - 1 };
                self.selection_effect = None;
            }
        }
    }

    pub fn select_item(&mut self) -> Result<(), Box<dyn Error>> {
        if self.page == 0 {
            if let Some(item) = self.options.get(self.cursor) {
                self.cart.push(item.clone());
                self.db.execute(
                    "INSERT INTO cart (name) VALUES (?1)",
                    params![serde_json::to_string(item)?],
                )?;
                self.selection_effect = Some((self.cursor, 20));
            }
        } else if self.page == 1 {
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
        self.page = (self.page + 1) % 4;
        self.cursor = 0;
        if self.page == 2 {
            self.input.clear();
        }
    }

    pub fn add_item_to_menu(&mut self) {
        if let Some(item) = OrderItem::from_str(&self.input.trim()) {
            self.options.push(item);
            self.input.clear();
        }
    }

    pub fn add_order(&mut self) -> Result<(), Box<dyn Error>> {
        if !self.cart.is_empty() {
            let items_json = serde_json::to_string(&self.cart)?;
            self.db.execute(
                "INSERT INTO orders (items) VALUES (?1)",
                params![items_json],
            )?;
            self.cart.clear();
            self.db.execute("DELETE FROM cart", [])?;
        }
        Ok(())
    }

    pub fn remove_order(&mut self) -> Result<(), Box<dyn Error>> {
        if self.page == 3 && !self.orders.is_empty() {
            let order = self.orders.remove(self.cursor);
            let items_json = serde_json::to_string(&order.items)?;
            self.db.execute(
                "DELETE FROM orders WHERE items = ?1",
                params![items_json],
            )?;
            if self.cursor >= self.orders.len() {
                self.cursor = self.orders.len().saturating_sub(1);
            }
        }
        Ok(())
    }

    pub fn current_list(&self) -> Vec<String> {
        match self.page {
            0 => self.options.iter().map(|item| format!("{:?}", item)).collect(),
            1 => self.cart.iter().map(|item| format!("{:?}", item)).collect(),
            3 => self.orders.iter().map(|order| order.items.iter().map(|item| format!("{:?}", item)).collect::<Vec<_>>().join(", ")).collect(),
            _ => vec![],
        }
    }

    pub fn update_gradient(&mut self) {
        self.gradient_index = (self.gradient_index + 1) % 360;
        if self.gradient_index % 4 == 0 { // Slow down the magnify effect
            self.magnify_index = (self.magnify_index + 1) % self.current_list().get(self.cursor).map_or(1, |s| s.len());
        }
    }

    pub fn render_gradient_text(&self, text: &str, index: usize) -> Vec<Span> {
        text.chars()
            .enumerate()
            .map(|(i, c)| {
                let hue = ((index + i * 3) % 360) as f64;
                let (r, g, b) = hsv_to_rgb(hue, 0.6, 0.8);
                let style = if i == self.magnify_index {
                    Style::default()
                        .fg(Color::Rgb(r, g, b))
                        .add_modifier(Modifier::BOLD)
                } else if i == (self.magnify_index + 1) % text.len() || i == (self.magnify_index + text.len() - 1) % text.len() {
                    Style::default()
                        .fg(Color::Rgb(r, g, b))
                        .add_modifier(Modifier::ITALIC)
                } else {
                    Style::default()
                        .fg(Color::Rgb(r, g, b))
                };
                Span::styled(c.to_string(), style)
            })
            .collect()
    }

    pub fn render_selection_effect(&self, f: &mut Frame<CrosstermBackend<Stdout>>, area: Rect, text: &str) {
        let border: Vec<Span> = "-".repeat(text.len() + 4)
            .chars()
            .enumerate()
            .map(|(i, c)| {
                let hue = ((self.gradient_index + i * 3) % 360) as f64;
                let (r, g, b) = hsv_to_rgb(hue, 0.6, 0.8);
                Span::styled(
                    c.to_string(),
                    Style::default().fg(Color::Rgb(r, g, b)).add_modifier(Modifier::BOLD),
                )
            })
            .collect();

        let border_spans = Spans::from(border);
        let content = Paragraph::new(vec![
            border_spans.clone(),
            Spans::from(Span::styled(
                format!("| {} |", text),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            )),
            border_spans,
        ])
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(content, area);
    }

    pub fn render_list(&self, f: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
        let items: Vec<Spans> = self
            .current_list()
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if let Some((effect_index, _)) = self.selection_effect {
                    if i == effect_index {
                        return Spans::from("");
                    }
                }
                if i == self.cursor {
                    Spans::from(self.render_gradient_text(item, self.gradient_index))
                } else {
                    Spans::from(Span::styled(
                        item.clone(),
                        Style::default().fg(Color::Magenta),
                    ))
                }
            })
            .collect();

        let mut list_area = area;
        if let Some((effect_index, _)) = self.selection_effect {
            if effect_index < self.cursor {
                list_area.height += 10;
            }
        }

        let content = Paragraph::new(items).block(Block::default().borders(Borders::ALL));
        f.render_widget(content, list_area);

        if let Some((effect_index, _)) = self.selection_effect {
            if effect_index == self.cursor {
                self.render_selection_effect(f, area, &self.current_list()[effect_index]);
            }
        }
    }
}
use crate::io::Stdout;
use ratatui::layout::Rect;
use std::{io, thread, time::Duration};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::{Constraint, Direction, Layout}, style::{Color, Modifier, Style}, text::{Span, Spans}, widgets::{Block, Borders, Paragraph}, Frame, Terminal};
use rusqlite::{params, Connection};
use serde_json;
use std::error::Error;
use serde::{Deserialize, Serialize};

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (u8, u8, u8) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match h as u64 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    (
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}

#[derive(Clone, Serialize, Deserialize, Debug)]
enum OrderItem {
    ClassicMilkTea,
    TaroMilkTea,
    MatchaMilkTea,
    ThaiMilkTea,
    Espresso,
    Latte,
}

impl OrderItem {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "ClassicMilkTea" => Some(OrderItem::ClassicMilkTea),
            "TaroMilkTea" => Some(OrderItem::TaroMilkTea),
            "MatchaMilkTea" => Some(OrderItem::MatchaMilkTea),
            "ThaiMilkTea" => Some(OrderItem::ThaiMilkTea),
            "Espresso" => Some(OrderItem::Espresso),
            "Latte"=> Some(OrderItem::Latte),
            _ => None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Order {
    items: Vec<OrderItem>,
}
struct App {
    db: Connection,
    options: Vec<OrderItem>,
    cart: Vec<OrderItem>,
    orders: Vec<Order>,
    cursor: usize,
    page: usize,
    gradient_index: usize,
    magnify_index: usize,
    input: String,
    selection_effect: Option<(usize, usize)>,
}

impl App {
    fn new() -> Result<Self, Box<dyn Error>> {
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
    fn next_item(&mut self) {
        match self.current_list().len() {
            0 => {} // Do nothing if the list is empty
            len => {
                self.cursor = (self.cursor + 1) % len;
                self.selection_effect = None; // Clear selection effect
            }
        }
    }

    fn load_data(&mut self) -> Result<(), Box<dyn Error>> {
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

    fn prev_item(&mut self) {
        match self.current_list().len() {
            0 => {} // Do nothing if the list is empty
            len => {
                self.cursor = if self.cursor == 0 { len - 1 } else { self.cursor - 1 };
                self.selection_effect = None; // Clear selection effect
            }
        }
    }

    fn select_item(&mut self) -> Result<(), Box<dyn Error>> {
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

    fn next_page(&mut self) {
        self.page = (self.page + 1) % 4; // Updated to include the orders page
        self.cursor = 0; // Reset cursor for the new page
        if self.page == 2 {
            self.input.clear(); // Clear input on the Add New Item page
        }
    }

    fn add_item_to_menu(&mut self) {
        if let Some(item) = OrderItem::from_str(&self.input.trim()) {
            self.options.push(item);
            self.input.clear();
        }
    }

    fn add_order(&mut self) -> Result<(), Box<dyn Error>> {
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
    fn remove_order(&mut self) -> Result<(), Box<dyn Error>> {
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
    fn current_list(&self) -> Vec<String> {
        match self.page {
            0 => self.options.iter().map(|item| format!("{:?}", item)).collect(),
            1 => self.cart.iter().map(|item| format!("{:?}", item)).collect(),
            3 => self.orders.iter().map(|order| order.items.iter().map(|item| format!("{:?}", item)).collect::<Vec<_>>().join(", ")).collect(),
            _ => vec![],
        }
    }

    fn update_gradient(&mut self) {
        self.gradient_index = (self.gradient_index + 1) % 360;
        if self.gradient_index % 4 == 0 { // Slow down the magnify effect
            self.magnify_index = (self.magnify_index + 1) % self.current_list().get(self.cursor).map_or(1, |s| s.len());
        }
    }

    fn render_gradient_text(&self, text: &str, index: usize) -> Vec<Span> {
        text.chars()
            .enumerate()
            .map(|(i, c)| {
                let hue = ((index + i * 3) % 360) as f64;
                let (r, g, b) = hsv_to_rgb(hue, 0.6, 0.8);
                let style = if i == self.magnify_index {
                    Style::default()
                        .fg(Color::Rgb(r, g, b))
                        .add_modifier(Modifier::BOLD) // Bold effect
                } else if i == (self.magnify_index + 1) % text.len() || i == (self.magnify_index + text.len() - 1) % text.len() {
                    Style::default()
                        .fg(Color::Rgb(r, g, b))
                        .add_modifier(Modifier::ITALIC) // Semi-bold effect
                } else {
                    Style::default()
                        .fg(Color::Rgb(r, g, b))
                };
                Span::styled(c.to_string(), style)
            })
            .collect()
    }
    fn render_selection_effect(&self, f: &mut Frame<CrosstermBackend<Stdout>>, area: Rect, text: &str) {
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

    fn render_list(&self, f: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
        let items: Vec<Spans> = self
            .current_list()
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if let Some((effect_index, _)) = self.selection_effect {
                    if i == effect_index {
                        return Spans::from(""); // Leave space for the selection effect
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

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new()?;
    app.load_data()?;

    loop {
        terminal.draw(|f| {
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(100)].as_ref())
                .split(size);

            match app.page {
                0 => {
                    let title = Paragraph::new("Menu")
                        .block(Block::default().borders(Borders::ALL));
                    f.render_widget(title, chunks[0]);
                    app.render_list(f, chunks[0]);
                }
                1 => {
                    let title = Paragraph::new("Cart")
                        .block(Block::default().borders(Borders::ALL));
                    f.render_widget(title, chunks[0]);
                    app.render_list(f, chunks[0]);
                }
                2 => {
                    let input = Paragraph::new(app.input.clone())
                        .block(Block::default().borders(Borders::ALL).title("Add New Item"));
                    f.render_widget(input, chunks[0]);
                }
                3 => {
                    let title = Paragraph::new("Orders")
                        .block(Block::default().borders(Borders::ALL));
                    f.render_widget(title, chunks[0]);
                    app.render_list(f, chunks[0]);
                }
                _ => {}
            }
        })?;

        if event::poll(Duration::from_millis(43))? {
            if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                match code {
                    KeyCode::Char('q') => break,
                    KeyCode::Up if app.page != 2 => app.prev_item(),
                    KeyCode::Down if app.page != 2 => app.next_item(),
                    KeyCode::Enter => {
                        if app.page == 0 || app.page == 1 {
                            app.select_item()?;
                        } else if app.page == 2 {
                            app.add_item_to_menu();
                        }
                    }
                    KeyCode::Tab => app.next_page(),
                    KeyCode::Char(c) if app.page == 2 => app.input.push(c),
                    KeyCode::Backspace if app.page == 2 => {
                        app.input.pop();
                    }
                    KeyCode::Esc if app.page == 1 => {
                        app.add_order()?; // Save the cart as a new order in the database
                        app.load_data()?; // Reload data to reflect the updated orders
                    }
                    KeyCode::Char(d) if app.page == 3 => {
                        app.remove_order()?; // Remove the selected order
                        app.load_data()?; // Reload data to reflect the updated orders
                    }
                    _ => {}
                }
            }
        }

        if let Some((_, remaining_frames)) = &mut app.selection_effect {
            if *remaining_frames > 0 {
                *remaining_frames -= 1;
            } else {
                app.selection_effect = None;
            }
        }

        app.update_gradient();
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    Ok(())
}
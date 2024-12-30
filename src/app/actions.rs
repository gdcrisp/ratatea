use crate::app::App;
use rusqlite::params;
use std::error::Error;

impl App {
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
}
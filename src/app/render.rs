use crate::app::App;
use crate::utils::hsv_to_rgb;
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::io::Stdout;

impl App {
    pub fn render_user_list(&self, f: &mut Frame<CrosstermBackend<Stdout>>, area: Rect) {
        let items = self.map_items(&self.users);
        let content = Paragraph::new(items).block(Block::default().borders(Borders::ALL));
        f.render_widget(content, area);
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

    pub fn render_selection_effect(
        &self,
        f: &mut Frame<CrosstermBackend<Stdout>>,
        area: Rect,
        text: &str,
    ) {
        let border: Vec<Span> = "-".repeat(text.len() + 4)
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
}
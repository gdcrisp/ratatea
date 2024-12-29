mod app;
mod order_item;
mod order;
mod utils;

use crate::app::App;
use crate::utils::hsv_to_rgb;
use std::{io, thread, time::Duration};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, layout::{Constraint, Direction, Layout}, style::{Color, Modifier, Style}, text::{Span, Spans}, widgets::{Block, Borders, Paragraph}, Frame, Terminal};
use std::error::Error;

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
                        app.add_order()?;
                        app.load_data()?;
                    }
                    KeyCode::Char(d) if app.page == 3 => {
                        app.remove_order()?;
                        app.load_data()?;
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
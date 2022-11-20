use super::game;

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io,
    time::{Duration, Instant},
};
use tui::{
    backend::CrosstermBackend,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Block, Borders, Widget},
    Terminal,
};

/// A widget to render a [Game]
pub struct LevelWidget<'a> {
    block: Block<'a>,
    game: &'a game::Game,
}

impl<'a> LevelWidget<'a> {
    pub fn new(game: &'a game::Game) -> Self {
        let block = Block::default().title("Tetris").borders(Borders::ALL);
        LevelWidget { block, game }
    }

    /// Render the game level into a [Buffer], this is a helper function to
    /// implement [Widget] trait.
    fn render_to_buffer(self) -> Buffer {
        let display = self.game.render();
        let d_height = display.rows as u16;
        let d_width = display.columns as u16;

        let mut buf = Buffer::empty(Rect::new(0, 0, d_width * 2, d_height));

        for r in 0..display.rows {
            for c in 0..display.columns {
                if display[(r, c)] {
                    let x = (c * 2) as u16;
                    let y = (display.rows - r - 1) as u16;
                    buf.get_mut(x, y).set_symbol(symbols::block::FULL);
                    buf.get_mut(x + 1, y).set_symbol(symbols::block::FULL);
                }
            }
        }

        let mut tooltip: Option<Span> = None;
        match self.game.state {
            game::State::End => {
                tooltip = Some(Span::styled(
                    "GAME OVER",
                    Style::default()
                        .fg(Color::Red)
                        .add_modifier(Modifier::RAPID_BLINK),
                ));
            }
            game::State::Paused => {
                tooltip = Some(Span::styled(
                    "Paused",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::RAPID_BLINK),
                ));
            }
            _ => {}
        }

        if tooltip.is_some() {
            let s = tooltip.as_ref().unwrap();
            let s_len = s.content.len() as u16;
            buf.set_span(
                d_width.checked_sub(s_len / 2).unwrap_or(0),
                d_height / 2,
                s,
                s_len,
            );
        }
        buf
    }

    /// Return the expected area of this widget. Note that `(x,y)` is always
    /// set to `(0,0)`, only `width` and `height` are meaningful.
    pub fn expected_area(&self) -> Rect {
        let width = (self.game.level.columns * 2 + 2) as u16;
        let height = (self.game.level.rows + 2) as u16;
        Rect {
            x: 0,
            y: 0,
            width,
            height,
        }
    }
}

impl<'a> Widget for LevelWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let b = self.block.clone();
        let level_area = b.inner(area);
        b.render(area, buf);

        let mut level_buf = self.render_to_buffer();
        if level_buf.area.height > level_area.height || level_buf.area.width > level_area.width {
            buf.set_string(
                level_area.left(),
                level_area.bottom() - (level_area.height / 2),
                "Not enough display space",
                Style::default(),
            );
            return;
        }

        // put level_buf in the top-center of buf
        let center = (level_area.left() + level_area.right()) / 2;
        let new_x = center
            .checked_sub(level_buf.area.width)
            .unwrap_or(level_area.left());
        level_buf.resize(Rect {
            x: new_x,
            y: level_area.top(),
            ..level_buf.area
        });
        buf.merge(&level_buf);
    }
}

/// Start the game.
pub fn start() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let game_size: (u16, u16) = (16, 22);

    let mut g = game::Game::new((game_size.1 as usize, game_size.0 as usize));
    g.handle_event(game::Event::Start);

    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(200);
    loop {
        term.draw(|f| {
            let size = f.size();
            let level = LevelWidget::new(&g);
            let expected_area = level.expected_area();
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Length(expected_area.width)].as_ref())
                .split(size);

            f.render_widget(
                level,
                Rect {
                    width: expected_area.width,
                    height: expected_area.height,
                    ..chunks[0]
                },
            );
        })?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Down => {
                        for _ in 0..5 {
                            g.tick();
                        }
                    }
                    KeyCode::Left => {
                        g.handle_event(game::Event::Left);
                    }
                    KeyCode::Right => {
                        g.handle_event(game::Event::Right);
                    }
                    KeyCode::Up => {
                        g.handle_event(game::Event::Rotate);
                    }
                    KeyCode::Char('p') => {
                        if g.state == game::State::Paused {
                            g.handle_event(game::Event::Start);
                        } else {
                            g.handle_event(game::Event::Pause);
                        }
                    }
                    KeyCode::Char('q') => break,
                    _ => {}
                }
            }
        }

        while last_tick.elapsed() >= tick_rate {
            g.tick();
            last_tick += tick_rate;
        }
    }

    disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.show_cursor()?;

    Ok(())
}

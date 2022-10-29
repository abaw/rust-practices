#[macro_use]
extern crate matrix;

use matrix::prelude::*;

#[allow(dead_code)]
fn print_matrix(m: &Conventional<bool>) {
    println!("{:=<20}", "");
    for r in (0..m.rows).rev() {
        for c in 0..m.columns {
            if m[(r, c)] {
                print!("X");
            } else {
                print!(" ");
            }
        }
        println!("");
    }
    println!("{:=<20}", "");
}

/// A Shape is a piece you could control in a Tetris level. A true element means
/// there is a cell in that position. You could move rotate it in a
/// Tetris level.
#[derive(Debug, Clone)]
struct Shape(Conventional<bool>);

impl Shape {
    fn new(matrix: Conventional<bool>) -> Self {
        Shape(matrix)
    }

    /// Return the width of this shape
    fn width(&self) -> usize {
        self.0.columns
    }

    /// Return the height of this shape
    fn height(&self) -> usize {
        self.0.rows
    }

    fn cells(&self) -> &Conventional<bool> {
        &self.0
    }

    /// Rotate the shape clock-wise by 90°.
    fn rotate(&mut self) {
        let mut new = Conventional::<bool>::new((self.width(), self.height()));
        for row in 0..new.rows {
            for col in 0..new.columns {
                new[(row, col)] = self.0[(col, new.rows - row - 1)];
            }
        }
        self.0 = new;
    }
}

mod game {
    use super::Shape;
    use matrix::prelude::*;
    use rand::prelude::*;
    use std::collections::VecDeque;
    use std::convert::identity;

    /// The state of the current game
    #[derive(PartialEq, Eq, Debug)]
    pub enum State {
        Init,
        Playing,
        Paused,
        End,
    }

    /// The event that could happen in a game
    pub enum Event {
        Start,
        Left,
        Right,
        Rotate,
        Pause,
    }

    pub struct ShapesFactory {
        shapes: Vec<Shape>,
    }

    impl ShapesFactory {
        pub fn new() -> Self {
            let shapes = vec![
                // square
                Shape::new(Conventional::from_vec(
                    (2, 2),
                    matrix![
                        true, true;
                        true, true;
                    ],
                )),
                // stick
                Shape::new(Conventional::from_vec(
                    (4, 1),
                    matrix![
                        true;
                        true;
                        true;
                        true;
                    ],
                )),
                // J
                Shape::new(Conventional::from_vec(
                    (2, 3),
                    matrix![
                        true, true, true;
                        true, false, false;
                    ],
                )),
                // L
                Shape::new(Conventional::from_vec(
                    (2, 3),
                    matrix![
                        true, true, true;
                        false, false, true;
                    ],
                )),
                // S
                Shape::new(Conventional::from_vec(
                    (2, 3),
                    matrix![
                        true, true, false;
                        false, true, true;
                    ],
                )),
                // Z
                Shape::new(Conventional::from_vec(
                    (2, 3),
                    matrix![
                        false, true, true;
                        true, true, false;
                    ],
                )),
                // T
                Shape::new(Conventional::from_vec(
                    (2, 3),
                    matrix![
                        true, true, true;
                        false, true, false;
                    ],
                )),
            ];

            ShapesFactory { shapes }
        }

        fn create_shape(&self) -> Shape {
            let sel = thread_rng().gen_range(0..self.shapes.len());
            self.shapes[sel].clone()
        }
    }

    #[derive(Debug, Clone)]
    struct ShapeInLevel {
        /// The shape
        shape: Shape,
        /// The position in the level. Note the position indicates where the
        /// bottom-left corner of the shape is in the level.
        pos: (isize, isize),
    }

    /// A game represents a game
    pub struct Game {
        shape: Option<ShapeInLevel>,

        pub state: State,
        /// What state the game is currently in.

        /// This matrix represents the cells in a level.
        pub level: Conventional<bool>,

        /// This is used to create shapes
        shapes_factory: ShapesFactory,
    }

    impl Game {
        /// Return a new Game with the given height and width.
        pub fn new(size: (usize, usize)) -> Game {
            Game {
                shape: None,
                state: State::Init,
                level: Conventional::new(size),
                shapes_factory: ShapesFactory::new(),
            }
        }

        /// Handle a game event, it returns false if we should quit the game.
        pub fn handle_event(&mut self, e: Event) -> bool {
            match e {
                Event::Start => match self.state {
                    State::Init | State::End => {
                        self.reset();
                        true
                    }
                    State::Paused => {
                        self.state = State::Playing;
                        true
                    }
                    _ => true,
                },
                Event::Left => {
                    if self.state != State::Playing {
                        return true;
                    }

                    self.move_shape((0, -1));
                    true
                }
                Event::Right => {
                    if self.state != State::Playing {
                        return true;
                    }

                    self.move_shape((0, 1));
                    true
                }
                Event::Pause => {
                    if self.state == State::Playing {
                        self.state = State::Paused;
                    }
                    true
                }
                Event::Rotate => {
                    if self.state != State::Playing {
                        return true;
                    }

                    let s = self.shape.as_ref().unwrap();
                    let mut new_s = s.clone();
                    new_s.shape.rotate();
                    if !self.check_shape_out_of_bound(Some(&new_s))
                        && !self.check_collision(Some(&new_s))
                    {
                        self.shape = Some(new_s);
                    }
                    true
                }
            }
        }

        /// Do one tick.
        pub fn tick(&mut self) {
            if self.state != State::Playing {
                return;
            }

            let dropped = self.drop_shape();
            if dropped {
                return;
            }

            self.eliminate_rows();
            self.create_new_shape();
            if self.check_shape_out_of_bound(None) || self.check_collision(None) {
                self.state = State::End;
            }
        }

        /// drop the shape by single row, return false if the shape could not be
        /// dropped any more.
        fn drop_shape(&mut self) -> bool {
            if self.move_shape((-1, 0)) {
                return true;
            }

            let s = self.shape.take().unwrap();
            let s_width = s.shape.width() as isize;
            let s_height = s.shape.height() as isize;

            for hi in 0..s_height {
                for wi in 0..s_width {
                    let s_pos = (hi as usize, wi as usize);
                    let l_pos = ((s.pos.0 + hi) as usize, (s.pos.1 + wi) as usize);
                    if s.shape.cells()[s_pos] {
                        self.level[l_pos] = true;
                    }
                }
            }
            false
        }

        fn eliminate_rows(&mut self) -> bool {
            let mut rows_to_eliminate = VecDeque::<usize>::new();
            for row in 0..self.level.rows {
                if (0..self.level.columns)
                    .map(|col| self.level[(row, col)])
                    .all(identity)
                {
                    rows_to_eliminate.push_back(row);
                }
            }
            if rows_to_eliminate.len() == 0 {
                return false;
            }

            let mut new = Conventional::new(self.level.dimensions());
            let mut row_src = 0;
            for row in 0..self.level.rows {
                while rows_to_eliminate.front().map_or(false, |r| *r == row_src) {
                    row_src += 1;
                    rows_to_eliminate.pop_front();
                }

                for col in 0..self.level.columns {
                    new[(row, col)] = self.level[(row_src, col)];
                }

                row_src += 1;
                if row_src >= self.level.rows {
                    break;
                }
            }

            self.level = new;
            true
        }

        /// Return true if the any part of the shape is out of bound
        fn check_shape_out_of_bound(&self, s: Option<&ShapeInLevel>) -> bool {
            let s1 = s.or_else(|| self.shape.as_ref()).unwrap();
            let pos = s1.pos;
            let s_width = s1.shape.width() as isize;
            let s_height = s1.shape.height() as isize;

            let l_width = self.level.columns as isize;
            let l_height = self.level.rows as isize;

            // Check if the shape is still in the level boundary
            pos.0 < 0 || (pos.0 + s_height) > l_height || pos.1 < 0 || (pos.1 + s_width) > l_width
        }

        /// Return true if the shape collides with any cells in the level.
        fn check_collision(&self, s: Option<&ShapeInLevel>) -> bool {
            let s1 = s.or_else(|| self.shape.as_ref()).unwrap();
            let s_width = s1.shape.width() as isize;
            let s_height = s1.shape.height() as isize;

            // Check if the shape collides with existing cell in the level
            for hi in 0..s_height {
                for wi in 0..s_width {
                    let s_pos = (hi as usize, wi as usize);
                    let l_pos = ((s1.pos.0 + hi) as usize, (s1.pos.1 + wi) as usize);
                    if s1.shape.cells()[s_pos] && self.level[l_pos] {
                        return true;
                    }
                }
            }
            false
        }

        /// Reset game level and switch to state State::Playing
        fn reset(&mut self) {
            for x in self.level.iter_mut() {
                *x = false;
            }
            self.create_new_shape();
            self.state = State::Playing;
        }

        fn create_new_shape(&mut self) {
            // we create a new shape and put it in the middle of the top
            let mut s = ShapeInLevel {
                shape: self.shapes_factory.create_shape(),
                pos: (0, 0),
            };
            s.pos = (
                (self.level.rows - s.shape.height()) as isize,
                (self.level.columns as isize) / 2,
            );

            while self.check_collision(Some(&s)) {
                s.pos.0 += 1;
            }
            self.shape = Option::Some(s);
        }

        /// Move the shape, it returns true if the shape is moved without
        /// collisions.
        fn move_shape(&mut self, dir: (isize, isize)) -> bool {
            if self.state == State::Playing {
                let mut s = self.shape.take().unwrap();
                let orig_pos = s.pos;
                s.pos = (s.pos.0 + dir.0, s.pos.1 + dir.1);

                let ok =
                    !self.check_shape_out_of_bound(Some(&s)) && !self.check_collision(Some(&s));
                if !ok {
                    s.pos = orig_pos;
                }
                self.shape = Some(s);
                return ok;
            }
            false
        }

        /// Return a matrix respresting cells for the level + shape
        pub fn render(&self) -> Conventional<bool> {
            let mut res = self.level.clone();
            let s = self.shape.as_ref().unwrap();
            let s_width = s.shape.width() as isize;
            let s_height = s.shape.height() as isize;

            for hi in 0..s_height {
                let l_row = (s.pos.0 + hi) as usize;
                if l_row >= self.level.rows {
                    break;
                }
                for wi in 0..s_width {
                    let l_col = (s.pos.1 + wi) as usize;
                    if l_col >= self.level.columns {
                        break;
                    }
                    let s_pos = (hi as usize, wi as usize);
                    if s.shape.cells()[s_pos] {
                        res[(l_row, l_col)] = true;
                    }
                }
            }
            res
        }
    }
}

mod ui {
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
            if level_buf.area.height > level_area.height || level_buf.area.width > level_area.width
            {
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
}

#[allow(dead_code)]
fn test_game() {
    let mut g = game::Game::new((5, 10));
    g.handle_event(game::Event::Start);

    for _ in 0..20 {
        print_matrix(&g.render());
        println!("{:?}", g.state);
        g.handle_event(game::Event::Rotate);
        g.tick();
    }
}

use std::io;

fn main() -> Result<(), io::Error> {
    ui::start()?;
    Ok(())
}

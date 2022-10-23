#[macro_use]
extern crate matrix;

use matrix::prelude::*;

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
        Quit,
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
        level: Conventional<bool>,

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
                    if !self.check_shape_out_of_bound(Some(&new_s)) && !self.check_collision(Some(&new_s)) {
                        self.shape = Some(new_s);
                    }
                    true
                }
                Event::Quit => false,
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
                    self.level[l_pos] = s.shape.cells()[s_pos];
                }
            }
            false
        }

        fn eliminate_rows(&mut self) -> bool {
            let mut rows_to_eliminate = 0;
            for row in 0..self.level.rows {
                if (0..self.level.columns)
                    .map(|col| self.level[(row, col)])
                    .all(identity)
                {
                    rows_to_eliminate += 1;
                }
            }
            if rows_to_eliminate == 0 {
                return false;
            }

            let mut new = Conventional::new(self.level.dimensions());
            for row in rows_to_eliminate..self.level.rows {
                for col in 0..self.level.columns {
                    new[(row-rows_to_eliminate, col)] = self.level[(row, col)];
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
            pos.0 < 0
                || (pos.0 + s_height) > l_height
                || pos.1 < 0
                || (pos.1 + s_width) >= l_width
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
            let mut s = ShapeInLevel { shape: self.shapes_factory.create_shape(), pos: (0, 0) };
            s.pos = (
                (self.level.rows - s.shape.height()) as isize,
                (self.level.columns as isize) / 2,
            );

            // FIXME:
            super::print_matrix(&s.shape.cells());

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

                let ok = !self.check_shape_out_of_bound(Some(&s)) && !self.check_collision(Some(&s));
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

fn main() {
    let mut g = game::Game::new((5, 10));
    g.handle_event(game::Event::Start);

    for _ in 0..20 {
        print_matrix(&g.render());
        println!("{:?}", g.state);
        g.handle_event(game::Event::Rotate);
        g.tick();
    }
}
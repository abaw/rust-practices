use matrix::prelude::*;
use rand::prelude::*;
use std::collections::VecDeque;
use std::convert::identity;

/// A Shape is a piece you could control in a Tetris level. A true element means
/// there is a cell in that position. You could move rotate it in a
/// Tetris level.
#[derive(Debug, Clone, PartialEq)]
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

impl Eq for Shape {}

macro_rules! count_shape_row {
    () => (0);
    ( $($acc:expr),+;) => (1);
    ( $($head:expr),+; $($($tail:expr),+;)*) => (1 + count_shape_row!($($($tail),+;)*));
}

macro_rules! count_shape_col {
    ( $($head_row:expr),+; $($($tail_row:expr),+;)+) => (count_shape_col!($($head_row),+));
    () => (0);
    ( $head:expr ) => (1);
    ( $head:expr, $($tail:expr),*) => (1+ count_shape_col!($($tail),*));
}

macro_rules! shape {
    ( $($head:expr),+; $($($tail:expr),+;)* ) => {
        shape![ $($head),+; -> [$($($tail),+;)*] ]
    };
    ( $($($acc:expr),+;)* -> [$($head:expr),+; $($($tail:expr),+;)*]) => {
        shape![ $($head),+; $($($acc),+;)* -> [$($($tail),+;)*]]
    };
    ( $($($acc:expr),+;)* -> [] ) => {
        {
            const ROWS: usize = count_shape_row!($($($acc),+;)*);
            const COLS: usize = count_shape_col!($($($acc),+;)*);

            Shape::new(Conventional::from_vec(
                (ROWS, COLS),
                matrix![$($($acc),+;)*] ))
        }
    };
}


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
            shape![
                true, true;
                true, true;
            ],
            // stick
            shape![
                true;
                true;
                true;
                true;
            ],
            // J
            shape![
                true, false, false;
                true, true, true;
            ],
            // L
            shape![
                false, false, true;
                true, true, true;
            ],
            // S
            shape![
                false, true, true;
                true, true, false;
            ],
            // Z
            shape![
                true, true, false;
                false, true, true;
            ],
            // T
            shape![
                false, true, false;
                true, true, true;
            ],
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotate_shape1() {
        let factory = ShapesFactory::new();
        let mut s = factory.create_shape();
        let s_orig = s.clone();
        s.rotate();

        assert_eq!(s_orig.height(), s.width());
        assert_eq!(s_orig.width(), s.height());

        s.rotate();
        s.rotate();
        s.rotate();
        assert_eq!(s_orig, s);
    }

    #[test]
    fn rotate_shape2() {
        let factory = ShapesFactory::new();
        let mut s = factory.create_shape();
        let s_orig = s.clone();
        s.rotate();
        s.rotate();
        s.rotate();
        s.rotate();
        assert_eq!(s_orig, s);
    }
}

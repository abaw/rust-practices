#[macro_use]
extern crate matrix;

mod game;
mod ui;

use std::io;

fn main() -> Result<(), io::Error> {
    ui::start()?;
    Ok(())
}

mod coords;
mod draw;
mod game;
mod gameloop;
mod gameplay;

use crate::gameloop::game_run;

fn main() {
	game_run().unwrap();
}

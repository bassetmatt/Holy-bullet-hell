#![allow(unused_assignments)]
mod coords;
mod draw;
mod game;
mod gameplay;

use crate::{draw::N_SIZES, game::Game};
use smol_str::SmolStr;
use std::time::{Duration, Instant};
use winit::{
	error::EventLoopError,
	event::{ElementState, Event, KeyEvent, WindowEvent},
	event_loop::EventLoop,
	keyboard::Key,
};

fn main() -> Result<(), EventLoopError> {
	let event_loop = EventLoop::new()?;
	let mut game = Game::launch(&event_loop);
	// TODO: Put that in main loop when there is a menu
	game.load_levels();
	game.start_level(0);

	let mut t = Instant::now();
	let mut dt = Duration::from_secs(1);
	event_loop.run(move |event, evt_loop_target| match event {
		Event::WindowEvent { window_id, ref event } if window_id == game.window.id() => match event {
			WindowEvent::CloseRequested => {
				//TODO: Save game ?
				evt_loop_target.exit();
			},
			// The window shouldn't be manually resized
			WindowEvent::Resized(_) => {},
			WindowEvent::KeyboardInput { event: KeyEvent { logical_key, state, .. }, .. } => {
				use winit::keyboard::NamedKey::*;
				if matches!(state, ElementState::Pressed) {
					// TODO: Move these into a function
					match logical_key {
						Key::Named(Escape) => {
							evt_loop_target.exit();
						},
						Key::Character(key) if key == &SmolStr::new("[") => {
							game.infos.resolution_choice += 1;
							game.infos.resolution_choice %= N_SIZES;
							game.resize();
						},
						Key::Character(key) if key == &SmolStr::new("]") => {
							game.infos.resolution_choice -= 1;
							game.infos.resolution_choice %= N_SIZES;
							game.resize();
						},
						_ => {},
					}
				}
				game.process_input(state, logical_key);
			},
			_ => {},
		},
		Event::AboutToWait => {
			// TODO: Handle game state
			// Computes time elapsed
			// TODO: Can I swap the 2 last lines ??
			dt = Instant::elapsed(&t);
			t = Instant::now();
			game.update_fps(dt);
			// TODO: The game doesn't handle game window resizing
			game.tick(dt, evt_loop_target);

			// Drawing
			game.draw_in_game();

			game.infos.update();
			game.redraw();
			game.render();
		},
		_ => {},
	})?;
	Ok(())
}

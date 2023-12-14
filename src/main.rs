#![allow(unused_assignments)]
mod coords;
mod draw;
mod game;
mod gameplay;

use pixels::Error;
use std::time::{Duration, Instant};
use winit::event_loop::{ControlFlow, EventLoop};

use crate::{draw::N_SIZES, game::Game};

fn main() -> Result<(), Error> {
	let event_loop = EventLoop::new();
	let mut game = Game::launch(&event_loop);
	// TODO: Put that in main loop when there is a menu
	game.load_levels();
	game.start_level(0);

	let mut t = Instant::now();
	let mut dt = Duration::from_secs(1);
	use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
	event_loop.run(move |event, _, control_flow| match event {
		Event::WindowEvent { window_id, ref event } if window_id == game.window.id() => match event {
			WindowEvent::CloseRequested => {
				//TODO: Save game ?
				*control_flow = ControlFlow::Exit;
			},
			// TODO: Don't allow manual resizing, only in the menu
			WindowEvent::Resized(size) => game.resize(size),
			WindowEvent::KeyboardInput {
				input: KeyboardInput { state, virtual_keycode: Some(key), .. },
				..
			} => {
				if matches!(state, ElementState::Pressed) {
					match key {
						VirtualKeyCode::Escape => {
							*control_flow = ControlFlow::Exit;
						},
						VirtualKeyCode::Plus => {
							game.options.resolution_choice += 1;
							game.options.resolution_choice %= N_SIZES as u8;
							game.cycle_window_size();
							game.resize(&game.window.inner_size());
						},
						VirtualKeyCode::Minus => {
							game.options.resolution_choice -= 1;
							game.options.resolution_choice %= N_SIZES as u8;
							game.cycle_window_size();
							game.resize(&game.window.inner_size());
						},
						_ => {},
					}
				}
				game.process_input(state, key);
			},
			_ => {},
		},
		Event::MainEventsCleared => {
			// TODO: Handle game state
			// Computes time elapsed
			// TODO: Can I swap the 2 last lines ??
			dt = Instant::elapsed(&t);
			t = Instant::now();
			game.update_fps(dt);
			// TODO: The game doesn't handle game window resizing
			game.tick(dt, control_flow);

			// Drawing
			game.draw_in_game();

			game.infos.update();
			game.redraw();
		},
		Event::RedrawRequested(_) => {
			game.render();
		},
		_ => {},
	});
}

#![allow(unused_assignments)]
mod coords;
mod draw;
mod game;
mod gameplay;

use pixels::Error;
use std::time::{Duration, Instant};
use winit::event_loop::{ControlFlow, EventLoop};

use crate::game::Game;

fn main() -> Result<(), Error> {
	let event_loop = EventLoop::new();
	let mut game = Game::launch(&event_loop);

	let mut t = Instant::now();
	let mut dt = Duration::from_secs(1);
	use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
	event_loop.run(move |event, _, control_flow| match event {
		Event::WindowEvent { window_id, ref event } if window_id == game.window.id() => match event {
			WindowEvent::CloseRequested => {
				//TODO: Save game ?
				*control_flow = ControlFlow::Exit;
			},
			WindowEvent::Resized(size) => game.resize(size),
			WindowEvent::KeyboardInput {
				input: KeyboardInput { state, virtual_keycode: Some(key), .. },
				..
			} => {
				if matches!(state, ElementState::Pressed) && matches!(key, VirtualKeyCode::Escape) {
					*control_flow = ControlFlow::Exit;
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
			game.update_fps(dt);
			t = Instant::now();

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

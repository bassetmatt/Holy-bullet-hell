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
	let mut game = Game::launch(event_loop);

	let mut t = Instant::now();
	let mut dt = Duration::from_secs(1);
	use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
	event_loop.run(move |event, _, control_flow| match event {
		Event::WindowEvent { window_id, ref event } if window_id == game.window.id() => match event {
			WindowEvent::CloseRequested
			| WindowEvent::KeyboardInput {
				input:
					KeyboardInput {
						state: ElementState::Pressed,
						virtual_keycode: Some(VirtualKeyCode::Escape),
						..
					},
				..
			} => {
				*control_flow = ControlFlow::Exit;
			},
			WindowEvent::Resized(size) => game.resize(size),
			WindowEvent::KeyboardInput {
				input: KeyboardInput { state, virtual_keycode: Some(key), .. },
				..
			} => game.process_input(state, key),
			_ => {},
		},
		Event::MainEventsCleared => {
			// Computes time elapsed
			dt = Instant::elapsed(&t);
			game.update_fps(dt);
			t = Instant::now();

			// TODO: Put in gmae.update_physic() or sthg
			// Applying events
			world.process_events();

			// Main physics calculations
			world.update_entities(dt);
			// Projectiles physics
			world.update_projectiles(dt);

			world.check_end(control_flow);

			////////////
			// Drawing
			game.draw_in_game();

			// TODO Put in sub method ??
			game.infos.update();
			game.redraw();
		},
		Event::RedrawRequested(_) => {
			frame_buffer.render().unwrap();
		},
		_ => {},
	});
}

use std::time::Instant;
use winit::{
	application::ApplicationHandler,
	error::EventLoopError,
	event::{ElementState, KeyEvent, WindowEvent},
	event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
	keyboard::Key,
};

use crate::game::{Game, MenuChoice, RunState};

struct EventLoopState {
	game_opt: Option<Game>,
}

impl ApplicationHandler for EventLoopState {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		if self.game_opt.is_none() {
			let mut game = Game::launch(event_loop);
			game.load_levels();
			self.game_opt = Some(game);
		}
	}

	fn window_event(
		&mut self,
		event_loop: &ActiveEventLoop,
		window_id: winit::window::WindowId,
		event: WindowEvent,
	) {
		let game = self.game_opt.as_mut().unwrap();
		if window_id != game.window.id() {
			return;
		}
		match event {
			WindowEvent::CloseRequested => {
				event_loop.exit();
			},
			WindowEvent::Resized(size) => {
				game.resize(&size);
			},

			WindowEvent::KeyboardInput { event: KeyEvent { ref logical_key, state, .. }, .. } => {
				use winit::keyboard::NamedKey::*;
				if matches!(state, ElementState::Pressed) {
					// TODO: Move these into a function ???
					if logical_key == &Key::Named(Escape) && game.state == RunState::Playing {
						game.world = None;
						game.state = RunState::Menu(MenuChoice::Play);
					}
				}
				game.process_input(&state, logical_key);
				if game.state == RunState::Quitting {
					event_loop.exit();
				}
			},
			_ => {},
		}
	}

	fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
		let game = self.game_opt.as_mut().unwrap();
		// TODO: Handle game state
		// Computes time elapsed
		// TODO: Can I swap the 2 last lines ??
		game.infos.dt = Instant::elapsed(&game.infos.t);
		game.infos.t = Instant::now();
		game.update_fps();

		match game.state {
			RunState::Playing => {
				game.tick(event_loop);

				// Drawing
				game.draw_in_game();

				game.infos.update();
				game.redraw();
				game.render();
			},
			RunState::Menu(choice) => {
				game.draw_menu(choice);

				game.infos.update();
				game.redraw();
				game.render();
			},
			_ => {},
		}
	}

	fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
		let _game = self.game_opt.as_mut().unwrap();
		// TODO: Implement game save???
		// game.save();
		// game.window.close();
	}
}

pub fn game_run() -> Result<(), EventLoopError> {
	let event_loop = EventLoop::new()?;
	event_loop.set_control_flow(ControlFlow::Poll);
	let mut loop_state = EventLoopState { game_opt: None };
	event_loop.run_app(&mut loop_state)
}

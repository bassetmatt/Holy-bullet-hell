#![allow(unused_assignments)]
mod coords;
mod draw;
mod game;
mod gameplay;

use pixels::{Error, SurfaceTexture};
use std::time::{Duration, Instant};
use winit::dpi::PhysicalSize;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use crate::coords::Dimensions;
use crate::game::Game;

fn main() -> Result<(), Error> {
	env_logger::init();
	let event_loop = EventLoop::new();
	let window = {
		use draw::{WIN_H, WIN_W};
		let win_size = PhysicalSize::new(WIN_W, WIN_H);
		WindowBuilder::new()
			.with_title("Holy Bullet Hell")
			.with_inner_size(win_size)
			.with_min_inner_size(win_size)
			.with_max_inner_size(win_size)
			.build(&event_loop)
			.unwrap()
	};
	// Center the window
	let screen_size = window.available_monitors().next().unwrap().size();
	let window_outer_size = window.outer_size();
	window.set_outer_position(winit::dpi::PhysicalPosition::new(
		screen_size.width / 2 - window_outer_size.width / 2,
		screen_size.height / 2 - window_outer_size.height / 2,
	));

	use draw::{conv_srgb_to_linear, BG_COLOR};
	let bg_color_wgpu: pixels::wgpu::Color = {
		pixels::wgpu::Color {
			r: conv_srgb_to_linear(BG_COLOR[0] as f64 / 255.0),
			g: conv_srgb_to_linear(BG_COLOR[1] as f64 / 255.0),
			b: conv_srgb_to_linear(BG_COLOR[2] as f64 / 255.0),
			a: conv_srgb_to_linear(BG_COLOR[3] as f64 / 255.0),
		}
	};
	let frame_buffer_dims: Dimensions<u32> = window.inner_size().into();
	let mut frame_buffer = {
		let dims = frame_buffer_dims;
		let surface_texture = SurfaceTexture::new(dims.w, dims.h, &window);
		pixels::PixelsBuilder::new(dims.w, dims.h, surface_texture)
			.clear_color(bg_color_wgpu)
			.build()
			.unwrap()
	};
	let mut world = load_level("./levels/level1.hbh", frame_buffer_dims.into_dim::<f32>()).unwrap();
	let mut game = Game::launch();
	let mut t = Instant::now();
	let mut dt = Duration::from_secs(1);
	use winit::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
	event_loop.run(move |event, _, control_flow| match event {
		Event::WindowEvent { window_id, ref event } if window_id == window.id() => match event {
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
			WindowEvent::Resized(size) => {
				frame_buffer
					.resize_surface(size.width, size.height)
					.unwrap();
				// TODO: Adapt frame_buffer_dims
			},
			WindowEvent::KeyboardInput {
				input: KeyboardInput { state, virtual_keycode: Some(key), .. },
				..
			} => game.process_input(*state, *key),
			_ => {},
		},
		Event::MainEventsCleared => {
			// Computes time elapsed
			dt = Instant::elapsed(&t);
			world.update_fps(dt);
			t = Instant::now();

			// Applying events
			world.process_events();

			// Main physics calculations
			world.update_entities(dt);
			// Projectiles physics
			world.update_projectiles(dt);

			world.check_end(control_flow);

			////////////
			// Drawing
			world.draw_gameplay(&mut frame_buffer, frame_buffer_dims, &spritesheet);
			world.draw_interface(&mut frame_buffer, frame_buffer_dims, &font_sheet);

			world.infos.update();
			window.request_redraw();
		},
		Event::RedrawRequested(_) => {
			frame_buffer.render().unwrap();
		},
		_ => {},
	});
}

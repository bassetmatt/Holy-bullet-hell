#![allow(unused_assignments)]
mod coords;
mod draw;
mod gameplay;

use image::ImageFormat;
use pixels::{Error, SurfaceTexture};
use std::fs;
use std::time::{Duration, Instant};
use winit::dpi::PhysicalSize;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use crate::coords::Dimensions;
use crate::gameplay::{EnemyType, EventType, Game};

fn load_level(level_file: &str, dimensions: Dimensions<f32>) -> std::io::Result<Game> {
	let level_raw_data = fs::read_to_string(level_file)?;

	let mut world = Game::start(Dimensions { w: 0.8 * dimensions.w, h: dimensions.h });

	let events = level_raw_data
		.split('\n')
		.filter_map(|x| x.strip_prefix('@'));

	for event in events {
		let mut event = event.split_whitespace();
		match event.next().unwrap() {
			"spawn-enemy" => {
				let variant = match event.next().unwrap() {
					"basic" => EnemyType::Basic,
					"sniper" => EnemyType::Sniper,
					other => unimplemented!("Enemy type {other} doesn't exist"),
				};
				let t: f32 = event.next().unwrap().parse().unwrap();
				let t = Duration::from_secs_f32(t);
				let x: f32 = event.next().unwrap().parse().unwrap();
				let y: f32 = event.next().unwrap().parse().unwrap();
				let event = EventType::SpawnEnemy(t, (x, y).into(), variant);
				world.push_event(t, event);
			},
			evt => unimplemented!("Unknown event {evt}"),
		}
	}
	Ok(world)
}

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

	let font_file = include_bytes!("../assets/font.png");
	let font_sheet = image::load_from_memory_with_format(font_file, ImageFormat::Png)
		.expect("Failed to load font file");

	let sheet_file = include_bytes!("../assets/spritesheet.png");
	let spritesheet = image::load_from_memory_with_format(sheet_file, ImageFormat::Png)
		.expect("Failed to load font file");

	let mut t = Instant::now();
	let mut dt = Duration::from_secs(1);
	use winit::event::*;
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
			},
			WindowEvent::KeyboardInput {
				input: KeyboardInput { state, virtual_keycode: Some(key), .. },
				..
			} => match key {
				VirtualKeyCode::Up => world.inputs.up = matches!(state, ElementState::Pressed),
				VirtualKeyCode::Down => world.inputs.down = matches!(state, ElementState::Pressed),
				VirtualKeyCode::Left => world.inputs.left = matches!(state, ElementState::Pressed),
				VirtualKeyCode::Right => world.inputs.right = matches!(state, ElementState::Pressed),
				VirtualKeyCode::X => world.inputs.shoot = matches!(state, ElementState::Pressed),
				_ => {},
			},
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

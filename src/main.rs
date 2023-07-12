use cgmath::{Point2, Vector2, Zero};
use pixels::{Error, SurfaceTexture};
use std::thread::sleep;
use std::time::Duration;
use winit::dpi::PhysicalSize;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

#[derive(Clone, Copy)]
struct Dimensions<T: Copy> {
	w: T,
	h: T,
}

impl From<winit::dpi::PhysicalSize<u32>> for Dimensions<i32> {
	fn from(size: winit::dpi::PhysicalSize<u32>) -> Dimensions<i32> {
		Dimensions { w: size.width as i32, h: size.height as i32 }
	}
}

impl From<winit::dpi::PhysicalSize<u32>> for Dimensions<u32> {
	fn from(size: winit::dpi::PhysicalSize<u32>) -> Dimensions<u32> {
		Dimensions { w: size.width, h: size.height }
	}
}

#[derive(Clone, Copy)]
struct Rect {
	top_left: Point2<i32>,
	dims: Dimensions<i32>,
}

impl Rect {
	fn top(self) -> i32 {
		self.top_left.y
	}
	fn left(self) -> i32 {
		self.top_left.x
	}
	fn bottom_excluded(self) -> i32 {
		self.top_left.y + self.dims.h
	}
	fn right_excluded(self) -> i32 {
		self.top_left.x + self.dims.w
	}

	fn contains(self, coords: Point2<i32>) -> bool {
		self.left() <= coords.x
			&& coords.x < self.right_excluded()
			&& self.top() <= coords.y
			&& coords.y < self.bottom_excluded()
	}

	fn from_float(pos: Point2<f32>, dims: Dimensions<f32>) -> Rect {
		Rect {
			top_left: Point2 {
				x: (pos.x - dims.w / 2.).round() as i32,
				y: (pos.y - dims.h / 2.).round() as i32,
			},
			dims: Dimensions { w: dims.w.round() as i32, h: dims.h.round() as i32 },
		}
	}

	fn iter(self) -> IterPointRect {
		IterPointRect::with_rect(self)
	}
}

struct IterPointRect {
	current: Point2<i32>,
	rect: Rect,
}
impl IterPointRect {
	fn with_rect(rect: Rect) -> IterPointRect {
		IterPointRect { current: rect.top_left, rect }
	}
}
impl Iterator for IterPointRect {
	type Item = Point2<i32>;
	fn next(&mut self) -> Option<Point2<i32>> {
		let coords = self.current;
		self.current.x += 1;
		if !self.rect.contains(self.current) {
			self.current.x = self.rect.left();
			self.current.y += 1;
		}
		if self.rect.contains(coords) {
			Some(coords)
		} else {
			None
		}
	}
}

#[derive(Default)]
struct Inputs {
	left: bool,
	right: bool,
	up: bool,
	down: bool,
	shoot: bool,
}

impl Inputs {
	fn new() -> Inputs {
		Inputs { ..Default::default() }
	}
}

//TODO remove
#[allow(dead_code)]
struct Player {
	pos: Point2<f32>,
	vel: Vector2<f32>,
	inputs: Inputs,
	size: Dimensions<f32>,
	size_hit: Dimensions<f32>,
	hp: u16,
}
impl Player {
	fn new() -> Self {
		Self {
			pos: (25., 25.).into(),
			vel: (0., 0.).into(),
			inputs: Inputs::new(),
			size: Dimensions { w: 48., h: 48. },
			size_hit: Dimensions { w: 10., h: 10. },
			hp: 5,
		}
	}
}

//TODO remove
#[allow(dead_code)]
struct Projectile {
	pos: Point2<f32>,
	vel: Vector2<f32>,
}

#[allow(dead_code)]
struct World {
	player: Player,
	projectiles: Vec<Projectile>,
	dims: Dimensions<i32>,
	dims_f: Dimensions<f32>,
}

impl World {
	/// Create a new `World` instance that can draw a moving box.
	fn start(dims: Dimensions<i32>) -> Self {
		Self {
			player: Player::new(),
			projectiles: Vec::new(),
			dims,
			dims_f: Dimensions { w: dims.w as f32, h: dims.h as f32 },
		}
	}
}

fn draw_rect(
	pixel_buffer: &mut pixels::Pixels,
	pixel_buffer_dims: Dimensions<u32>,
	dst: Rect,
	color: [u8; 4],
) {
	let window = Rect {
		top_left: (0, 0).into(),
		dims: Dimensions { w: pixel_buffer_dims.w as i32, h: pixel_buffer_dims.h as i32 },
	};
	for coords in dst.iter() {
		if window.contains(coords) {
			let pixel_index = coords.y * pixel_buffer_dims.w as i32 + coords.x;
			let pixel_byte_index = pixel_index as usize * 4;
			let pixel_bytes = pixel_byte_index..(pixel_byte_index + 4);
			pixel_buffer.frame_mut()[pixel_bytes].copy_from_slice(&color);
		}
	}
}

fn main() -> Result<(), Error> {
	env_logger::init();
	let event_loop = EventLoop::new();
	let window = {
		let win_size = PhysicalSize::new(480, 360);
		WindowBuilder::new()
			.with_title("Holy Bullet Hell")
			.with_inner_size(win_size)
			.with_min_inner_size(win_size)
			// .with_max_inner_size(max_size)
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

	let bg_color = [0x5b, 0xce, 0xfa, 0xff];
	let bg_color_wgpu = {
		fn conv_srgb_to_linear(x: f64) -> f64 {
			// See https://github.com/gfx-rs/wgpu/issues/2326
			// Stolen from https://github.com/three-rs/three/blob/07e47da5e0673aa9a16526719e16debd59040eec/src/color.rs#L42
			// (licensed MIT, not a substancial portion so not concerned by license obligations)
			// Basically the brightness is adjusted somewhere by wgpu or something due to sRGB stuff,
			// color is hard.
			if x > 0.04045 {
				((x + 0.055) / 1.055).powf(2.4)
			} else {
				x / 12.92
			}
		}
		pixels::wgpu::Color {
			r: conv_srgb_to_linear(bg_color[0] as f64 / 255.0),
			g: conv_srgb_to_linear(bg_color[1] as f64 / 255.0),
			b: conv_srgb_to_linear(bg_color[2] as f64 / 255.0),
			a: conv_srgb_to_linear(bg_color[3] as f64 / 255.0),
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
	let mut world =
		World::start(Dimensions { w: frame_buffer_dims.w as i32, h: frame_buffer_dims.h as i32 });
	// let mut t = Instant::now();
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
				VirtualKeyCode::Up => world.player.inputs.up = matches!(state, ElementState::Pressed),
				VirtualKeyCode::Down => {
					world.player.inputs.down = matches!(state, ElementState::Pressed)
				},
				VirtualKeyCode::Left => {
					world.player.inputs.left = matches!(state, ElementState::Pressed)
				},
				VirtualKeyCode::Right => {
					world.player.inputs.right = matches!(state, ElementState::Pressed)
				},
				VirtualKeyCode::X => world.player.inputs.shoot = matches!(state, ElementState::Pressed),
				_ => {},
			},
			_ => {},
		},
		Event::MainEventsCleared => {
			// Main physics calculations
			sleep(Duration::from_millis(1));

			// Movement
			world.player.vel = Vector2::zero();
			let inputs = &world.player.inputs;
			if inputs.left {
				world.player.vel -= Vector2::unit_x();
			}
			if inputs.right {
				world.player.vel += Vector2::unit_x();
			}
			if inputs.up {
				world.player.vel -= Vector2::unit_y();
			}
			if inputs.down {
				world.player.vel += Vector2::unit_y();
			}

			// Update pos
			if world.player.vel != Vector2::zero() {
				let new_pos = world.player.pos + 10. * world.player.vel;
				// Separate x and y checks to allow orthogonal movement while on the edge
				if 0. <= new_pos.x && new_pos.x <= world.dims_f.w {
					world.player.pos.x = new_pos.x;
				}
				if 0. <= new_pos.y && new_pos.y <= world.dims_f.h {
					world.player.pos.y = new_pos.y;
				}
			}
			if inputs.shoot {
				let proj = Projectile {
					pos: world.player.pos - world.player.size.h / 2. * Vector2::unit_y(),
					vel: Vector2::unit_y() * -10.,
				};
				world.projectiles.push(proj);
			}
			let mut to_remove: Vec<usize> = vec![];
			for (i, proj) in world.projectiles.iter_mut().enumerate() {
				proj.pos += proj.vel;
				if proj.pos.x < 0.
					|| proj.pos.x >= world.dims_f.w
					|| proj.pos.y < 0.
					|| proj.pos.y >= world.dims_f.h
				{
					to_remove.push(i);
				}
			}
			to_remove.reverse();
			for i in to_remove {
				world.projectiles.remove(i);
			}

			////////////
			// Drawing

			// Draws Background
			frame_buffer
				.frame_mut()
				.chunks_exact_mut(4)
				.for_each(|pixel| pixel.copy_from_slice(&bg_color));

			// Draws everything else
			draw_rect(
				&mut frame_buffer,
				frame_buffer_dims,
				Rect::from_float(world.player.pos, world.player.size),
				[0x00, 0x00, 0xff, 0xff],
			);

			draw_rect(
				&mut frame_buffer,
				frame_buffer_dims,
				Rect::from_float(world.player.pos, world.player.size_hit),
				[0xff, 0x00, 0x00, 0xff],
			);

			for proj in world.projectiles.iter() {
				draw_rect(
					&mut frame_buffer,
					frame_buffer_dims,
					Rect::from_float(proj.pos, Dimensions { w: 10., h: 10. }),
					[0x00, 0xff, 0x00, 0xff],
				)
			}
			window.request_redraw();
		},
		Event::RedrawRequested(_) => {
			frame_buffer.render().unwrap();
		},
		_ => {},
	});
}

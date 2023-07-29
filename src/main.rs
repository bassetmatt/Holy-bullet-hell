mod coords;

use cgmath::{InnerSpace, Point2, Vector2, Zero};
use image::{DynamicImage, GenericImageView, ImageFormat};
use pixels::{Error, SurfaceTexture};
use std::time::{Duration, Instant};
use winit::dpi::PhysicalSize;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use crate::coords::*;

const DT_60: f32 = 1. / 60.;
// const DT_144: f32 = 1. / 144.;

impl Rect {
	fn life_bar_full(pos: Point2<f32>, dims: Dimensions<f32>) -> Rect {
		Rect {
			top_left: Point2 {
				x: (pos.x - dims.w / 2.).round() as i32,
				y: (pos.y - dims.h / 2.).round() as i32 - 8,
			},
			dims: Dimensions { w: dims.w.round() as i32, h: 8 },
		}
	}
	fn life_bar(pos: Point2<f32>, dims: Dimensions<f32>, hp_ratio: f32) -> Rect {
		Rect {
			top_left: Point2 {
				x: (pos.x - dims.w / 2.).round() as i32,
				y: (pos.y - dims.h / 2.).round() as i32 - 8,
			},
			dims: Dimensions { w: (dims.w * hp_ratio).round() as i32, h: 8 },
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

struct Player {
	pos: Point2<f32>,
	vel: Vector2<f32>,
	inputs: Inputs,
	size: Dimensions<f32>,
	size_hit: Dimensions<f32>,
	hp: u32,
	hp_cd: Cooldown,
	proj_cd: Cooldown,
}

impl Player {
	fn new() -> Self {
		Self {
			pos: (75., 200.).into(),
			vel: (0., 0.).into(),
			inputs: Inputs::new(),
			size: Dimensions { w: 48., h: 48. },
			size_hit: Dimensions { w: 10., h: 10. },
			hp: 5,
			hp_cd: Cooldown::with_duration(Duration::from_secs_f32(2.)),
			proj_cd: Cooldown::with_duration(Duration::from_secs_f32(5. * DT_60)),
		}
	}
}

struct Cooldown {
	last_emit: Option<Instant>,
	cooldown: Duration,
}

impl Cooldown {
	/// Creates cooldown with secs second duration
	fn new(secs: f32) -> Self {
		Cooldown { last_emit: None, cooldown: Duration::from_secs_f32(secs) }
	}
	fn with_duration(value: Duration) -> Self {
		Cooldown { last_emit: None, cooldown: value }
	}

	fn is_over(&self) -> bool {
		if let Some(last) = self.last_emit {
			return Instant::elapsed(&last) >= self.cooldown;
		}
		true
	}
}

#[derive(Clone, Copy)]
enum EnemyType {
	Basic,
	Sniper,
}

struct Enemy {
	pos: Point2<f32>,
	_vel: Vector2<f32>,
	size: Dimensions<f32>,
	hp: f32,
	proj_cd: Cooldown,
	variant: EnemyType,
}

impl Enemy {
	fn _new(variant: EnemyType) -> Self {
		Self {
			pos: (150., 40.).into(),
			_vel: Vector2::zero(),
			size: Dimensions { w: 48., h: 48. },
			hp: 400.,
			proj_cd: Cooldown::with_duration(Duration::from_secs_f32(10. * DT_60)),
			variant,
		}
	}

	fn spawn(pos: Point2<f32>, variant: EnemyType) -> Enemy {
		let size;
		let hp;
		let proj_cd;
		match variant {
			EnemyType::Basic => {
				hp = 100.;
				size = (48., 48.).into();
				proj_cd = Cooldown::new(10. * DT_60);
			},
			EnemyType::Sniper => {
				hp = 50.;
				size = (32., 48.).into();
				proj_cd = Cooldown::new(30. * DT_60);
			},
		};
		Self { pos, _vel: Vector2::zero(), size, hp, proj_cd, variant }
	}

	fn hp_max_from_variant(&self) -> f32 {
		match self.variant {
			EnemyType::Basic => 100.,
			EnemyType::Sniper => 50.,
		}
	}

	fn color_from_variant(&self) -> [u8; 4] {
		match self.variant {
			EnemyType::Basic => [0x60, 0x4f, 0xb2, 0xff],
			EnemyType::Sniper => [0x9f, 0x16, 0x16, 0xff],
		}
	}
}

enum ProjType {
	Basic,
	Aimed,
	PlayerShoot,
}

struct Projectile {
	pos: Point2<f32>,
	vel: Vector2<f32>,
	variant: ProjType,
}

impl Projectile {
	fn color_from_variant(&self) -> [u8; 4] {
		match self.variant {
			ProjType::Basic => [0x00, 0xff, 0x00, 0xff],
			ProjType::Aimed => [0xff, 0xff, 0x00, 0xff],
			ProjType::PlayerShoot => [0xff, 0xff, 0xff, 0xff],
		}
	}
}

struct World {
	player: Player,
	projectiles: Vec<Projectile>,
	enemies: Vec<Enemy>,
	_dims: Dimensions<i32>,
	dims_f: Dimensions<f32>,
	fps_cd: Cooldown,
}

impl World {
	/// Create a new `World` instance that can draw a moving box.
	fn start(dims: Dimensions<i32>) -> Self {
		Self {
			player: Player::new(),
			projectiles: Vec::new(),
			enemies: vec![
				Enemy::spawn((200., 100.).into(), EnemyType::Basic),
				Enemy::spawn((300., 100.).into(), EnemyType::Basic),
				Enemy::spawn((500., 150.).into(), EnemyType::Sniper),
			],
			_dims: dims,
			dims_f: Dimensions { w: dims.w as f32, h: dims.h as f32 },
			fps_cd: Cooldown::with_duration(Duration::from_millis(100)),
		}
	}
}

macro_rules! opacity {
	($color: expr, $bg: expr, $alpha:expr, $index: literal) => {
		($alpha * ($color[$index] as f32) + (1. - $alpha) * ($bg[$index] as f32)).round() as u8
	};
}

fn draw_rect(
	pixel_buffer: &mut pixels::Pixels,
	pixel_buffer_dims: Dimensions<u32>,
	dst: Rect,
	mut color: [u8; 4],
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
			if color[3] == 0x00 {
				continue;
			} else if color[3] != 0xff {
				let old_color = pixel_buffer.frame_mut().get(pixel_bytes.clone()).unwrap();
				let alpha = color[3] as f32 / 255.;
				color[0] = opacity!(color, old_color, alpha, 0);
				color[1] = opacity!(color, old_color, alpha, 1);
				color[2] = opacity!(color, old_color, alpha, 2);
				color[3] = 0xff;
			}
			pixel_buffer.frame_mut()[pixel_bytes].copy_from_slice(&color);
		}
	}
}

fn char_position(c: char) -> Option<(i32, i32)> {
	let fourth_line = "`~!@#$%^&*'\".";
	let fifth_line = "()[]{}?/\\|:;,";
	let sixth_line = "-+=_<>";
	match c {
		'A'..='M' => Some((c as i32 - 'A' as i32, 0)),
		'N'..='Z' => Some((c as i32 - 'N' as i32, 1)),
		'0'..='9' => Some((c as i32 - '0' as i32, 2)),

		ch if fourth_line.contains(ch) => {
			Some((fourth_line.chars().position(|c| c == ch).unwrap() as i32, 3))
		},
		ch if fifth_line.contains(ch) => {
			Some((fifth_line.chars().position(|c| c == ch).unwrap() as i32, 4))
		},
		ch if sixth_line.contains(ch) => {
			Some((sixth_line.chars().position(|c| c == ch).unwrap() as i32, 5))
		},
		_ => unimplemented!(),
	}
}

fn draw_text(
	frame_buffer: &mut pixels::Pixels,
	frame_buffer_dims: Dimensions<u32>,
	font_sheet: &DynamicImage,
	dst: Rect,
	mut color: [u8; 4],
	text: &str,
) {
	if color[3] == 0x00 {
		return;
	}
	let len = text.len() as i32;
	// Ensures the text zone is a multiple of pixel font size
	assert_eq!(dst.dims.w % (4 * len), 0);
	assert_eq!(dst.dims.h % 6, 0);
	let char_dims = Dimensions { w: dst.dims.w / len, h: dst.dims.h };
	for (i, c) in text.chars().enumerate() {
		if c == ' ' {
			continue;
		}
		let char_sheet_coords: Point2<i32> = char_position(c).unwrap().into();
		let top_left = dst.top_left + Vector2::new(i as i32 * char_dims.w, 0);
		let dst_c = Rect { top_left, dims: char_dims };
		let window = Rect {
			top_left: (0, 0).into(),
			dims: Dimensions { w: frame_buffer_dims.w as i32, h: frame_buffer_dims.h as i32 },
		};
		for coords in dst_c.iter() {
			if !window.contains(coords) {
				continue;
			}

			let is_visible = {
				let sx = 4 * char_sheet_coords.x + 4 * (coords.x - top_left.x) / char_dims.w;
				let sy = 6 * char_sheet_coords.y + 6 * (coords.y - top_left.y) / char_dims.h;
				let px = font_sheet.get_pixel(sx as u32, sy as u32).0;
				px[3] != 0x00
			};
			if !is_visible {
				continue;
			}
			let pixel_index = coords.y * frame_buffer_dims.w as i32 + coords.x;
			let pixel_byte_index = pixel_index as usize * 4;
			let pixel_bytes = pixel_byte_index..(pixel_byte_index + 4);
			if color[3] != 0xff {
				let old_color = frame_buffer.frame_mut().get(pixel_bytes.clone()).unwrap();
				let alpha = color[3] as f32 / 255.;
				color[0] = opacity!(color, old_color, alpha, 0);
				color[1] = opacity!(color, old_color, alpha, 1);
				color[2] = opacity!(color, old_color, alpha, 2);
				color[3] = 0xff;
			}
			frame_buffer.frame_mut()[pixel_bytes].copy_from_slice(&color);
		}
	}
}

fn main() -> Result<(), Error> {
	const WIN_W: u32 = 1280;
	const WIN_H: u32 = 720;
	env_logger::init();
	let event_loop = EventLoop::new();
	let window = {
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

	let bg_color = [0x08, 0x0b, 0x1e, 0xff];
	let bg_color_ui = [0x20, 0x11, 0x38, 0xff];
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
			r: conv_srgb_to_linear(bg_color_ui[0] as f64 / 255.0),
			g: conv_srgb_to_linear(bg_color_ui[1] as f64 / 255.0),
			b: conv_srgb_to_linear(bg_color_ui[2] as f64 / 255.0),
			a: conv_srgb_to_linear(bg_color_ui[3] as f64 / 255.0),
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
	let mut world = World::start(Dimensions {
		w: (0.8 * frame_buffer_dims.w as f32) as i32,
		h: frame_buffer_dims.h as i32,
	});

	let font_file = include_bytes!("../assets/font.png");
	let font_sheet = image::load_from_memory_with_format(font_file, ImageFormat::Png)
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
			let player = &mut world.player;
			// Movement
			player.vel = Vector2::zero();
			let inputs = &player.inputs;
			if inputs.left {
				player.vel -= Vector2::unit_x();
			}
			if inputs.right {
				player.vel += Vector2::unit_x();
			}
			if inputs.up {
				player.vel -= Vector2::unit_y();
			}
			if inputs.down {
				player.vel += Vector2::unit_y();
			}

			// Update pos
			if player.vel != Vector2::zero() {
				let new_pos = player.pos + 5. * player.vel;
				// Separate x and y checks to allow orthogonal movement while on the edge
				if 0. <= new_pos.x && new_pos.x <= world.dims_f.w {
					player.pos.x = new_pos.x;
				}
				if 0. <= new_pos.y && new_pos.y <= world.dims_f.h {
					player.pos.y = new_pos.y;
				}
			}
			if inputs.shoot & player.proj_cd.is_over() {
				let proj = Projectile {
					pos: player.pos - player.size.h / 2. * Vector2::unit_y(),
					vel: Vector2::unit_y() * -5.,
					variant: ProjType::PlayerShoot,
				};
				world.projectiles.push(proj);
				player.proj_cd.last_emit = Some(Instant::now());
			}
			for enemy in world.enemies.iter_mut() {
				if enemy.proj_cd.is_over() {
					let proj = {
						let pos = enemy.pos + enemy.size.h * 0.6 * Vector2::unit_y();
						match enemy.variant {
							EnemyType::Basic => {
								Projectile { pos, vel: Vector2::unit_y() * 5., variant: ProjType::Basic }
							},
							EnemyType::Sniper => {
								let delta = player.pos - pos;
								let mut to_player = Vector2::zero();
								if delta != Vector2::zero() {
									to_player = delta.normalize();
								}
								Projectile { pos, vel: 5. * to_player, variant: ProjType::Aimed }
							},
						}
					};
					world.projectiles.push(proj);
					enemy.proj_cd.last_emit = Some(Instant::now());
				}
			}

			fn collide_rectangle(
				pos_a: Point2<f32>,
				pos_b: Point2<f32>,
				size_a: Dimensions<f32>,
				size_b: Dimensions<f32>,
			) -> bool {
				((pos_a.x - size_a.w / 2. <= pos_b.x - size_b.w / 2.
					&& pos_b.x - size_b.w / 2. <= pos_a.x + size_a.w / 2.)
					|| (pos_a.x - size_a.w / 2. <= pos_b.x + size_b.w / 2.
						&& pos_b.x + size_b.w / 2. <= pos_a.x + size_a.w / 2.))
					&& ((pos_a.y - size_a.h / 2. <= pos_b.y - size_b.h / 2.
						&& pos_b.y - size_b.h / 2. <= pos_a.y + size_a.h / 2.)
						|| (pos_a.y - size_a.h / 2. <= pos_b.y + size_b.h / 2.
							&& pos_b.y + size_b.h / 2. <= pos_a.y + size_a.h / 2.))
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
				} else {
					for (j, enemy) in world.enemies.iter_mut().enumerate() {
						if collide_rectangle(
							enemy.pos,
							proj.pos,
							enemy.size,
							Dimensions { w: 10., h: 10. },
						) {
							enemy.hp -= 2.;
							to_remove.push(i);
							if enemy.hp <= 0. {
								world.enemies.remove(j);
								break;
							}
						}
					}
					if player.hp_cd.is_over()
						& collide_rectangle(
							player.pos,
							proj.pos,
							player.size_hit,
							Dimensions { w: 10., h: 10. },
						) {
						player.hp -= 1;
						to_remove.push(i);
						if player.hp == 0 {
							// Goofiest dead message
							println!("Ur so dead 💀, RIP BOZO 🔫🔫😂😂😂😂");
							*control_flow = ControlFlow::Exit;
							break;
						}
						player.hp_cd.last_emit = Some(Instant::now());
					}
				}
			}
			for i in to_remove.into_iter().rev() {
				world.projectiles.remove(i);
			}

			////////////
			// Drawing

			// Draws Background
			frame_buffer
				.frame_mut()
				.chunks_exact_mut(4)
				.for_each(|pixel| pixel.copy_from_slice(&bg_color));

			// Player
			let player_color = if player.hp_cd.is_over() {
				[0x00, 0x00, 0xff, 0xff]
			} else {
				[0x00, 0x00, 0xff, 0x50]
			};
			draw_rect(
				&mut frame_buffer,
				frame_buffer_dims,
				Rect::from_float(player.pos, player.size),
				player_color,
			);

			// Player hitbox
			draw_rect(
				&mut frame_buffer,
				frame_buffer_dims,
				Rect::from_float(player.pos, player.size_hit),
				[0xff, 0x00, 0x00, 0xff],
			);

			// Enemies
			for enemy in world.enemies.iter() {
				draw_rect(
					&mut frame_buffer,
					frame_buffer_dims,
					Rect::from_float(enemy.pos, enemy.size),
					enemy.color_from_variant(),
				);
				draw_rect(
					&mut frame_buffer,
					frame_buffer_dims,
					Rect::life_bar_full(enemy.pos, enemy.size),
					[0xff, 0x00, 0x00, 0xff],
				);
				draw_rect(
					&mut frame_buffer,
					frame_buffer_dims,
					Rect::life_bar(
						enemy.pos,
						enemy.size,
						enemy.hp / enemy.hp_max_from_variant(),
					),
					[0x00, 0xff, 0x00, 0xff],
				);
			}

			//projectiles
			for proj in world.projectiles.iter() {
				draw_rect(
					&mut frame_buffer,
					frame_buffer_dims,
					Rect::from_float(proj.pos, Dimensions { w: 10., h: 10. }),
					proj.color_from_variant(),
				)
			}

			// Interface
			frame_buffer
				.frame_mut()
				.chunks_exact_mut(4)
				.enumerate()
				.for_each(|(i, pixel)| {
					if i % WIN_W as usize > (0.8 * WIN_W as f32) as usize {
						pixel.copy_from_slice(&bg_color_ui)
					}
				});
			for i in 0..player.hp {
				draw_rect(
					&mut frame_buffer,
					frame_buffer_dims,
					Rect {
						top_left: (1040 + 48 * i as i32, 256).into(),
						dims: (32, 32).into(),
					},
					[0x11, 0x81, 0x0c, 0xff],
				)
			}

			draw_text(
				&mut frame_buffer,
				frame_buffer_dims,
				&font_sheet,
				Rect { top_left: (1040, 128).into(), dims: (4 * 4 * 5, 6 * 5 * 2).into() },
				[0xff, 0x00, 0x00, 0xff],
				"TEST",
			);

			// Limit fps refresh for it to be readable
			if world.fps_cd.is_over() {
				dt = Instant::elapsed(&t);
				world.fps_cd.last_emit = Some(Instant::now());
			}
			t = Instant::now();
			let fps = format!("FPS: {fps:3}", fps = (1. / dt.as_secs_f64()).round() as u32);
			let text_dims = Dimensions { w: fps.len() as i32 * 4 * 5, h: 6 * 5 };
			draw_text(
				&mut frame_buffer,
				frame_buffer_dims,
				&font_sheet,
				Rect { top_left: (WIN_W as i32 - text_dims.w, 0).into(), dims: text_dims },
				[0xff, 0xff, 0xff, 0xb0],
				&fps,
			);
			window.request_redraw();
		},
		Event::RedrawRequested(_) => {
			frame_buffer.render().unwrap();
		},
		_ => {},
	});
}

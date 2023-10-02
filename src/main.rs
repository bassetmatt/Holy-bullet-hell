mod coords;

use cgmath::{InnerSpace, Point2, Vector2, Zero};
use image::{DynamicImage, GenericImageView, ImageFormat};
use pixels::{Error, SurfaceTexture};
use std::time::{Duration, Instant};
use std::{fs, vec};
use winit::dpi::PhysicalSize;
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use crate::coords::*;

const DT_60: f32 = 1. / 60.;
// const DT_144: f32 = 1. / 144.;

impl RectI {
	fn life_bar_full(pos: Point2<f32>, dims: Dimensions<f32>) -> RectI {
		RectI {
			top_left: Point2 {
				x: (pos.x - dims.w / 2.).round() as i32,
				y: (pos.y - dims.h / 2.).round() as i32 - 8,
			},
			dims: Dimensions { w: dims.w.round() as i32, h: 8 },
		}
	}
	fn life_bar(pos: Point2<f32>, dims: Dimensions<f32>, hp_ratio: f32) -> RectI {
		RectI {
			top_left: Point2 {
				x: (pos.x - dims.w / 2.).round() as i32,
				y: (pos.y - dims.h / 2.).round() as i32 - 8,
			},
			dims: Dimensions { w: (dims.w * hp_ratio).round() as i32, h: 8 },
		}
	}
}

struct Cooldown {
	last_emit: Option<Instant>,
	cooldown: Duration,
}

impl Cooldown {
	/// Creates cooldown with secs second duration
	fn with_secs(secs: f32) -> Self {
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

struct Player {
	pos: Point2<f32>,
	vel: Vector2<f32>,
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
			size: Dimensions { w: 48., h: 48. },
			size_hit: Dimensions { w: 10., h: 10. },
			hp: 5,
			hp_cd: Cooldown::with_secs(2.),
			proj_cd: Cooldown::with_secs(5. * DT_60),
		}
	}

	fn sprite_coords(&self) -> SpriteCoords {
		SpriteCoords {
			sheet_pos: if self.hp_cd.is_over() { (1, 0) } else { (1, 1) }.into(),
			dims: (8, 8).into(),
		}
	}

	fn sprite_coords_hit(&self) -> SpriteCoords {
		SpriteCoords { sheet_pos: (0, 0).into(), dims: (8, 8).into() }
	}

	fn update_pos(&mut self, inputs: &Inputs, bounds: RectF, dt: f32) {
		// Inputs
		self.vel = Vector2::zero();
		if inputs.left {
			self.vel -= Vector2::unit_x();
		}
		if inputs.right {
			self.vel += Vector2::unit_x();
		}
		if inputs.up {
			self.vel -= Vector2::unit_y();
		}
		if inputs.down {
			self.vel += Vector2::unit_y();
		}

		// Update pos
		if self.vel != Vector2::zero() {
			let new_pos = self.pos + 5. * self.vel * dt / DT_60;
			// Separate x and y checks to allow movement while on an edge
			if 0. <= new_pos.x && new_pos.x <= bounds.dims.w {
				self.pos.x = new_pos.x;
			}
			if 0. <= new_pos.y && new_pos.y <= bounds.dims.h {
				self.pos.y = new_pos.y;
			}
		}
	}
}

#[derive(Clone, Copy)]
enum EnemyType {
	Basic,
	Sniper,
}

enum EnemyState {
	NotSpawned,
	_OnScreen(fn(&mut Enemy)),
	_OffScreen,
}

struct Enemy {
	pos: Point2<f32>,
	vel: Vector2<f32>,
	size: Dimensions<f32>,
	hp: f32,
	proj_cd: Cooldown,
	variant: EnemyType,
	_state: EnemyState,
}

impl Enemy {
	fn _new(variant: EnemyType) -> Self {
		Self {
			pos: (150., 40.).into(),
			vel: Vector2::zero(),
			size: Dimensions { w: 48., h: 48. },
			hp: 400.,
			proj_cd: Cooldown::with_secs(10. * DT_60),
			variant,
			_state: EnemyState::NotSpawned,
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
				proj_cd = Cooldown::with_secs(10. * DT_60);
			},
			EnemyType::Sniper => {
				hp = 50.;
				size = (32., 48.).into();
				proj_cd = Cooldown::with_secs(30. * DT_60);
			},
		};
		Self {
			pos,
			vel: Vector2::zero(),
			size,
			hp,
			proj_cd,
			variant,
			_state: EnemyState::NotSpawned,
		}
	}

	fn hp_max_from_variant(&self) -> f32 {
		match self.variant {
			EnemyType::Basic => 100.,
			EnemyType::Sniper => 50.,
		}
	}

	fn sprite_coords(&self) -> SpriteCoords {
		SpriteCoords {
			sheet_pos: match self.variant {
				EnemyType::Basic => (2, 0),
				EnemyType::Sniper => (3, 0),
			}
			.into(),
			dims: (8, 8).into(),
		}
	}

	fn update_pos(&mut self, dt: f32) {
		// Enemies behavior
		const SPEED: f32 = 0.5;
		match self.variant {
			EnemyType::Basic => {
				self.vel = Vector2::zero();
				if self.pos.y <= 150. {
					self.vel = Vector2::unit_y() * SPEED;
				} else if self.pos.x <= 750. {
					self.vel = Vector2::unit_x() * SPEED;
				}
			},
			EnemyType::Sniper => {
				self.vel = Vector2::zero();
				if self.pos.y <= 200. {
					self.vel = Vector2::unit_y() * SPEED;
				} else if self.pos.x <= 600. {
					self.vel = Vector2::unit_x() * SPEED
				}
				if self.pos.x >= 600. && self.pos.y >= 100. {
					self.vel = -Vector2::unit_y() * SPEED;
				}
			},
		}
		// Update pos
		if self.vel != Vector2::zero() {
			self.pos += self.vel * dt / DT_60;
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
	fn sprite_coords(&self) -> SpriteCoords {
		SpriteCoords {
			sheet_pos: match self.variant {
				ProjType::Basic => (2, 1),
				ProjType::Aimed => (3, 1),
				ProjType::PlayerShoot => (0, 1),
			}
			.into(),
			dims: (8, 8).into(),
		}
	}
}

struct GlobalInfo {
	begin: Instant,
	time: Duration,
	frame_count: u64,
}

impl GlobalInfo {
	fn new() -> GlobalInfo {
		GlobalInfo {
			begin: Instant::now(),
			time: Duration::from_secs(0),
			frame_count: 0,
		}
	}
}

enum EventType {
	SpawnEnemy(Duration, Point2<f32>, EnemyType),
	_SpawnBoss(Duration, Point2<f32>),
}
struct Event {
	time: Instant,
	variant: EventType,
}

#[derive(Default)]
struct Inputs {
	left: bool,
	right: bool,
	up: bool,
	down: bool,
	shoot: bool,
	_pause: bool,
}

impl Inputs {
	fn new() -> Inputs {
		Inputs { ..Default::default() }
	}
}

struct Game {
	player: Player,
	projectiles: Vec<Projectile>,
	enemies: Vec<Enemy>,
	rect: RectF,
	inputs: Inputs,
	fps_cd: Cooldown,
	fps: u32,
	infos: GlobalInfo,
	event_list: Vec<Event>,
}

impl Game {
	/// Create a new `World` instance that can draw a moving box.
	fn start(dims: Dimensions<f32>) -> Self {
		Self {
			player: Player::new(),
			projectiles: Vec::new(),
			enemies: vec![],
			rect: dims.into_rect(),
			inputs: Inputs::new(),
			fps_cd: Cooldown::with_duration(Duration::from_millis(100)),
			fps: 60,
			infos: GlobalInfo::new(),
			event_list: vec![],
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
	dst: RectI,
	mut color: [u8; 4],
) {
	let window = pixel_buffer_dims.into_rect();
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

fn char_position(c: char) -> Option<(u32, u32)> {
	let fourth_line = "`~!@#$%^&*'\".";
	let fifth_line = "()[]{}?/\\|:;,";
	let sixth_line = "-+=_<>";
	match c {
		'A'..='M' => Some((c as u32 - 'A' as u32, 0)),
		'N'..='Z' => Some((c as u32 - 'N' as u32, 1)),
		'0'..='9' => Some((c as u32 - '0' as u32, 2)),

		ch if fourth_line.contains(ch) => {
			Some((fourth_line.chars().position(|c| c == ch).unwrap() as u32, 3))
		},
		ch if fifth_line.contains(ch) => {
			Some((fifth_line.chars().position(|c| c == ch).unwrap() as u32, 4))
		},
		ch if sixth_line.contains(ch) => {
			Some((sixth_line.chars().position(|c| c == ch).unwrap() as u32, 5))
		},
		_ => unimplemented!("Character {c} doesn't exist in font"),
	}
}

struct SpriteCoords {
	sheet_pos: Point2<u32>,
	dims: Dimensions<u32>,
}

fn draw_text(
	frame_buffer: &mut pixels::Pixels,
	frame_buffer_dims: Dimensions<u32>,
	font_sheet: &DynamicImage,
	dst: RectI,
	color: [u8; 4],
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
		let top_left = dst.top_left + Vector2::new(i as i32 * char_dims.w, 0);
		let dst_c = Rect { top_left, dims: char_dims };
		draw_sprite(
			frame_buffer,
			frame_buffer_dims,
			font_sheet,
			SpriteCoords { sheet_pos: char_position(c).unwrap().into(), dims: (4, 6).into() },
			dst_c,
			Some(color),
		);
	}
}

fn draw_sprite(
	frame_buffer: &mut pixels::Pixels,
	frame_buffer_dims: Dimensions<u32>,
	sheet: &DynamicImage,
	SpriteCoords { sheet_pos, dims }: SpriteCoords,
	dst: RectI,
	color: Option<[u8; 4]>,
) {
	let window = Rect {
		top_left: (0, 0).into(),
		dims: frame_buffer_dims.into_dim::<i32>(),
	};
	for coords in dst.iter() {
		if !window.contains(coords) {
			continue;
		}
		let mut px = {
			let sx =
				dims.w * sheet_pos.x + dims.w * (coords.x - dst.top_left.x) as u32 / dst.dims.w as u32;
			let sy =
				dims.h * sheet_pos.y + dims.h * (coords.y - dst.top_left.y) as u32 / dst.dims.h as u32;
			sheet.get_pixel(sx, sy).0
		};
		if px[3] == 0x00 {
			continue;
		}
		let pixel_index = coords.y * frame_buffer_dims.w as i32 + coords.x;
		let pixel_byte_index = pixel_index as usize * 4;
		let pixel_bytes = pixel_byte_index..(pixel_byte_index + 4);
		px = match color {
			None => px,
			Some(col) => col,
		};
		if px[3] != 0xff {
			let background = frame_buffer.frame_mut().get(pixel_bytes.clone()).unwrap();
			let alpha = px[3] as f32 / 255.;
			px[0] = opacity!(px, background, alpha, 0);
			px[1] = opacity!(px, background, alpha, 1);
			px[2] = opacity!(px, background, alpha, 2);
			px[3] = 0xff;
		}
		frame_buffer.frame_mut()[pixel_bytes].copy_from_slice(&px);
	}
}

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

				world
					.event_list
					.push(Event { time: world.infos.begin + t, variant: event });
			},
			evt => unimplemented!("Unknown event {evt}"),
		}
	}
	Ok(world)
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

	const BG_COLOR: [u8; 4] = [0x08, 0x0b, 0x1e, 0xff];
	const BG_COLOR_UI: [u8; 4] = [0x20, 0x11, 0x38, 0xff];
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
			r: conv_srgb_to_linear(BG_COLOR_UI[0] as f64 / 255.0),
			g: conv_srgb_to_linear(BG_COLOR_UI[1] as f64 / 255.0),
			b: conv_srgb_to_linear(BG_COLOR_UI[2] as f64 / 255.0),
			a: conv_srgb_to_linear(BG_COLOR_UI[3] as f64 / 255.0),
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
	let mut world = load_level(
		"./levels/level1.hbh",
		Dimensions { w: frame_buffer_dims.w as f32, h: frame_buffer_dims.h as f32 },
	)
	.unwrap();

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
			// Applying events
			let mut to_remove = vec![];
			for (i, e) in world.event_list.iter().enumerate() {
				if e.time >= Instant::now() {
					if let EventType::SpawnEnemy(_, pos, variant) = e.variant {
						world.enemies.push(Enemy::spawn(pos, variant));
					}
					to_remove.push(i);
				}
			}
			for i in to_remove.into_iter().rev() {
				world.event_list.remove(i);
			}
			////
			// Main physics calculations
			// Player
			let player = &mut world.player;
			let inputs = &world.inputs;
			player.update_pos(inputs, world.rect, dt.as_secs_f32());
			// Player shoot
			if inputs.shoot & player.proj_cd.is_over() {
				let proj = Projectile {
					pos: player.pos - player.size.h / 2. * Vector2::unit_y(),
					vel: Vector2::unit_y() * -5.,
					variant: ProjType::PlayerShoot,
				};
				world.projectiles.push(proj);
				player.proj_cd.last_emit = Some(Instant::now());
			}

			// Enemies physics
			for enemy in world.enemies.iter_mut() {
				// Updates position
				enemy.update_pos(dt.as_secs_f32());
				// Shooting
				if enemy.proj_cd.is_over() && world.rect.contains(enemy.pos) {
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

			/// Collider helper function
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

			// Projectiles physics
			let mut to_remove: Vec<usize> = vec![];
			for (i, proj) in world.projectiles.iter_mut().enumerate() {
				proj.pos += proj.vel * dt.as_secs_f32() / DT_60;
				if !world.rect.contains(proj.pos) {
					to_remove.push(i);
					continue;
				}
				for (j, enemy) in world.enemies.iter_mut().enumerate() {
					if collide_rectangle(
						enemy.pos,
						proj.pos,
						enemy.size,
						Dimensions { w: 10., h: 10. },
					) & matches!(proj.variant, ProjType::PlayerShoot)
					{
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
					) & !matches!(proj.variant, ProjType::PlayerShoot)
				{
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
			for i in to_remove.into_iter().rev() {
				world.projectiles.remove(i);
			}

			////////////
			// Drawing

			// Draws Background
			frame_buffer
				.frame_mut()
				.chunks_exact_mut(4)
				.for_each(|pixel| pixel.copy_from_slice(&BG_COLOR));

			// Player

			draw_sprite(
				&mut frame_buffer,
				frame_buffer_dims,
				&spritesheet,
				player.sprite_coords(),
				Rect::from_float(player.pos, player.size),
				None,
			);
			// Player hitbox
			draw_sprite(
				&mut frame_buffer,
				frame_buffer_dims,
				&spritesheet,
				player.sprite_coords_hit(),
				Rect::from_float(player.pos, player.size_hit),
				None,
			);

			// Enemies
			for enemy in world.enemies.iter() {
				draw_sprite(
					&mut frame_buffer,
					frame_buffer_dims,
					&spritesheet,
					enemy.sprite_coords(),
					Rect::from_float(enemy.pos, enemy.size),
					None,
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
				draw_sprite(
					&mut frame_buffer,
					frame_buffer_dims,
					&spritesheet,
					proj.sprite_coords(),
					Rect::from_float(proj.pos, Dimensions { w: 10., h: 10. }),
					None,
				);
			}

			// Interface
			frame_buffer
				.frame_mut()
				.chunks_exact_mut(4)
				.enumerate()
				.for_each(|(i, pixel)| {
					if i % WIN_W as usize > (0.8 * WIN_W as f32) as usize {
						pixel.copy_from_slice(&BG_COLOR_UI)
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

			let s = "LEVEL 1";
			draw_text(
				&mut frame_buffer,
				frame_buffer_dims,
				&font_sheet,
				Rect {
					top_left: (1040, 128).into(),
					dims: (4 * s.len() as i32 * 5, 6 * 5 * 2).into(),
				},
				[0xff, 0x00, 0x00, 0xff],
				s,
			);

			// Limit fps refresh for it to be readable
			dt = Instant::elapsed(&t);
			if world.fps_cd.is_over() {
				world.fps = (1. / dt.as_secs_f64()).round() as u32;
				world.fps_cd.last_emit = Some(Instant::now());
			}
			t = Instant::now();
			let fps_str = format!("FPS: {fps:3}", fps = world.fps);
			let text_dims = Dimensions { w: fps_str.len() as i32 * 4 * 5, h: 6 * 5 };
			draw_text(
				&mut frame_buffer,
				frame_buffer_dims,
				&font_sheet,
				Rect { top_left: (WIN_W as i32 - text_dims.w, 0).into(), dims: text_dims },
				[0xff, 0xff, 0xff, 0xb0],
				&fps_str,
			);
			world.infos.time = Instant::elapsed(&world.infos.begin);
			world.infos.frame_count += 1;
			window.request_redraw();
		},
		Event::RedrawRequested(_) => {
			frame_buffer.render().unwrap();
		},
		_ => {},
	});
}

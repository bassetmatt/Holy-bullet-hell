use cgmath::{Point2, Vector2};
use image::{DynamicImage, GenericImageView, ImageFormat};
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Fullscreen, Window, WindowBuilder};

use crate::coords::{Dimensions, Rect, RectI};
use crate::game::Game;
use crate::gameplay::World;
use crate::gameplay::{Enemy, EnemyType, Player, ProjType, Projectile};

pub const BG_COLOR: [u8; 4] = [0x08, 0x0b, 0x1e, 0xff];
const BG_COLOR_UI: [u8; 4] = [0x20, 0x11, 0x38, 0xff];

pub struct Sheets {
	font: DynamicImage,
	spritesheet: DynamicImage,
}

impl Sheets {
	pub fn load() -> Self {
		const FONT_FILE: &[u8] = include_bytes!("../assets/font.png");
		const SPRITESHEET_FILE: &[u8] = include_bytes!("../assets/spritesheet.png");
		let font: DynamicImage = image::load_from_memory_with_format(FONT_FILE, ImageFormat::Png)
			.expect("Failed to load font file");
		let spritesheet: DynamicImage =
			image::load_from_memory_with_format(SPRITESHEET_FILE, ImageFormat::Png)
				.expect("Failed to load spritesheet");
		Sheets { font, spritesheet }
	}
}

pub fn conv_srgb_to_linear(x: f64) -> f64 {
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

const SIZES: [Dimensions<u32>; 5] = [
	Dimensions { w: 640, h: 360 },
	Dimensions { w: 960, h: 540 },
	Dimensions { w: 1280, h: 720 },
	Dimensions { w: 1600, h: 900 },
	Dimensions { w: 1920, h: 1080 },
];
pub const N_SIZES: usize = SIZES.len();

pub fn create_window(event_loop: &EventLoop<()>) -> Window {
	let window = {
		let win_size = PhysicalSize::new(SIZES[2].w, SIZES[2].h);
		WindowBuilder::new()
			.with_title("Holy Bullet Hell")
			.with_inner_size(win_size)
			.with_fullscreen(None)
			.build(event_loop)
			.unwrap()
	};
	// Centers the window
	let screen_size = window.available_monitors().next().unwrap().size();
	let window_outer_size = window.outer_size();
	window.set_outer_position(winit::dpi::PhysicalPosition::new(
		screen_size.width / 2 - window_outer_size.width / 2,
		screen_size.height / 2 - window_outer_size.height / 2,
	));
	window
}

pub struct FrameBuffer {
	pub buffer: Pixels,
	pub dims: Dimensions<u32>,
}

impl FrameBuffer {
	pub fn new(window: &Window) -> Self {
		let dims: Dimensions<u32> = window.inner_size().into();
		let bg_color_wgpu: pixels::wgpu::Color = {
			pixels::wgpu::Color {
				r: conv_srgb_to_linear(BG_COLOR[0] as f64 / 255.0),
				g: conv_srgb_to_linear(BG_COLOR[1] as f64 / 255.0),
				b: conv_srgb_to_linear(BG_COLOR[2] as f64 / 255.0),
				a: conv_srgb_to_linear(BG_COLOR[3] as f64 / 255.0),
			}
		};
		let buffer = {
			let surface_texture = SurfaceTexture::new(dims.w, dims.h, &window);
			pixels::PixelsBuilder::new(dims.w, dims.h, surface_texture)
				.clear_color(bg_color_wgpu)
				.build()
				.unwrap()
		};
		FrameBuffer { buffer, dims }
	}
}

impl Game {
	pub fn redraw(&mut self) {
		self.window.request_redraw();
	}

	pub fn resize(&mut self, size: &PhysicalSize<u32>) {
		let buf = &mut self.frame_buffer.buffer;
		// Resize the window surface
		buf.resize_surface(size.width, size.height).unwrap();
		// Resize the pixel buffer
		buf.resize_buffer(size.width, size.height).unwrap();
		// Update the dimensions
		self.frame_buffer.dims = self.window.inner_size().into();
	}

	pub fn cycle_window_size(&mut self) {
		// Last entry must be the screen size
		let index = self.options.resolution_choice as usize;
		if index == N_SIZES - 1 {
			self
				.window
				.set_fullscreen(Some(winit::window::Fullscreen::Borderless(
					self.window.current_monitor(),
				)));
		} else {
			self.window.set_fullscreen(None);
		}
		self
			.window
			.set_inner_size(winit::dpi::LogicalSize::new(SIZES[index].w, SIZES[index].h));
	}

	pub fn _toggle_fullscreen(&mut self) {
		let window = &self.window;
		if window.fullscreen().is_some() {
			window.set_fullscreen(None);
		} else {
			let fs = Fullscreen::Borderless(window.current_monitor());
			window.set_fullscreen(Some(fs));
		}
	}

	pub fn draw_in_game(&mut self) {
		let world = &mut self.world.as_mut().unwrap();
		self
			.frame_buffer
			.buffer
			.frame_mut()
			.chunks_exact_mut(4)
			.for_each(|pixel| pixel.copy_from_slice(&BG_COLOR));

		world.draw_gameplay(&mut self.frame_buffer, &self.sheets);
		world.draw_interface(
			&mut self.frame_buffer,
			&self.sheets,
			self.infos.fps_info.fps,
		);
	}

	pub fn render(&mut self) {
		self.frame_buffer.buffer.render().unwrap();
	}
}

macro_rules! opacity {
	($color: expr, $bg: expr, $alpha:expr, $index: literal) => {
		($alpha * ($color[$index] as f32) + (1. - $alpha) * ($bg[$index] as f32)).round() as u8
	};
}

// TODO: Change arguments to FrameBuffer
pub fn draw_rect(
	pixel_buffer: &mut pixels::Pixels,
	pixel_buffer_dims: Dimensions<u32>,
	dst: RectI,
	mut color: [u8; 4],
) {
	// Transparent
	if color[3] == 0x00 {
		return;
	}
	let window = pixel_buffer_dims.into_rect();
	for coords in dst.iter() {
		if window.contains(coords) {
			let pixel_index = coords.y * pixel_buffer_dims.w as i32 + coords.x;
			let pixel_byte_index = pixel_index as usize * 4;
			let pixel_bytes = pixel_byte_index..(pixel_byte_index + 4);
			if color[3] != 0xff {
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

// TODO: Change arguments to FrameBuffer
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

// TODO: Change arguments to FrameBuffer
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

// TODO: Change arguments to FrameBuffer
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

// TODO: Move to gameplay.rs (I think?)
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

impl Player {
	fn sprite_coords(&self) -> SpriteCoords {
		SpriteCoords {
			sheet_pos: if self.hp_cd_over() { (1, 0) } else { (1, 1) }.into(),
			dims: (8, 8).into(),
		}
	}

	fn sprite_coords_hit(&self) -> SpriteCoords {
		SpriteCoords { sheet_pos: (0, 0).into(), dims: (8, 8).into() }
	}
}

impl Enemy {
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

impl World {
	pub fn draw_gameplay(&self, frame_buffer: &mut FrameBuffer, sheets: &Sheets) {
		let frame_buffer_dims = frame_buffer.dims;
		let frame_buffer = &mut frame_buffer.buffer;
		// Draws Background
		frame_buffer
			.frame_mut()
			.chunks_exact_mut(4)
			.for_each(|pixel| pixel.copy_from_slice(&BG_COLOR));

		// Player
		let player = &self.player;
		draw_sprite(
			frame_buffer,
			frame_buffer_dims,
			&sheets.spritesheet,
			player.sprite_coords(),
			Rect::from_float(player.pos, player.size),
			None,
		);
		// Player hitbox
		draw_sprite(
			frame_buffer,
			frame_buffer_dims,
			&sheets.spritesheet,
			player.sprite_coords_hit(),
			Rect::from_float(player.pos, player.size_hit),
			None,
		);

		// Enemies
		for enemy in self.enemies.iter() {
			draw_sprite(
				frame_buffer,
				frame_buffer_dims,
				&sheets.spritesheet,
				enemy.sprite_coords(),
				Rect::from_float(enemy.pos, enemy.size),
				None,
			);
			draw_rect(
				frame_buffer,
				frame_buffer_dims,
				Rect::life_bar_full(enemy.pos, enemy.size),
				[0xff, 0x00, 0x00, 0xff],
			);
			draw_rect(
				frame_buffer,
				frame_buffer_dims,
				Rect::life_bar(
					enemy.pos,
					enemy.size,
					enemy.hp / Enemy::max_hp(enemy.variant),
				),
				[0x00, 0xff, 0x00, 0xff],
			);
		}

		//projectiles
		for proj in self.projectiles.iter() {
			draw_sprite(
				frame_buffer,
				frame_buffer_dims,
				&sheets.spritesheet,
				proj.sprite_coords(),
				Rect::from_float(proj.pos, Dimensions { w: 10., h: 10. }),
				None,
			);
		}
	}

	pub fn draw_interface(&self, frame_buffer: &mut FrameBuffer, sheets: &Sheets, fps: u32) {
		let frame_buffer_dims = frame_buffer.dims;
		let frame_buffer = &mut frame_buffer.buffer;
		let win_w = frame_buffer_dims.w;
		let interf_begin_x = (0.8 * win_w as f32) as usize;
		// Interface background
		frame_buffer
			.frame_mut()
			.chunks_exact_mut(4)
			.enumerate()
			.for_each(|(i, pixel)| {
				if i % frame_buffer_dims.w as usize > interf_begin_x {
					pixel.copy_from_slice(&BG_COLOR_UI)
				}
			});
		for i in 0..self.player.hp {
			draw_rect(
				frame_buffer,
				frame_buffer_dims,
				Rect {
					top_left: (interf_begin_x as i32 + 16 + 48 * i as i32, 256).into(),
					dims: (32, 32).into(),
				},
				[0x11, 0x81, 0x0c, 0xff],
			)
		}

		let s = "LEVEL 1";
		draw_text(
			frame_buffer,
			frame_buffer_dims,
			&sheets.font,
			Rect {
				top_left: (interf_begin_x as i32 + 16, 128).into(),
				dims: (4 * s.len() as i32 * 5, 6 * 5 * 2).into(),
			},
			[0xff, 0x00, 0x00, 0xff],
			s,
		);

		let score_str = format!("SCORE: {score:3}", score = self.score);
		let text_dims = Dimensions { w: score_str.len() as i32 * 4 * 5, h: 6 * 5 };
		draw_text(
			frame_buffer,
			frame_buffer_dims,
			&sheets.font,
			Rect {
				top_left: (win_w as i32 - text_dims.w, 40).into(),
				dims: text_dims,
			},
			[0xff, 0xff, 0xff, 0xb0],
			&score_str,
		);

		let fps_str = format!("FPS: {fps:3}", fps = fps);
		let text_dims = Dimensions { w: fps_str.len() as i32 * 4 * 5, h: 6 * 5 };
		draw_text(
			frame_buffer,
			frame_buffer_dims,
			&sheets.font,
			Rect { top_left: (win_w as i32 - text_dims.w, 0).into(), dims: text_dims },
			[0xff, 0xff, 0xff, 0xb0],
			&fps_str,
		);
	}
}

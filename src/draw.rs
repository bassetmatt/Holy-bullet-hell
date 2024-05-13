use std::rc::Rc;

use cgmath::{Point2, Vector2};
use image::{DynamicImage, GenericImageView, ImageFormat};
use pixels::{Pixels, SurfaceTexture, TextureError};
use winit::{
	dpi::{PhysicalPosition, PhysicalSize},
	event_loop::ActiveEventLoop,
	window::{Fullscreen, Window},
};

use crate::{
	coords::{text_box, Dimensions, Rect, RectI},
	game::{Config, Game, GameInfo, MenuChoice},
	gameplay::{Enemy, EnemyType, Player, ProjType, Projectile, World},
};

#[derive(Debug)]
pub struct DrawConstants {
	interface_begin4: u32,
	pub sizes: [Dimensions<u32>; 3],
}

pub const DRAW_CONSTANTS: DrawConstants = DrawConstants {
	interface_begin4: 3,
	sizes: [
		Dimensions { w: 1280, h: 720 },
		Dimensions { w: 1600, h: 900 },
		Dimensions { w: 1920, h: 1080 },
	],
};

pub const N_SIZES: u8 = DRAW_CONSTANTS.sizes.len() as u8;

#[derive(Debug)]
struct ColorPalette {
	bg: [u8; 4],
	bg_ui: [u8; 4],
	menu_select: [u8; 4],
	menu_text: [u8; 4],
}

const COLORS: ColorPalette = ColorPalette {
	bg: [0x08, 0x0b, 0x1e, 0xff],
	bg_ui: [0x20, 0x11, 0x38, 0xff],
	menu_select: [0xff, 0x00, 0x00, 0xff],
	menu_text: [0xff, 0xff, 0xff, 0xff],
};

#[derive(Debug)]
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

pub const CHAR_DIMS: Dimensions<u32> = Dimensions { w: 4, h: 6 };

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

pub fn create_window(event_loop: &ActiveEventLoop) -> Window {
	let win_size = PhysicalSize::new(DRAW_CONSTANTS.sizes[1].w, DRAW_CONSTANTS.sizes[1].h);
	let window_attributes = Window::default_attributes()
		.with_title("Holy Bullet Hell")
		.with_inner_size(win_size)
		.with_resizable(false)
		.with_fullscreen(None)
		// Window is on the top left corner
		.with_position(PhysicalPosition::new(0, 0));
	event_loop.create_window(window_attributes).unwrap()
}

#[derive(Debug)]
pub struct FrameBuffer {
	pub buffer: Pixels,
	pub dims: Dimensions<u32>,
}

impl FrameBuffer {
	pub fn new(window: &Window) -> Self {
		let dims: Dimensions<u32> = window.inner_size().into();
		let bg_color_wgpu: pixels::wgpu::Color = {
			pixels::wgpu::Color {
				r: conv_srgb_to_linear(COLORS.bg[0] as f64 / 255.0),
				g: conv_srgb_to_linear(COLORS.bg[1] as f64 / 255.0),
				b: conv_srgb_to_linear(COLORS.bg[2] as f64 / 255.0),
				a: conv_srgb_to_linear(COLORS.bg[3] as f64 / 255.0),
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

	fn resize_buffer(&mut self, size: &PhysicalSize<u32>) -> Result<(), TextureError> {
		// Resize the window surface
		self.buffer.resize_surface(size.width, size.height)?;
		// Resize the pixel buffer
		self.buffer.resize_buffer(size.width, size.height)?;
		// Update the dimensions
		self.dims = (*size).into();
		Ok(())
	}

	fn fill_with_color(&mut self, color: [u8; 4]) {
		self
			.buffer
			.frame_mut()
			.chunks_exact_mut(4)
			.for_each(|pixel| pixel.copy_from_slice(&color));
	}

	fn iter_pixel_mut(&mut self) -> impl Iterator<Item = &mut [u8]> {
		self.buffer.frame_mut().chunks_exact_mut(4)
	}
}

pub trait ResizableWindow {
	fn request_window_resize(&mut self, index: u8) -> PhysicalSize<u32>;
}

impl ResizableWindow for Window {
	fn request_window_resize(&mut self, index: u8) -> PhysicalSize<u32> {
		// Last entry must be the screen size
		if index == N_SIZES - 1 {
			let fs = Fullscreen::Borderless(self.current_monitor());
			self.set_fullscreen(Some(fs));
		} else {
			self.set_fullscreen(None);
		}
		let size: PhysicalSize<u32> = DRAW_CONSTANTS.sizes[index as usize].into();
		let _ = self.request_inner_size(size);
		size
	}
}

impl Game {
	pub fn redraw(&mut self) {
		self.window.request_redraw();
	}

	pub fn resize(&mut self, size: &PhysicalSize<u32>) {
		self.frame_buffer.resize_buffer(size).unwrap();
		self.config.scale4 = 4 * size.width / DRAW_CONSTANTS.sizes[0].w;
	}

	pub fn render(&mut self) {
		self.frame_buffer.buffer.render().unwrap();
	}

	pub fn draw_in_game(&mut self) {
		self.frame_buffer.fill_with_color(COLORS.bg);
		let world = &mut self.world.as_mut().unwrap();

		world.draw_gameplay(&mut self.frame_buffer, &self.sheets, self.config.scale4);
		world.draw_interface(
			&mut self.frame_buffer,
			&self.sheets,
			&self.config,
			&self.infos,
		);
	}

	fn draw_menu_entry(
		&mut self,
		text: &str,
		text_scale: (i32, i32),
		mut dst: Point2<i32>,
		selected: bool,
	) {
		let text = text.to_uppercase();
		let text_dims = text_box(text.len(), 4) * text_scale;
		// Centers text
		dst.x -= text_dims.w / 2;
		let color = if selected {
			COLORS.menu_select
		} else {
			COLORS.menu_text
		};

		draw_text(
			&mut self.frame_buffer,
			&self.sheets.font,
			Rect { top_left: dst, dims: text_dims },
			color,
			&text,
		);
	}

	pub fn draw_menu(&mut self, choice: MenuChoice) {
		// Background color
		self.frame_buffer.fill_with_color(COLORS.bg);

		// Base positions for the menu entries
		let (base_x, base_y, title_y) = {
			let frame_buffer_dims = self.frame_buffer.dims;
			let win_w = frame_buffer_dims.w;
			let win_h = frame_buffer_dims.h;
			(win_w as i32 / 2, win_h as i32 / 2, win_h as i32 / 10)
		};

		match choice {
			// Main menu
			MenuChoice::Play | MenuChoice::Quit | MenuChoice::Options => {
				self.draw_menu_entry("Holy Bullet Hell", (5, 5), (base_x, title_y).into(), false);

				self.draw_menu_entry(
					"Start",
					(3, 3),
					(base_x, base_y).into(),
					choice == MenuChoice::Play,
				);
				self.draw_menu_entry(
					"Options",
					(3, 3),
					(base_x, base_y + 100).into(),
					choice == MenuChoice::Options,
				);
				self.draw_menu_entry(
					"Quit",
					(3, 3),
					(base_x, base_y + 200).into(),
					choice == MenuChoice::Quit,
				);
			},
			// Level selection menu
			MenuChoice::Level(id) => {
				self.draw_menu_entry("Level Selection", (5, 5), (base_x, title_y).into(), false);
				// Gets the level list while dropping the mutable borrowing of `self`
				let level_list: Vec<(u32, Rc<String>)> =
					self.levels.iter().map(|x| (x.id, x.name.clone())).collect();

				for (i, entry) in level_list.iter().enumerate() {
					self.draw_menu_entry(
						&entry.1,
						(3, 3),
						(base_x, base_y + 100 * i as i32).into(),
						id == i as u16,
					);
				}
			},
			// Options menu
			MenuChoice::Resolution => {
				self.draw_menu_entry("Resolution", (5, 5), (base_x, title_y).into(), false);

				let res_choice = self.config.resolution_choice;
				for (i, res) in DRAW_CONSTANTS.sizes.iter().enumerate() {
					self.draw_menu_entry(
						&format!("{:4} X {:4}", res.w, res.h),
						(3, 3),
						(base_x, base_y + 100 * i as i32).into(),
						res_choice == i as u8,
					);
				}
			},
		}
	}
}

macro_rules! opacity {
	($color: expr, $bg: expr, $alpha:expr, $index: literal) => {
		($alpha * ($color[$index] as f32) + (1. - $alpha) * ($bg[$index] as f32)).round() as u8
	};
}

pub fn draw_rect(frame_buffer: &mut FrameBuffer, dst: RectI, mut color: [u8; 4]) {
	let frame_buffer_dims = frame_buffer.dims;
	// Transparent
	if color[3] == 0x00 {
		return;
	}
	let window = frame_buffer_dims.into_rect();
	for coords in dst.iter() {
		if window.contains(coords) {
			let pixel_index = coords.y * frame_buffer_dims.w as i32 + coords.x;
			let pixel_byte_index = pixel_index as usize * 4;
			let pixel_bytes = pixel_byte_index..(pixel_byte_index + 4);
			if color[3] != 0xff {
				let old_color = frame_buffer
					.buffer
					.frame_mut()
					.get(pixel_bytes.clone())
					.unwrap();
				let alpha = color[3] as f32 / 255.;
				color[0] = opacity!(color, old_color, alpha, 0);
				color[1] = opacity!(color, old_color, alpha, 1);
				color[2] = opacity!(color, old_color, alpha, 2);
				color[3] = 0xff;
			}
			frame_buffer.buffer.frame_mut()[pixel_bytes].copy_from_slice(&color);
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

#[derive(Clone, Debug)]
struct SpriteCoords {
	sheet_pos: Point2<u32>,
	dims: Dimensions<u32>,
}

fn draw_text(
	frame_buffer: &mut FrameBuffer,
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
			font_sheet,
			SpriteCoords { sheet_pos: char_position(c).unwrap().into(), dims: (4, 6).into() },
			dst_c,
			Some(color),
		);
	}
}

fn draw_sprite(
	frame_buffer: &mut FrameBuffer,
	sheet: &DynamicImage,
	SpriteCoords { sheet_pos, dims }: SpriteCoords,
	dst: RectI,
	color: Option<[u8; 4]>,
) {
	let frame_buffer_dims = frame_buffer.dims;
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
			let background = frame_buffer
				.buffer
				.frame_mut()
				.get(pixel_bytes.clone())
				.unwrap();
			let alpha = px[3] as f32 / 255.;
			px[0] = opacity!(px, background, alpha, 0);
			px[1] = opacity!(px, background, alpha, 1);
			px[2] = opacity!(px, background, alpha, 2);
			px[3] = 0xff;
		}
		frame_buffer.buffer.frame_mut()[pixel_bytes].copy_from_slice(&px);
	}
}

impl Player {
	fn sprite_coords(&self) -> SpriteCoords {
		SpriteCoords {
			sheet_pos: if self.immunity_over() { (1, 0) } else { (1, 1) }.into(),
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
	pub fn draw_gameplay(&self, frame_buffer: &mut FrameBuffer, sheets: &Sheets, scale4: u32) {
		let scale = scale4 as f32 / 4.;
		// Player
		let player = &self.player;
		draw_sprite(
			frame_buffer,
			&sheets.spritesheet,
			player.sprite_coords(),
			Rect::from_float_scale(player.pos, player.size, scale),
			None,
		);
		// Player hitbox
		draw_sprite(
			frame_buffer,
			&sheets.spritesheet,
			player.sprite_coords_hit(),
			Rect::from_float_scale(player.pos, player.hitbox.dims, scale),
			None,
		);

		// Enemies
		for enemy in self.enemies.iter() {
			draw_sprite(
				frame_buffer,
				&sheets.spritesheet,
				enemy.sprite_coords(),
				Rect::from_float_scale(enemy.pos, enemy.size, scale),
				None,
			);
			draw_rect(
				frame_buffer,
				Rect::life_bar_full(enemy.pos, enemy.size).scale4(scale4),
				[0xff, 0x00, 0x00, 0xff],
			);
			draw_rect(
				frame_buffer,
				Rect::life_bar(
					enemy.pos,
					enemy.size,
					enemy.hp / Enemy::max_hp(enemy.variant),
				)
				.scale4(scale4),
				[0x00, 0xff, 0x00, 0xff],
			);
		}

		//projectiles
		for proj in self.projectiles.iter() {
			draw_sprite(
				frame_buffer,
				&sheets.spritesheet,
				proj.sprite_coords(),
				Rect::from_float_scale(proj.pos, Dimensions { w: 10., h: 10. }, scale),
				None,
			);
		}
	}

	pub fn draw_interface(
		&self,
		frame_buffer: &mut FrameBuffer,
		sheets: &Sheets,
		config: &Config,
		infos: &GameInfo,
	) {
		let frame_buffer_dims = frame_buffer.dims;
		let win_w = frame_buffer_dims.w;
		let interf_begin_x = DRAW_CONSTANTS.interface_begin4 * win_w / 4;
		let scale4 = config.scale4;
		// Interface background
		frame_buffer
			.iter_pixel_mut()
			.enumerate()
			.for_each(|(i, pixel)| {
				if i as u32 % win_w > interf_begin_x {
					pixel.copy_from_slice(&COLORS.bg_ui)
				}
			});
		// HP
		for i in 0..self.player.hp {
			draw_rect(
				frame_buffer,
				Rect {
					top_left: ((20 + 60 * i) as i32, 120).into(),
					dims: (40, 40).into(),
				}
				.to_interface(interf_begin_x as i32, scale4),
				[0x11, 0x81, 0x0c, 0xff],
			)
		}

		const TEXT_SCALE: u32 = 4;
		// Use base window size for interface to scale
		let win_w = DRAW_CONSTANTS.sizes[0].w as i32;

		let fps_str = format!("FPS: {fps:3}", fps = infos.fps);
		let text_dims = text_box(fps_str.len(), TEXT_SCALE);

		draw_text(
			frame_buffer,
			&sheets.font,
			Rect { top_left: (win_w - text_dims.w, 12).into(), dims: text_dims }
				.to_interface(0, scale4),
			[0xff, 0xff, 0xff, 0xb0],
			&fps_str,
		);

		let score_str = format!("SCORE: {score:3}", score = self.score);
		let score_dims = text_box(score_str.len(), TEXT_SCALE);
		draw_text(
			frame_buffer,
			&sheets.font,
			Rect { top_left: (win_w - score_dims.w, 60).into(), dims: score_dims }
				.to_interface(0, scale4),
			[0xff, 0xff, 0xff, 0xb0],
			&score_str,
		);

		let level_name = "LEVEL 1";
		draw_text(
			frame_buffer,
			&sheets.font,
			Rect {
				top_left: (20, 200).into(),
				dims: text_box(level_name.len(), 2 * TEXT_SCALE),
			}
			.to_interface(interf_begin_x as i32, scale4),
			[0xff, 0x00, 0x00, 0xff],
			level_name,
		);
	}
}

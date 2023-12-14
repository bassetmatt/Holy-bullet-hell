use std::{
	fs,
	path::Path,
	time::{Duration, Instant},
};
use winit::{
	event::{ElementState, VirtualKeyCode},
	event_loop::{ControlFlow, EventLoop},
	window::Window,
};

use crate::coords::Dimensions;
use crate::draw::{create_window, FrameBuffer, Sheets};
use crate::gameplay::{Cooldown, EnemyType, Event, EventType, World};

enum GameState {
	_Playing,
	_Paused,
	_Menu(MenuChoice),
	_GameOver,
}

enum MenuChoice {
	_Play,
	_Options(OptionChoice),
	_Quit,
}

enum OptionChoice {
	_Resolution(u8),
	_Fullscreen(bool),
	_Back,
}

pub struct GameOptions {
	pub resolution_choice: u8,
	pub _fullscreen: bool,
}

impl GameOptions {
	fn new() -> GameOptions {
		GameOptions { resolution_choice: 2, _fullscreen: false }
	}

	// ? May be used when using old options saved somewhere
	fn _new_from_args(resolution_choice: u8, _fullscreen: bool) -> GameOptions {
		GameOptions { resolution_choice, _fullscreen }
	}
}

struct Level {
	_id: u32,
	name: String,
	event_list: Vec<Event>,
}

pub const LEVEL_REF: u32 = u32::MAX;
impl Level {
	fn level_from_file(game: &mut Game, level_file: &str) {
		let level_raw_data = fs::read_to_string(level_file).unwrap();
		let mut level = Level {
			_id: game.levels.len() as u32,
			event_list: vec![],
			name: String::new(),
		};

		let meta_data = level_raw_data
			.split('\n')
			.filter_map(|x| x.strip_prefix('$'));

		for data in meta_data {
			let data = data.split_once(char::is_whitespace).unwrap();
			match data.0 {
				"title" => {
					level.name = data.1.into();
				},
				data => {
					unimplemented!("'{data}' keyword doesn't exist")
				},
			}
		}

		let events = level_raw_data
			.split('\n')
			.filter_map(|x| x.strip_prefix('@'));
		let id: u32 = 0;
		for event in events {
			let mut event = event.split_whitespace();
			match event.next().unwrap() {
				"spawn-enemy" => {
					let variant = match event.next().unwrap() {
						"basic" => EnemyType::Basic,
						"sniper" => EnemyType::Sniper,
						other => unimplemented!("Enemy type '{other}' doesn't exist"),
					};
					let t: f32 = event.next().unwrap().parse().unwrap();
					let t = Duration::from_secs_f32(t);
					let x: f32 = event.next().unwrap().parse().unwrap();
					let y: f32 = event.next().unwrap().parse().unwrap();
					let ref_evt = event.next().unwrap().parse::<u32>().ok().map(|x| (x, t));
					let variant = EventType::_SpawnEnemy((x, y).into(), variant);
					// Events are all relative, the "absolute" events will be relative to the beginning of the level
					let evt = match ref_evt {
						Some(_) => Event { id, time: None, variant, ref_evt },
						None => Event { id, time: None, variant, ref_evt: Some((LEVEL_REF, t)) },
					};
					level.event_list.push(evt);
				},
				evt => unimplemented!("Unknown event '{evt}'"),
			}
		}
		game.levels.push(level);
	}
}

#[derive(Default)]
pub struct Inputs {
	pub left: bool,
	pub right: bool,
	pub up: bool,
	pub down: bool,
	pub shoot: bool,
	pub _pause: bool,
}

impl Inputs {
	fn new() -> Inputs {
		Inputs { ..Default::default() }
	}
}

pub struct FpsCounter {
	pub fps: u32,
	cooldown: Cooldown,
}

pub struct GlobalInfo {
	_game_begin: Instant,
	_level_begin: Option<Instant>,
	frame_count: u64,
	pub fps_info: FpsCounter,
}

impl GlobalInfo {
	fn new() -> GlobalInfo {
		GlobalInfo {
			_game_begin: Instant::now(),
			_level_begin: None,
			frame_count: 0,
			fps_info: FpsCounter {
				fps: 0,
				cooldown: Cooldown::with_duration(Duration::from_millis(100)),
			},
		}
	}

	fn start_level(&mut self) {
		self._level_begin = Some(Instant::now());
	}

	pub fn update(&mut self) {
		self.frame_count += 1;
	}

	pub fn _since_game_begin(&self) -> Duration {
		Instant::elapsed(&self._game_begin)
	}

	pub fn _since_level_begin(&self) -> Duration {
		Instant::elapsed(&self._level_begin.unwrap())
	}
}

pub struct Game {
	_state: GameState,
	pub options: GameOptions,
	pub world: Option<World>,
	levels: Vec<Level>,
	inputs: Inputs,
	pub infos: GlobalInfo,
	pub window: Window,
	pub frame_buffer: FrameBuffer,
	pub sheets: Sheets,
}

impl Game {
	pub fn launch(event_loop: &EventLoop<()>) -> Game {
		env_logger::init();
		let window = create_window(event_loop);
		Game {
			_state: GameState::_Playing,
			options: GameOptions::new(),
			world: None,
			levels: vec![],
			inputs: Inputs::new(),
			infos: GlobalInfo::new(),
			frame_buffer: FrameBuffer::new(&window),
			window,
			sheets: Sheets::load(),
		}
	}

	pub fn load_levels(&mut self) {
		let level_dir: &Path = Path::new("./levels");
		if !level_dir.exists() {
			panic!("Levels directory doesn't exist");
		}
		for level in fs::read_dir(level_dir).unwrap() {
			let path = level.unwrap().path();
			if path.is_file() && path.extension().is_some_and(|ext| ext == "hbh") {
				Level::level_from_file(self, path.to_str().unwrap());
			}
		}
	}

	pub fn process_input(&mut self, state: &ElementState, key: &VirtualKeyCode) {
		match key {
			VirtualKeyCode::Up => self.inputs.up = matches!(state, ElementState::Pressed),
			VirtualKeyCode::Down => self.inputs.down = matches!(state, ElementState::Pressed),
			VirtualKeyCode::Left => self.inputs.left = matches!(state, ElementState::Pressed),
			VirtualKeyCode::Right => self.inputs.right = matches!(state, ElementState::Pressed),
			VirtualKeyCode::X => self.inputs.shoot = matches!(state, ElementState::Pressed),
			_ => {},
		}
	}

	pub fn start_level(&mut self, id: u32) {
		self.infos.start_level();
		let dims = self.frame_buffer.dims;
		let new_world = World::start(
			Dimensions { w: (0.8 * dims.w as f32), h: dims.h as f32 },
			self.levels.get(id as usize).unwrap().event_list.clone(),
		);
		self.world = Some(new_world);
	}

	pub fn tick(&mut self, dt: Duration, control_flow: &mut ControlFlow) {
		let world = &mut self.world.as_mut().unwrap();
		// Applying events
		world.process_events();
		// Main physics calculations
		world.update_entities(dt, &self.inputs);
		// Projectiles physics
		world.update_projectiles(dt);
		// Checks end condition
		world.check_end(control_flow);
	}
	pub fn update_fps(&mut self, dt: Duration) {
		// Limit fps refresh for it to be readable
		let fps_infos = &mut self.infos.fps_info;
		if fps_infos.cooldown.is_over() {
			fps_infos.fps = (1. / dt.as_secs_f64()).round() as u32;
			fps_infos.cooldown.emit();
		}
	}
}

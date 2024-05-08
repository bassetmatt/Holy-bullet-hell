use smol_str::SmolStr;
use std::{
	fs,
	path::Path,
	time::{Duration, Instant},
};
use winit::{event::ElementState, event_loop::ActiveEventLoop, keyboard::Key, window::Window};

use crate::{
	coords::Dimensions,
	draw::{create_window, FrameBuffer, ResizableWindow, Sheets, DRAW_CONSTANTS},
	gameplay::{Cooldown, EnemyType, Event, EventType, World},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RunState {
	Playing,
	_Paused,
	Menu(MenuChoice),
	_GameOver,
	Quitting,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum MenuChoice {
	// Main menu
	Play,
	Options,
	Quit,
	// Play menu
	// Id of the level
	Level(u16),
	// Options menu
	Resolution,
}

pub struct Level {
	pub id: u32,
	pub name: String,
	event_list: Vec<Event>,
}

pub const LEVEL_REF: u32 = u32::MAX;
impl Level {
	fn level_from_file(game: &mut Game, level_file: &str) {
		let level_raw_data = fs::read_to_string(level_file).unwrap();
		let mut level = Level {
			id: game.levels.len() as u32,
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

pub struct Config {
	pub resolution_choice: u8,
	pub _fullscreen: bool,
	/// Four times the scaling factor to avoid floating point operations
	pub scale4: u32,
}

impl Config {
	fn new() -> Config {
		Config { resolution_choice: 1, _fullscreen: false, scale4: 4 }
	}
}

pub struct GameInfo {
	_game_begin: Instant,
	_level_begin: Option<Instant>,
	frame_count: u64,
	pub fps: u32,
	fps_cooldown: Cooldown,
	pub dt: Duration,
	pub t: Instant,
}

impl GameInfo {
	fn new() -> GameInfo {
		GameInfo {
			_game_begin: Instant::now(),
			_level_begin: None,
			frame_count: 0,
			fps: 0,
			fps_cooldown: Cooldown::with_secs(0.1),
			dt: Duration::from_secs(1),
			t: Instant::now(),
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
	pub state: RunState,
	pub world: Option<World>,
	pub inputs: Inputs,
	pub window: Window,
	pub frame_buffer: FrameBuffer,
	pub sheets: Sheets,
	pub levels: Vec<Level>,
	pub config: Config,
	pub infos: GameInfo,
}

impl Game {
	pub fn launch(event_loop: &ActiveEventLoop) -> Game {
		env_logger::init();
		let window = create_window(event_loop);
		Game {
			state: RunState::Menu(MenuChoice::Play),
			world: None,
			inputs: Inputs::new(),
			frame_buffer: FrameBuffer::new(&window),
			window,
			sheets: Sheets::load(),
			levels: vec![],
			config: Config::new(),
			infos: GameInfo::new(),
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
		// Sort inversely by id
		// TODO: Have better sorting function?
		self.levels.sort_by_key(|x| u32::MAX - x.id);
	}

	fn menu_key_handling(&mut self, key_state: &ElementState, key: &Key) {
		use winit::keyboard::NamedKey::*;
		if key_state == &ElementState::Released {
			return;
		}
		let menu_choice = match self.state {
			RunState::Menu(choice) => choice,
			_ => panic!("Not in menu state"),
		};
		let is_main_menu = matches!(
			menu_choice,
			MenuChoice::Play | MenuChoice::Options | MenuChoice::Quit
		);
		match key {
			Key::Named(Escape) => {
				if is_main_menu {
					self.state = RunState::Menu(MenuChoice::Quit);
				}
				if menu_choice == MenuChoice::Resolution {
					self.state = RunState::Menu(MenuChoice::Options);
				} else if matches!(menu_choice, MenuChoice::Level(_)) {
					self.state = RunState::Menu(MenuChoice::Play);
				}
			},
			Key::Named(ArrowDown) => {
				if is_main_menu {
					self.state = RunState::Menu(match menu_choice {
						MenuChoice::Play => MenuChoice::Options,
						MenuChoice::Options => MenuChoice::Quit,
						MenuChoice::Quit => MenuChoice::Play,
						_ => panic!("Invalid menu choice"),
					})
				} else if let MenuChoice::Level(id) = menu_choice {
					let id = (id + 1) % self.levels.len() as u16;
					self.state = RunState::Menu(MenuChoice::Level(id));
				} else if menu_choice == MenuChoice::Resolution {
					let res_choice = &mut self.config.resolution_choice;
					*res_choice = (*res_choice + 1) % DRAW_CONSTANTS.sizes.len() as u8;
					self.window.request_window_resize(*res_choice);
				}
			},
			Key::Named(ArrowUp) => {
				if is_main_menu {
					self.state = RunState::Menu(match menu_choice {
						MenuChoice::Play => MenuChoice::Quit,
						MenuChoice::Options => MenuChoice::Play,
						MenuChoice::Quit => MenuChoice::Options,
						_ => panic!("Invalid menu choice"),
					})
				} else if let MenuChoice::Level(id) = menu_choice {
					let id = (id - 1) % self.levels.len() as u16;
					self.state = RunState::Menu(MenuChoice::Level(id));
				} else if menu_choice == MenuChoice::Resolution {
					let res_choice = &mut self.config.resolution_choice;
					*res_choice = (*res_choice - 1) % DRAW_CONSTANTS.sizes.len() as u8;
					self.window.request_window_resize(*res_choice);
				}
			},
			Key::Named(Enter) => match menu_choice {
				MenuChoice::Play => {
					self.state = RunState::Menu(MenuChoice::Level(0));
				},
				MenuChoice::Options => {
					self.state = RunState::Menu(MenuChoice::Resolution);
				},
				MenuChoice::Quit => {
					self.state = RunState::Quitting;
				},
				MenuChoice::Level(id) => {
					self.state = RunState::Playing;
					self.start_level(id as u32);
				},
				MenuChoice::Resolution => {
					self.state = RunState::Menu(MenuChoice::Options);
				},
			},
			_ => {},
		}
	}
	pub fn process_input(&mut self, key_state: &ElementState, key: &Key) {
		use winit::keyboard::NamedKey::*;
		// TODO: Some day, use data structures for keys

		if matches!(self.state, RunState::Menu(_)) {
			self.menu_key_handling(key_state, key);
		}
		match key {
			Key::Named(ArrowUp) => self.inputs.up = matches!(key_state, ElementState::Pressed),
			Key::Named(ArrowDown) => self.inputs.down = matches!(key_state, ElementState::Pressed),
			Key::Named(ArrowLeft) => self.inputs.left = matches!(key_state, ElementState::Pressed),
			Key::Named(ArrowRight) => self.inputs.right = matches!(key_state, ElementState::Pressed),
			Key::Character(key) if key == &SmolStr::new("x") => {
				self.inputs.shoot = matches!(key_state, ElementState::Pressed)
			},
			_ => {},
		}
	}

	pub fn start_level(&mut self, id: u32) {
		self.infos.start_level();
		// The wolrd size is fixed as the lowest resolution and the graphics are scaled up
		let new_world = World::start(
			Dimensions {
				w: DRAW_CONSTANTS.sizes[0].w as f32 * 0.75,
				h: DRAW_CONSTANTS.sizes[0].h as f32,
			},
			self.levels.get(id as usize).unwrap().event_list.clone(),
		);
		self.world = Some(new_world);
	}

	pub fn tick(&mut self, event_loop: &ActiveEventLoop) {
		// TODO: Maybe better assignment of world?
		// Applying events
		{
			let world = self.world.as_mut().unwrap();
			world.process_events();
		}
		// Projectiles physics
		self.update_projectiles();
		// Main physics calculations
		self.update_entities();
		// Checks end condition
		{
			let world = self.world.as_mut().unwrap();
			world.check_end(event_loop);
		}
	}

	pub fn update_fps(&mut self) {
		// Limit fps refresh for it to be readable
		if self.infos.fps_cooldown.is_over() {
			self.infos.fps = (1. / self.infos.dt.as_secs_f64()).round() as u32;
			self.infos.fps_cooldown.reset();
		}
	}
}

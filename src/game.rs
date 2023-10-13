use crate::{
	coords::Dimensions,
	draw::{create_window, FrameBuffer},
	gameplay::{Cooldown, EnemyType, Event, EventType, World},
};
use std::{
	fs,
	path::Path,
	time::{Duration, Instant},
};
use winit::{
	event::{ElementState, VirtualKeyCode},
	event_loop::EventLoop,
	window::{self, Window},
};

enum GameState {
	Menu(MenuChoice),
	Playing,
	GameOver,
}

enum MenuChoice {
	Play,
	Options(OptionChoice),
	Quit,
}

enum OptionChoice {
	Resolution,
	Fullscreen,
	Back,
}

struct Level {
	id: u32,
	name: String,
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
			let mut data = data.split_once(char::is_whitespace).unwrap();
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
					let variant = EventType::SpawnEnemy((x, y).into(), variant);
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

struct FpsCounter {
	fps: u32,
	cooldown: Cooldown,
}

pub struct GlobalInfo {
	game_begin: Instant,
	level_begin: Option<Instant>,
	frame_count: u64,
	fps_info: FpsCounter,
}

impl GlobalInfo {
	fn new() -> GlobalInfo {
		GlobalInfo {
			game_begin: Instant::now(),
			level_begin: None,
			frame_count: 0,
			fps_info: FpsCounter {
				fps: 0,
				cooldown: Cooldown::with_duration(Duration::from_millis(100)),
			},
		}
	}

	fn start_level(&mut self) {
		self.level_begin = Some(Instant::now());
	}

	pub fn update(&mut self) {
		self.frame_count += 1;
	}

	pub fn since_game_begin(&self) -> Duration {
		Instant::elapsed(&self.game_begin)
	}

	pub fn since_level_begin(&self) -> Duration {
		Instant::elapsed(&self.level_begin.unwrap())
	}
}

pub struct Game {
	state: GameState,
	pub world: Option<World>,
	levels: Vec<Level>,
	inputs: Inputs,
	pub infos: GlobalInfo,
	pub window: Window,
	pub frame_buffer: FrameBuffer,
}

impl Game {
	pub fn launch(event_loop: EventLoop<()>) -> Game {
		env_logger::init();
		let mut window = create_window(&event_loop);
		Game {
			state: GameState::Menu(MenuChoice::Play),
			world: None,
			levels: vec![],
			inputs: Inputs::new(),
			infos: GlobalInfo::new(),
			window,
			frame_buffer: FrameBuffer::new(&window),
		}
	}

	pub fn load_levels(&mut self) {
		const LEVEL_DIR: &Path = Path::new("./levels");
		if !LEVEL_DIR.exists() {
			panic!("Levels directory doesn't exist");
		}
		for level in fs::read_dir(LEVEL_DIR).unwrap() {
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

	fn start_level(&mut self, id: u32) {
		self.infos.start_level();
		let dims = self.frame_buffer.dims;
		let new_world = World::start(
			Dimensions { w: (0.8 * dims.w as f32), h: dims.h as f32 },
			self.levels.get(id as usize).unwrap().event_list.clone(),
		);
		self.world = Some(new_world);
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

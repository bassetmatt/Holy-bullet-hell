use std::{
	fs,
	time::{Duration, Instant},
};

use crate::{
	coords::Dimensions,
	gameplay::{EnemyType, Event, EventType, World},
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

impl Level {
	fn push_event(&mut self, t: Duration, event: EventType) {
		self
			.event_list
			// TODO: Redo the whole time thing
			.push(Event { time: self.infos.begin + t, variant: event });
	}

	fn level_from_file(game: Game, level_file: &str) -> Level {
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
					// TODO: Put this in push event
					let evt = match ref_evt {
						Some(thing) => Event { id, time: None, variant, ref_evt: Some(thing) },
						None => Event { id, time: None, variant, ref_evt: Some((u32::MAX, t)) },
					};
					level.event_list.push(evt);
				},
				evt => unimplemented!("Unknown event '{evt}'"),
			}
		}
		level
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

pub struct GlobalInfo {
	game_begin: Instant,
	level_begin: Option<Instant>,
	frame_count: u64,
}

impl GlobalInfo {
	fn new() -> GlobalInfo {
		GlobalInfo { game_begin: Instant::now(), level_begin: None, frame_count: 0 }
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
	world: Option<World>,
	levels: Vec<Level>,
	inputs: Inputs,
	infos: GlobalInfo,
}

use winit::event::{ElementState, VirtualKeyCode};
impl Game {
	pub fn launch() -> Game {
		Game {
			state: GameState::Menu(MenuChoice::Play),
			world: None,
			inputs: Inputs::new(),
		}
	}
	pub fn process_input(&mut self, state: ElementState, key: VirtualKeyCode) {
		match key {
			VirtualKeyCode::Up => self.inputs.up = matches!(state, ElementState::Pressed),
			VirtualKeyCode::Down => self.inputs.down = matches!(state, ElementState::Pressed),
			VirtualKeyCode::Left => self.inputs.left = matches!(state, ElementState::Pressed),
			VirtualKeyCode::Right => self.inputs.right = matches!(state, ElementState::Pressed),
			VirtualKeyCode::X => self.inputs.shoot = matches!(state, ElementState::Pressed),
			_ => {},
		}
	}
	fn start_level(&mut self, level: Level) -> World {
		return World::start(Dimensions { w: 0.8 * dimensions.w, h: dimensions.h });
	}
}

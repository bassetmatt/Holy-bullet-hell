use std::{fs, time::Duration};

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
	event_list: Vec<Event>,
}

impl Level {
	fn push_event(&mut self, t: Duration, event: EventType) {
		self
			.event_list
			// TODO: Redo the whole time thing
			.push(Event { time: self.infos.begin + t, variant: event });
	}
	fn load_level(level_file: &str) -> Level {
		let level_raw_data = fs::read_to_string(level_file).unwrap();

		let events = level_raw_data
			.split('\n')
			.filter_map(|x| x.strip_prefix('@'));

		let level = Level { event_list: vec![] };

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
					level.push_event(t, event);
				},
				evt => unimplemented!("Unknown event {evt}"),
			}
		}
		Ok(world)
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

pub struct Game {
	state: GameState,
	world: Option<World>,
	inputs: Inputs,
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

use cgmath::{InnerSpace, Point2, Vector2, Zero};
use std::{
	collections::HashMap,
	time::{Duration, Instant},
};
use winit::event_loop::ActiveEventLoop;

use crate::{
	coords::{collide_rectangle, CenteredBox, Dimensions, RectF},
	game::{Game, Inputs},
};

pub const DT_60: f32 = 1. / 60.;
#[derive(Clone, Debug)]
pub struct Cooldown {
	last_emit: Option<Instant>,
	cooldown: Duration,
}

impl Cooldown {
	/// Creates cooldown with secs second duration
	pub fn with_secs(secs: f32) -> Self {
		Cooldown { last_emit: None, cooldown: Duration::from_secs_f32(secs) }
	}

	#[allow(dead_code)]
	pub fn with_duration(value: Duration) -> Self {
		Cooldown { last_emit: None, cooldown: value }
	}

	pub fn is_over(&self) -> bool {
		if let Some(last) = self.last_emit {
			return Instant::elapsed(&last) >= self.cooldown;
		}
		true
	}

	pub fn reset(&mut self) {
		self.last_emit = Some(Instant::now());
	}
}

#[derive(Clone, Debug)]
pub struct Player {
	pub pos: Point2<f32>,
	vel: Vector2<f32>,
	pub size: Dimensions<f32>,
	pub hitbox: CenteredBox,
	pub hp: u32,
	immunity: Cooldown,
	new_shoot: Cooldown,
}

impl Player {
	fn new() -> Self {
		Self {
			pos: (75., 200.).into(),
			hitbox: CenteredBox { center: (75., 200.).into(), dims: (12., 12.).into() },
			vel: (0., 0.).into(),
			size: Dimensions { w: 48., h: 48. },
			hp: 5,
			immunity: Cooldown::with_secs(2.),
			new_shoot: Cooldown::with_secs(15. * DT_60),
		}
	}

	pub fn immunity_over(&self) -> bool {
		self.immunity.is_over()
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
			self.hitbox.center = self.pos;
		}
	}
}

#[derive(Clone, Copy, Debug)]
pub enum EnemyType {
	Basic,
	Sniper,
}

#[derive(Clone, Debug)]
enum EnemyState {
	NotSpawned,
	OnScreen(fn(&mut Enemy, RectF)),
	OffScreen,
	Dead,
}

#[derive(Clone, Debug)]
pub struct Enemy {
	pub pos: Point2<f32>,
	vel: Vector2<f32>,
	pub size: Dimensions<f32>,
	pub hp: f32,
	proj_cd: Cooldown,
	pub variant: EnemyType,
	state: EnemyState,
}

impl Enemy {
	fn spawn(pos: Point2<f32>, variant: EnemyType) -> Enemy {
		let (size, proj_cd) = match variant {
			EnemyType::Basic => ((48., 48.).into(), Cooldown::with_secs(25. * DT_60)),
			EnemyType::Sniper => ((32., 48.).into(), Cooldown::with_secs(40. * DT_60)),
		};
		Self {
			pos,
			vel: Vector2::zero(),
			size,
			hp: Self::max_hp(variant),
			proj_cd,
			variant,
			state: EnemyState::NotSpawned,
		}
	}

	pub fn max_hp(variant: EnemyType) -> f32 {
		match variant {
			EnemyType::Basic => 15.,
			EnemyType::Sniper => 8.,
		}
	}

	fn enemy_func(&mut self) -> fn(&mut Enemy, RectF) {
		const SPEED: f32 = 0.5;
		match self.variant {
			EnemyType::Basic => |enemy, bounds| {
				enemy.vel = Vector2::unit_y() * SPEED;
				if enemy.pos.x <= bounds.dims.w / 2. {
					enemy.vel -= Vector2::unit_x() * SPEED;
				} else if enemy.pos.x > bounds.dims.w / 2. {
					enemy.vel += Vector2::unit_x() * SPEED;
				}
			},
			EnemyType::Sniper => |enemy, bounds| {
				let mid_up: Point2<f32> = (bounds.dims.w / 2., 0.).into();
				let to_mid = (mid_up - enemy.pos).normalize();
				// Orthogonal, needs better solution because only one direction works
				enemy.vel = Vector2::new(to_mid.y, -to_mid.x) * SPEED * 5.;
			},
		}
	}

	fn update_pos(&mut self, bounds: RectF, dt: f32) {
		// Enemies behavior
		const SPEED: f32 = 0.5;
		match self.state {
			EnemyState::NotSpawned => {
				self.vel = Vector2::unit_y() * SPEED;
				self.pos += self.vel * dt / DT_60;
				if bounds.contains(self.pos) {
					self.state = EnemyState::OnScreen(self.enemy_func());
				};
			},
			EnemyState::OnScreen(f) => {
				f(self, bounds);
				if !bounds.contains(self.pos) {
					self.state = EnemyState::OffScreen;
				}
			},
			_ => {},
		}
		// Update pos
		if self.vel != Vector2::zero() {
			self.pos += self.vel * dt / DT_60;
		}
	}

	fn get_shot(&mut self, damage: f32) {
		self.hp -= damage;
		if self.hp <= 0. {
			self.state = EnemyState::Dead;
		}
	}
}

#[derive(Clone, Debug)]
pub enum ProjType {
	Basic,
	Aimed,
	PlayerShoot,
}

const PROJ_SIZE: Dimensions<f32> = Dimensions { w: 10., h: 10. };
#[derive(Clone, Debug)]
pub struct Projectile {
	pub pos: Point2<f32>,
	vel: Vector2<f32>,
	pub variant: ProjType,
}

impl Projectile {
	fn damage(&self) -> f32 {
		match self.variant {
			ProjType::Basic => 1.,
			ProjType::Aimed => 1.,
			ProjType::PlayerShoot => 2.,
		}
	}
}

#[derive(Clone, Debug)]
pub enum EventType {
	_SpawnEnemy(Point2<f32>, EnemyType),
	_SpawnBoss(Point2<f32>),
}

#[derive(Clone, Debug)]
pub struct Event {
	pub id: u32,
	pub time: Option<Instant>,
	/// (`id`, `offset`), id of the trigger event, and the duration of the wait after said event is triggered
	pub ref_evt: Option<(u32, Duration)>,
	pub variant: EventType,
}

#[derive(Clone, Debug)]
pub struct EventSystem {
	list: Vec<Event>,
	history: HashMap<u32, Instant>,
	_latest_id: u32,
}

impl EventSystem {
	fn new(evt_list: Vec<Event>) -> Self {
		use crate::game::LEVEL_REF;
		let mut list = vec![];
		for evt in evt_list {
			let mut evt = evt.clone();
			if evt.ref_evt.is_some_and(|(x, _)| x == LEVEL_REF) {
				evt.time = Some(Instant::now() + evt.ref_evt.unwrap().1);
				evt.ref_evt = None;
			}
			list.push(evt);
		}
		Self { list, history: HashMap::new(), _latest_id: 0 }
	}

	fn events_clear(&self) -> bool {
		self.list.is_empty()
	}
}

#[derive(Clone, Debug)]
pub struct World {
	pub player: Player,
	pub projectiles: Vec<Projectile>,
	pub enemies: Vec<Enemy>,
	boundaries: RectF,
	pub score: u64,
	event_syst: EventSystem,
}

impl World {
	/// Create a new `World` instance that can draw a moving box.
	pub fn start(dims: Dimensions<f32>, evt_list: Vec<Event>) -> Self {
		Self {
			player: Player::new(),
			projectiles: Vec::new(),
			enemies: vec![],
			boundaries: dims.into_rect(),
			score: 0,
			event_syst: EventSystem::new(evt_list),
		}
	}

	pub fn check_end(&self, event_loop: &ActiveEventLoop) {
		if self.player.hp == 0 {
			// Goofiest dead message
			println!("Ur so dead 💀, RIP BOZO 🔫🔫😂😂😂😂");
			event_loop.exit();
		}
		if self.enemies.is_empty() && self.event_syst.events_clear() {
			println!("You won! Score: {score}", score = self.score);
			event_loop.exit();
		}
	}

	pub fn process_events(&mut self) {
		let evt_list = &mut self.event_syst.list;
		let map = &mut self.event_syst.history;
		// Checks if absolute events are triggered
		evt_list.retain(|e| {
			if !e.time.is_some_and(|t| Instant::now() >= t) {
				return true;
			}
			match &e.variant {
				EventType::_SpawnEnemy(pos, variant) => {
					self.enemies.push(Enemy::spawn(*pos, *variant));
				},
				var => {
					unimplemented!("Event variant '{var:?}' not implemented")
				},
			}
			map.insert(e.id, Instant::now());
			false
		});
		// Updates relative events to be transformed into absolute events
		for e in evt_list.iter_mut() {
			if let Some((id, t)) = e.ref_evt {
				if map.contains_key(&id) {
					e.ref_evt = None;
					e.time = Some(map[&id] + t);
				}
			}
		}
	}
}

impl Game {
	pub fn update_entities(&mut self) {
		let world = &mut self.world.as_mut().unwrap();
		let dt = self.infos.dt;
		let inputs = &self.inputs;
		// Player
		let player = &mut world.player;
		player.update_pos(inputs, world.boundaries, dt.as_secs_f32());
		// Player shoot
		if inputs.shoot & player.new_shoot.is_over() {
			let proj = Projectile {
				pos: player.pos - player.size.h / 2. * Vector2::unit_y(),
				vel: Vector2::unit_y() * -10.,
				variant: ProjType::PlayerShoot,
			};
			world.projectiles.push(proj);
			player.new_shoot.reset();
		}

		// Enemies physics
		// Updates position
		world.enemies.retain_mut(|enemy| {
			enemy.update_pos(world.boundaries, dt.as_secs_f32());
			// If the enemy is dead, add points
			if matches!(enemy.state, EnemyState::Dead) {
				world.score += 100;
				return false;
			}
			// Removes if offscreen
			!matches!(enemy.state, EnemyState::OffScreen)
		});
		for enemy in world.enemies.iter_mut() {
			// Shooting
			if enemy.proj_cd.is_over() && world.boundaries.contains(enemy.pos) {
				let proj = {
					let pos = enemy.pos + enemy.size.h * 0.6 * Vector2::unit_y();
					match enemy.variant {
						EnemyType::Basic => {
							Projectile { pos, vel: Vector2::unit_y() * 10., variant: ProjType::Basic }
						},
						EnemyType::Sniper => {
							let delta = player.pos - pos;
							let mut to_player = Vector2::zero();
							if delta != Vector2::zero() {
								to_player = delta.normalize();
							}
							Projectile { pos, vel: 10. * to_player, variant: ProjType::Aimed }
						},
					}
				};
				world.projectiles.push(proj);
				enemy.proj_cd.reset();
			}
		}
	}

	pub fn update_projectiles(&mut self) {
		let world = &mut self.world.as_mut().unwrap();
		let player = &mut world.player;

		world.projectiles.retain_mut(|proj| {
			proj.pos += proj.vel * self.infos.dt.as_secs_f32() / DT_60;
			if !world.boundaries.contains(proj.pos) {
				return false;
			}

			for enemy in world.enemies.iter_mut() {
				if matches!(proj.variant, ProjType::PlayerShoot)
					& collide_rectangle(enemy.pos, enemy.size, proj.pos, PROJ_SIZE)
				{
					enemy.get_shot(proj.damage());
					return false;
				}
			}

			if player.immunity.is_over()
				& !matches!(proj.variant, ProjType::PlayerShoot)
				& collide_rectangle(player.pos, player.hitbox.dims, proj.pos, PROJ_SIZE)
			{
				if player.hp > 0 {
					// Avoids underflow if damage is more than 1
					player.hp = player.hp.saturating_sub(proj.damage() as u32)
				}
				if player.hp == 0 {
					return false;
				}

				player.immunity.reset();
				return false;
			}
			true
		});
	}
}

use crate::{
	coords::{Dimensions, PhysicalBox, RectF},
	game::Inputs,
};
use cgmath::{InnerSpace, Point2, Vector2, Zero};
use std::{
	collections::HashMap,
	time::{Duration, Instant},
};
use winit::event_loop::EventLoopWindowTarget;

pub const DT_60: f32 = 1. / 60.;

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

	pub fn emit(&mut self) {
		self.last_emit = Some(Instant::now());
	}
}

pub struct Player {
	pub pos: Point2<f32>,
	vel: Vector2<f32>,
	pub size: Dimensions<f32>,
	pub hitbox: PhysicalBox,
	pub hp: u32,
	immunity: Cooldown,
	new_shoot: Cooldown,
}

impl Player {
	fn new() -> Self {
		Self {
			pos: (75., 200.).into(),
			hitbox: PhysicalBox { center: (75., 200.).into(), dims: (12., 12.).into() },
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

enum EnemyState {
	NotSpawned,
	OnScreen(fn(&mut Enemy, RectF)),
	OffScreen,
}

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
}

pub enum ProjType {
	Basic,
	Aimed,
	PlayerShoot,
}

pub struct Projectile {
	pub pos: Point2<f32>,
	vel: Vector2<f32>,
	pub variant: ProjType,
}

#[derive(Clone, Debug)]
pub enum EventType {
	_SpawnEnemy(Point2<f32>, EnemyType),
	_SpawnBoss(Point2<f32>),
}

#[derive(Clone)]
pub struct Event {
	pub id: u32,
	pub time: Option<Instant>,
	/// (`id`, `offset`), id of the trigger event, and the duration of the wait after said event is triggered
	pub ref_evt: Option<(u32, Duration)>,
	pub variant: EventType,
}

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

pub struct World {
	pub player: Player,
	pub projectiles: Vec<Projectile>,
	pub enemies: Vec<Enemy>,
	rect: RectF,
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
			rect: dims.into_rect(),
			score: 0,
			event_syst: EventSystem::new(evt_list),
		}
	}

	pub fn check_end(&self, evt_loop_target: &EventLoopWindowTarget<()>) {
		if self.player.hp == 0 {
			// Goofiest dead message
			println!("Ur so dead 💀, RIP BOZO 🔫🔫😂😂😂😂");
			evt_loop_target.exit();
		}
		if self.enemies.is_empty() && self.event_syst.events_clear() {
			println!("You won! Score: {score}", score = self.score);
			evt_loop_target.exit();
		}
	}

	pub fn process_events(&mut self) {
		let mut to_remove = vec![];
		let evt_list = &mut self.event_syst.list;
		let map = &mut self.event_syst.history;
		// Checks if absolute events are triggered
		for (i, e) in evt_list.iter().enumerate() {
			if e.time.is_some_and(|t| Instant::now() >= t) {
				match &e.variant {
					EventType::_SpawnEnemy(pos, variant) => {
						self.enemies.push(Enemy::spawn(*pos, *variant));
					},
					var => {
						unimplemented!("Event variant '{var:?}' not implemented")
					},
				}
				map.insert(e.id, Instant::now());
				to_remove.push(i);
			}
		}
		// Removes done events
		for i in to_remove.into_iter().rev() {
			evt_list.remove(i);
		}
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

	pub fn update_entities(&mut self, dt: Duration, inputs: &Inputs) {
		// Player
		let player = &mut self.player;
		player.update_pos(inputs, self.rect, dt.as_secs_f32());
		// Player shoot
		if inputs.shoot & player.new_shoot.is_over() {
			let proj = Projectile {
				pos: player.pos - player.size.h / 2. * Vector2::unit_y(),
				vel: Vector2::unit_y() * -10.,
				variant: ProjType::PlayerShoot,
			};
			self.projectiles.push(proj);
			player.new_shoot.last_emit = Some(Instant::now());
		}

		// Enemies physics
		let mut to_remove = vec![];
		for (i, enemy) in self.enemies.iter_mut().enumerate() {
			// Updates position
			enemy.update_pos(self.rect, dt.as_secs_f32());
			if matches!(enemy.state, EnemyState::OffScreen) {
				to_remove.push(i);
			}
			// Shooting
			if enemy.proj_cd.is_over() && self.rect.contains(enemy.pos) {
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
				self.projectiles.push(proj);
				enemy.proj_cd.last_emit = Some(Instant::now());
			}
		}
		for i in to_remove.into_iter().rev() {
			self.enemies.remove(i);
		}
	}

	pub fn update_projectiles(&mut self, dt: Duration) {
		let player = &mut self.player;

		let mut to_remove: Vec<usize> = vec![];
		for (i, proj) in self.projectiles.iter_mut().enumerate() {
			proj.pos += proj.vel * dt.as_secs_f32() / DT_60;
			if !self.rect.contains(proj.pos) {
				to_remove.push(i);
				continue;
			}
			for (j, enemy) in self.enemies.iter_mut().enumerate() {
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
						self.enemies.remove(j);
						self.score += 100;
						break;
					}
				}
			}
			if player.immunity.is_over()
				& collide_rectangle(
					player.pos,
					proj.pos,
					player.hitbox.dims,
					Dimensions { w: 10., h: 10. },
				) & !matches!(proj.variant, ProjType::PlayerShoot)
			{
				if player.hp > 0 {
					player.hp -= 1;
				}
				if player.hp == 0 {
					break;
				}
				to_remove.push(i);

				player.immunity.last_emit = Some(Instant::now());
			}
		}
		for i in to_remove.into_iter().rev() {
			self.projectiles.remove(i);
		}
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

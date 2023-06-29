use cgmath::{Point2, Vector2};
use error_iter::ErrorIter as _;
use image;
use log::error;
use pixels::{Error, Pixels, SurfaceTexture};
use std::time::Instant;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Icon, WindowBuilder};
use winit_input_helper::WinitInputHelper;

const W_WIDTH: u32 = 320;
const W_HEIGHT: u32 = 240;

const SPRITESHEET_PATH: &str = "./assets/spritesheet.png";

struct Rect2D {
    width: f32,
    height: f32,
}

//TODO remove
#[allow(dead_code)]
struct Character {
    pos: Point2<f32>,
    size: Rect2D,
    size_hit: Rect2D,
    id: u32,
    hp: u16,
}

//TODO remove
#[allow(dead_code)]
struct Projectile {
    pos: Point2<f32>,
    vel: Vector2<f32>,
    sender: u32,
    variant: u16,
    size: Rect2D,
}
struct World {
    player: Character,
    projectiles: Vec<Projectile>,
}

/// Returns the window icon from the stylesheet
fn load_icon(path: &str) -> Option<Icon> {
    let icon = {
        let icon_size = (16, 16);
        let mut sprite_sheet = image::open(path).expect("Failed to load image");
        let icon = sprite_sheet.crop(0, 0, icon_size.0, icon_size.1);
        let px_array: Vec<u8> = icon.as_bytes().into();
        Icon::from_rgba(px_array, icon_size.0, icon_size.1).unwrap()
    };
    return Some(icon);
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(W_WIDTH as f64, W_HEIGHT as f64);
        let max_size = LogicalSize::new((2 * W_WIDTH) as f64, (2 * W_HEIGHT) as f64);
        let window_icon = load_icon(SPRITESHEET_PATH);
        WindowBuilder::new()
            .with_title("Holy Bullet Hell")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_max_inner_size(max_size)
            .with_window_icon(window_icon)
            .with_position(LogicalPosition::new(0, 0))
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(W_WIDTH, W_HEIGHT, surface_texture)?
    };
    let mut world = World::new();

    let mut t = Instant::now();
    event_loop.run(move |event, _, control_flow| {
        // Draw the current frame
        if let Event::RedrawRequested(_) = event {
            world.draw(pixels.frame_mut());
            if let Err(err) = pixels.render() {
                log_error("pixels.render", err);
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        // Handle input events
        if input.update(&event) {
            let dt = t.elapsed().as_secs_f64();
            t = Instant::now();
            println!("FPS: {}", 1. / dt);

            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.close_requested() {
                *control_flow = ControlFlow::Exit;
                return;
            }
            struct Keybind {
                up: VirtualKeyCode,
                left: VirtualKeyCode,
                down: VirtualKeyCode,
                right: VirtualKeyCode,
                shoot: VirtualKeyCode,
            }
            let keybind = if cfg!(feature = "kb_azerty") {
                Keybind {
                    up: VirtualKeyCode::Z,
                    left: VirtualKeyCode::Q,
                    down: VirtualKeyCode::S,
                    right: VirtualKeyCode::D,
                    shoot: VirtualKeyCode::X,
                }
            } else {
                Keybind {
                    up: VirtualKeyCode::W,
                    left: VirtualKeyCode::A,
                    down: VirtualKeyCode::S,
                    right: VirtualKeyCode::D,
                    shoot: VirtualKeyCode::X,
                }
            };
            let mut player_move: Vector2<f32> = (0., 0.).into();
            if input.key_held(keybind.up) || input.key_held(VirtualKeyCode::Up) {
                player_move += (0., -1.).into();
            }
            if input.key_held(keybind.left) || input.key_held(VirtualKeyCode::Left) {
                player_move += (-1., 0.).into();
            }
            if input.key_held(keybind.down) || input.key_held(VirtualKeyCode::Down) {
                player_move += (0., 1.).into();
            }
            if input.key_held(keybind.right) || input.key_held(VirtualKeyCode::Right) {
                player_move += (1., 0.).into();
            }

            if input.key_pressed(keybind.shoot) || input.key_held(keybind.shoot) {
                let proj_pos = world.player.pos + Vector2::new(0., -26.);
                let proj_vel: Vector2<f32> = (0., -50.).into();
                let p = Projectile::new(proj_pos, proj_vel, world.player.id);
                world.projectiles.push(p);
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                if let Err(err) = pixels.resize_surface(size.width, size.height) {
                    log_error("pixels.resize_surface", err);
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }

            // Update internal state and request a redraw

            world.update(Some(player_move), &dt);
            window.request_redraw();
        }
    });
}

fn log_error<E: std::error::Error + 'static>(method_name: &str, err: E) {
    error!("{method_name}() failed: {err}");
    for source in err.sources().skip(1) {
        error!("  Caused by: {source}");
    }
}

impl World {
    /// Create a new `World` instance that can draw a moving box.
    fn new() -> Self {
        Self {
            player: Character::new(0),
            projectiles: Vec::new(),
        }
    }

    /// Update the `World` internal state; bounce the box around the screen.
    fn update(&mut self, player_move: Option<Vector2<f32>>, dt: &f64) {
        let mut to_remove: Vec<usize> = vec![];
        for (i, p) in self.projectiles.iter_mut().enumerate() {
            if p.update(*dt) {
                to_remove.push(i);
            };
        }
        to_remove.reverse();
        for i in to_remove {
            self.projectiles.remove(i);
        }
        if let Some(player_move) = player_move {
            self.player.update_pos(player_move, *dt);
        }
    }

    /// Draw the `World` state to the frame buffer.
    ///
    /// Assumes the default texture format: `wgpu::TextureFormat::Rgba8UnormSrgb`
    fn draw(&self, frame: &mut [u8]) {
        let mut projectiles_pixels: Vec<usize> = vec![];

        for p in self.projectiles.iter() {
            let x = (p.pos.x - p.size.width / 2.).floor() as u32;
            let y = (p.pos.y - p.size.height / 2.).floor() as u32;
            let w = p.size.width as u32;
            let h = p.size.height as u32;
            for i in x..(x + w) {
                for j in y..(y + h) {
                    projectiles_pixels.push((j * W_WIDTH + i) as usize);
                }
            }
        }
        projectiles_pixels.sort();
        for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
            let x = (i % W_WIDTH as usize) as f32;
            let y = (i / W_WIDTH as usize) as f32;

            let (inside_hit, inside) = {
                let px = self.player.pos.x;
                let py = self.player.pos.y;
                let hw = self.player.size_hit.width;
                let hh = self.player.size_hit.height;
                let pw = self.player.size.width;
                let ph = self.player.size.height;
                let in_hit = px - hw / 2. <= x
                    && x <= px + hw / 2.
                    && py - hh / 2. <= y
                    && y <= py + hh / 2.;
                let in_play = px - pw / 2. <= x
                    && x <= px + pw / 2.
                    && py - ph / 2. <= y
                    && y <= py + ph / 2.;
                (in_hit, in_play)
            };
            let is_proj = projectiles_pixels.binary_search(&i).is_ok();
            // let is_proj = false;
            let rgba = if is_proj {
                [0x00, 0xff, 0x00, 0xff]
            } else if inside_hit {
                [0xff, 0x00, 0x00, 0xff]
            } else if inside {
                [0x00, 0x00, 0xff, 0xff]
            } else {
                [0x5b, 0xce, 0xfa, 0xff]
            };

            pixel.copy_from_slice(&rgba);
        }
    }
}

impl Character {
    fn new(id_: u32) -> Self {
        Self {
            pos: (25., 25.).into(),
            size: (50., 50.).into(),
            size_hit: (10., 10.).into(),
            id: id_,
            hp: 2,
        }
    }

    fn update_pos(&mut self, delta: Vector2<f32>, dt: f64) {
        let new_pos = self.pos + delta * 144. * (dt as f32);
        let hit_w = self.size_hit.width;
        let hit_h = self.size_hit.height;
        if hit_w / 2. <= new_pos.x && (new_pos.x + hit_w / 2.) <= (W_WIDTH as f32) {
            self.pos.x = new_pos.x;
        }
        if hit_h / 2. <= new_pos.y && (new_pos.y + hit_h / 2.) <= (W_HEIGHT as f32) {
            self.pos.y = new_pos.y;
        }
    }
}

impl Projectile {
    fn new(pos_: Point2<f32>, vel_: Vector2<f32>, origin: u32) -> Self {
        Self {
            pos: pos_,
            vel: vel_,
            sender: origin,
            variant: 0,
            size: (7., 7.).into(),
        }
    }
    fn _but_variant(mut self, var: u16) -> Self {
        self.variant = var;
        self
    }

    fn update(&mut self, dt: f64) -> bool {
        self.pos += self.vel * dt as f32;
        let w = self.size.width;
        let h = self.size.height;
        // Out of bounds
        let destroy = self.pos.x + w / 2. <= 0.
            || self.pos.x - w / 2. >= (W_WIDTH as f32)
            || self.pos.y + h / 2. <= 0.
            || self.pos.y - h / 2. >= (W_HEIGHT as f32);
        return destroy;
    }
}

impl From<(f32, f32)> for Rect2D {
    fn from(value: (f32, f32)) -> Self {
        Self {
            width: value.0,
            height: value.1,
        }
    }
}

use std::{collections::HashMap, path::Path};

use kira::{
	manager::{AudioManager, AudioManagerSettings},
	sound::static_sound::StaticSoundData,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundBase {
	PlayerShoot,
}

pub struct Audio {
	manager: AudioManager,
	data: HashMap<SoundBase, StaticSoundData>,
}

impl Audio {
	pub fn new() -> Audio {
		let mut audio = Audio {
			manager: AudioManager::new(AudioManagerSettings::default()).unwrap(),
			data: HashMap::new(),
		};
		audio.load_sounds();
		audio
	}

	fn load_sounds(&mut self) {
		let level_dir: &Path = Path::new("./assets/audio");
		if !level_dir.exists() {
			panic!("Audio directory doesn't exist");
		}
		// TODO: Better management for multiple files?
		self.data.insert(
			SoundBase::PlayerShoot,
			StaticSoundData::from_file("./assets/audio/player_shoot.wav", Default::default()).unwrap(),
		);
	}

	pub fn play_sound(&mut self, id: SoundBase) {
		self.manager.play(self.data[&id].clone()).unwrap();
	}
}

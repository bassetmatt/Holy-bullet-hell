use std::{fs, path::Path};

use kira::{
	manager::{AudioManager, AudioManagerSettings},
	sound::static_sound::StaticSoundData,
};

pub struct Audio {
	manager: AudioManager,
	data: Vec<StaticSoundData>,
}

impl Audio {
	pub fn new() -> Audio {
		let mut audio = Audio {
			manager: AudioManager::new(AudioManagerSettings::default()).unwrap(),
			data: vec![],
		};
		audio.load_base();
		audio
	}

	fn load_base(&mut self) {
		let level_dir: &Path = Path::new("./assets/audio");
		if !level_dir.exists() {
			panic!("Audio directory doesn't exist");
		}
		for file in fs::read_dir(level_dir).unwrap() {
			let path = file.unwrap().path();
			if path.is_file() {
				self.load_sound(path.to_str().unwrap());
			}
		}
	}

	fn load_sound(&mut self, path: &str) {
		let sound_data = StaticSoundData::from_file(path, Default::default()).unwrap();
		self.data.push(sound_data);
	}

	pub fn play_sound(&mut self, id: usize) {
		self.manager.play(self.data[id].clone()).unwrap();
	}
}

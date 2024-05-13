use std::{collections::HashMap, path::Path, time::Duration};

use kira::{
	manager::{AudioManager, AudioManagerSettings},
	sound::{
		static_sound::{StaticSoundData, StaticSoundHandle},
		PlaybackState,
	},
	tween::Tween,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundBase {
	PlayerShoot,
	_MainMenu,
	_MenuSelect,
	_MenuBack,
	_MenuMove,
	_GameMusic,
}

type PlayEntry = (usize, SoundBase);

pub struct Audio {
	manager: AudioManager,
	data: HashMap<SoundBase, StaticSoundData>,
	id_counter: usize,
	playing: HashMap<PlayEntry, StaticSoundHandle>,
}

impl Audio {
	pub fn new() -> Audio {
		let mut audio = Audio {
			manager: AudioManager::new(AudioManagerSettings::default()).unwrap(),
			data: HashMap::new(),
			id_counter: 0,
			playing: HashMap::new(),
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

	pub fn play_sound(&mut self, sound_type: SoundBase) -> usize {
		let handle = self.manager.play(self.data[&sound_type].clone()).unwrap();
		// Gets the sound handle and inserts it into the playing hashmap
		self.playing.insert((self.id_counter, sound_type), handle);
		self.id_counter += 1;
		self.id_counter - 1
	}

	pub fn _stop_sound(&mut self, entry: &PlayEntry) {
		if let Some(mut handle) = self.playing.remove(entry) {
			handle
				.stop(Tween { duration: Duration::from_micros(10), ..Default::default() })
				.unwrap();
		}
	}

	pub fn _stop_sound_condition(&mut self, condition: impl Fn(&(usize, SoundBase)) -> bool) {
		self.playing.retain(|key, handle| {
			if !condition(key) {
				return true;
			}
			handle
				.stop(Tween { duration: Duration::from_micros(10), ..Default::default() })
				.unwrap();
			false
		});
	}

	pub fn _stop_sound_by_type(&mut self, sound_type: SoundBase) {
		self._stop_sound_condition(|(_, sound)| sound != &sound_type);
	}

	pub fn delete_ended_sounds(&mut self) {
		self
			.playing
			.retain(|_, handle| handle.state() != PlaybackState::Stopped);
	}
}

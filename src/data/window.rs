use super::defaults::default_five;
use crate::Args;
use serde::Deserialize;
use std::{fs::File, io::BufReader};

#[derive(Deserialize, Clone, Debug)]
pub struct Window {
	pub width: u32,
	pub height: u32,
	pub scale: u32,
	pub frame_rate: usize,

	#[serde(default = "default_five")]
	pub duration_secs: f64,
}

const DEFAULT_169: Window = Window {
	width: 480,
	height: 270,
	scale: 4,
	frame_rate: 60,
	duration_secs: 5.0,
};

const DEFAULT_916: Window = Window {
	width: 216,
	height: 384,
	scale: 5,
	frame_rate: 30,
	duration_secs: 3.0,
};

const DEFAULT_918: Window = Window {
	width: 216,
	height: 432,
	scale: 5,
	frame_rate: 30,
	duration_secs: 3.0,
};

impl Window {
	pub fn load_from_args(cmd: &Args) -> Self {
		if let Some(preset) = &cmd.window_preset {
			match preset.as_ref() {
				"16x9" => return DEFAULT_169,
				"9x16" => return DEFAULT_916,
				"9x18" => return DEFAULT_918,
				_ => panic!("This preset does not exist - use '16x9', '9x16', or '9x18'."),
			}
		}

		Window::load_from_file(&cmd.window)
	}

	pub fn load_from_file(window_path: &str) -> Self {
		let file = File::open(window_path);
		if file.is_err() {
			panic!("Could not open window.json");
		}

		let rdr = BufReader::new(file.unwrap());

		serde_json::from_reader(rdr).unwrap()
	}
}

use super::{
	channel::SongError,
	defaults::{default_output, default_true},
	video::Encoding,
};
use crate::{
	display::{draw, RGB},
	SCREEN_FRAME_RATE, SCREEN_HEIGHT, SCREEN_WIDTH, SCREEN_DURATION_SECS
};
use image::RgbImage;
use midly::{
	num::{u24, u28, u7},
	MetaMessage, MidiMessage, Smf, TrackEventKind,
};
use serde::Deserialize;
use std::{collections::HashMap, fs::File, io::BufReader};

#[derive(Debug, Clone)]
pub struct MidiNote {
	pub tick_on: u32,
	pub tick_off: u32,
	pub note: u8,
}

#[derive(Debug, Clone)]
pub struct MidiChannel {
	pub name: String,
	pub note_min: u8,
	pub note_max: u8,
	last_tick: u32,
	pub colour: RGB,
	pub notes: Vec<MidiNote>,
}

impl MidiChannel {
	pub fn new(key: u7, delta: u32) -> Self {
		Self {
			name: String::new(),
			note_min: key.as_int(),
			note_max: key.as_int(),
			last_tick: delta,
			notes: vec![],
			colour: [24, 24, 24],
		}
	}

	pub fn new_with_name(name: String) -> Self {
		Self {
			name,
			note_min: 255,
			note_max: 0,
			last_tick: 0,
			notes: vec![],
			colour: [24, 24, 24],
		}
	}
}

fn lerp_range_u8(value: u8, v_min: u8, v_max: u8, m_min: f64, m_max: f64) -> f64 {
	(((value - v_min) as f64 / (v_max - v_min) as f64) * (m_max - m_min)) + m_min
}

fn lerp_range_i32(value: i32, v_min: i32, v_max: i32, m_min: f64, m_max: f64) -> f64 {
	(((value - v_min) as f64 / (v_max - v_min) as f64) * (m_max - m_min)) + m_min
}

#[derive(Debug, Deserialize, Default)]
pub struct MidiChannelConfig {
	#[serde(default)]
	pub order: usize,

	#[serde(default)]
	pub colour: RGB,

	#[serde(default = "default_true")]
	pub visible: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct MidiSongConfig {
	pub midi_file: String,

	pub channels: HashMap<String, MidiChannelConfig>,

	#[serde(default = "default_output")]
	pub video_file_out: String,

	#[serde(default = "default_true")]
	pub use_gradients: bool,
}

#[derive(Debug)]
pub struct MidiSong {
	pub ppq: u16,
	pub tempo: u32,
	pub us_per_tick: f64,
	pub duration_ticks: u32,
	pub playhead_secs: f64,
	pub seconds_per_frame: f64,
	pub config: MidiSongConfig,
	pub channels: HashMap<usize, MidiChannel>,
	pub channels_vec: Vec<MidiChannel>,
}

impl MidiSong {
	pub fn new(smf: &Smf, config: MidiSongConfig) -> Self {
		Self {
			us_per_tick: 0.0,
			ppq: MidiSong::get_ppq(smf),
			tempo: 0,
			duration_ticks: 0,
			playhead_secs: -(*SCREEN_DURATION_SECS / 2.0),
			seconds_per_frame: *SCREEN_DURATION_SECS,
			channels: HashMap::new(),
			channels_vec: Vec::with_capacity(16),
			config,
		}
	}

	pub fn load_from_file(json_path: &str) -> Self {
		let file = File::open(json_path);
		if file.is_err() {
			panic!("Could not open song.json");
		}
		let file = file.unwrap();

		let rdr = BufReader::new(file);
		let config: MidiSongConfig = serde_json::from_reader(rdr).unwrap();

		MidiSong::generate_song_from_midi(config)
	}

	fn update_name(&mut self, channel: usize, name: &[u8]) {
		let name_str = String::from_utf8(name.to_vec()).unwrap();

		if let Some(chan) = self.channels.get_mut(&channel) {
			if chan.name.is_empty() {
				chan.name = name_str;
			}
		} else {
			let chan = MidiChannel::new_with_name(name_str);
			self.channels.insert(channel, chan);
		}
	}

	fn add_note(&mut self, channel: usize, key: u7, delta: u28) {
		if !self.channels.contains_key(&channel) {
			self.channels.insert(channel, MidiChannel::new(key, 0));
		}

		let chan = self.channels.get_mut(&channel).unwrap();
		let note = key.as_int();

		// Update MIN MAX
		if note > chan.note_max {
			chan.note_max = note;
		}
		if note < chan.note_min {
			chan.note_min = note;
		}

		// Insert note
		chan.last_tick += delta.as_int();
		chan.notes.push(MidiNote {
			tick_on: chan.last_tick,
			tick_off: 0,
			note,
		});
	}

	fn end_note(&mut self, channel: usize, key: u7, delta: u28) {
		let chan = self.channels.get_mut(&channel).unwrap();

		chan.last_tick += delta.as_int();
		if let Some(hanging_note) = chan
			.notes
			.iter_mut()
			.find(|note| note.note == key.as_int() && note.tick_off == 0)
		{
			hanging_note.tick_off = chan.last_tick;
		}

		if chan.last_tick > self.duration_ticks {
			self.duration_ticks = chan.last_tick;
		}
	}

	fn get_ppq(smf: &Smf) -> u16 {
		match smf.header.timing {
			midly::Timing::Metrical(a) => a.as_int(),
			midly::Timing::Timecode(_, _) => unimplemented!("Logic to figure out here..."),
		}
	}

	fn get_tempo(&mut self, tempo: u24) {
		self.tempo = tempo.as_int();
		self.us_per_tick = self.tempo as f64 / self.ppq as f64;
	}

	pub fn get_song_duration(&self) -> f64 {
		self.duration_ticks as f64 * (self.us_per_tick / 1_000_000.0)
	}

	fn get_ticks_in_time_frame(&self) -> (u32, u32, i32, i32) {
		let tick_start = ((self.playhead_secs * 1_000_000.0) / self.us_per_tick)
			.clamp(0.0, self.duration_ticks as f64) as u32;

		let tick_end = (((self.playhead_secs + self.seconds_per_frame) * 1_000_000.0)
			/ self.us_per_tick)
			.clamp(0.0, self.duration_ticks as f64) as u32;

		let tick_start_unclamped = ((self.playhead_secs * 1_000_000.0) / self.us_per_tick) as i32;

		let tick_end_unclamped = (((self.playhead_secs + self.seconds_per_frame) * 1_000_000.0)
			/ self.us_per_tick) as i32;

		(
			tick_start,
			tick_end,
			tick_start_unclamped,
			tick_end_unclamped,
		)
	}

	fn get_notes_in_time_frame(
		channel: &MidiChannel,
		tick_start: u32,
		tick_end: u32,
	) -> Vec<MidiNote> {
		channel
			.notes
			.iter()
			.filter(|note| note.tick_off >= tick_start && note.tick_on <= tick_end)
			.cloned()
			.collect()
	}

	pub fn generate_song_from_midi(config: MidiSongConfig) -> MidiSong {
		let data = std::fs::read(&config.midi_file).unwrap();
		let smf = Smf::parse(&data).unwrap();
		let mut song = MidiSong::new(&smf, config);

		let mut channel_index = 0;
		smf.tracks.iter().for_each(|track| {
			track.iter().for_each(|event| match event.kind {
				TrackEventKind::Meta(message) => match message {
					MetaMessage::Tempo(tempo) => {
						song.get_tempo(tempo);
					}
					MetaMessage::TrackName(name) => song.update_name(channel_index, name),
					_ => {}
				},
				TrackEventKind::Midi {
					channel: _,
					message,
				} => match message {
					MidiMessage::NoteOn { key, vel: _ } => {
						song.add_note(channel_index, key, event.delta);
					}
					MidiMessage::NoteOff { key, vel: _ } => {
						song.end_note(channel_index, key, event.delta);
					}
					_ => {}
				},
				_ => {}
			});
			channel_index += 1;
		});

		song.channels_vec =
			song.channels
				.iter()
				.fold(Vec::with_capacity(16), |mut channels, (_, channel)| {
					if channel.notes.len() == 0 {
						return channels;
					}
					let mut new_channel = channel.clone();
					if let Some(config) = song.config.channels.get(&channel.name) {
						if !config.visible {
							return channels;
						}
						new_channel.colour = config.colour;
					}
					channels.push(new_channel);
					channels
				});

		// Compare based on JSON config, fallback to channel name sorting.
		song.channels_vec.sort_by(|a, b| {
			if let Some(a_cfg) = song.config.channels.get(&a.name) {
				if let Some(b_cfg) = song.config.channels.get(&b.name) {
					return a_cfg.order.cmp(&b_cfg.order);
				}
			}

			a.name.cmp(&b.name)
		});

		song.channels.clear();

		println!(
			"Info:\n  Duration: {} s\n  Ticks: {}",
			song.get_song_duration(),
			song.duration_ticks
		);

		song
	}

	pub fn draw(&mut self, frame: &mut RgbImage, encoding: &mut Encoding) -> Result<(), SongError> {
		let channel_height = *SCREEN_HEIGHT / self.channels_vec.len() as u32;
		let channel_width = *SCREEN_WIDTH;

		let x_min = 0;
		let x_min_f = x_min as f64;
		let x_max = x_min + channel_width - 1;
		let x_max_f = x_max as f64;
		let mut row = 0;

		for channel in &self.channels_vec {
			let y_min = channel_height * row as u32;
			let y_min_f = y_min as f64;
			let y_max = y_min + channel_height - 1;
			let y_max_f = y_max as f64;

			if self.config.use_gradients {
				draw::rect_gradient(frame, x_min, y_min, x_max, y_max, channel.colour);
			} else {
				draw::rect(frame, x_min, y_min, x_max, y_max, channel.colour);
			}

			let (tick_start, tick_end, ts_raw, te_raw) = self.get_ticks_in_time_frame();
			let notes = MidiSong::get_notes_in_time_frame(channel, tick_start, tick_end);
			for note in notes {
				let x1 =
					lerp_range_i32(note.tick_on as i32, ts_raw, te_raw, x_min_f, x_max_f).floor();

				let x2 =
					lerp_range_i32(note.tick_off as i32, ts_raw, te_raw, x_min_f, x_max_f).floor();

				let y = lerp_range_u8(
					note.note,
					channel.note_min,
					channel.note_max,
					y_max_f - 4.0,
					y_min_f + 4.0,
				)
				.floor();

				let x_mid = (ts_raw + te_raw) as f64 / 2.0;
				let dist_to_mid = x_mid - note.tick_on as f64;
				let dist_to_mid_clamped = if dist_to_mid < 0.0 { 20.0 } else { dist_to_mid };

				let mut scale = (1.0 - (dist_to_mid_clamped / 20.0)).max(0.0);

				let note_is_playing =
					(note.tick_on as f64) < x_mid && (note.tick_off as f64) > x_mid;

				if note_is_playing {
					scale += 1.0;
				}

				let x1_min = (x1 - scale).clamp(x_min_f, x_max_f - 1.0) as u32;
				let x2_min = (x2 + scale).clamp(x_min_f, x_max_f - 1.0) as u32;
				let y1_min = (y - scale).clamp(y_min_f, y_max_f - 1.0) as u32;
				let y2_min = (y + scale + 1.0).clamp(y_min_f, y_max_f - 1.0) as u32;

				draw::rect(
					frame,
					x1_min + 1,
					y1_min + 1,
					x2_min + 1,
					y2_min + 1,
					[0, 0, 0],
				);
				draw::rect(frame, x1_min, y1_min, x2_min, y2_min, [255, 255, 255]);
			}

			draw::text_colour(frame, x_min + 5, y_min + 5, &channel.name, [0, 0, 0]);
			draw::text(frame, x_min + 4, y_min + 4, &channel.name);

			row += 1;
		}

		// Render frame to video
		encoding.render_frame(frame);

		self.playhead_secs += 1.0 / *SCREEN_FRAME_RATE as f64;

		if self.playhead_secs >= self.get_song_duration() {
			return Err(SongError::End);
		}

		Ok(())
	}
}

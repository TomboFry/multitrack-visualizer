use super::{image::pixels_to_png, track::load_track_into_memory};
use crate::{
	display::{draw, RGB},
	SCREEN_HEIGHT, SCREEN_WIDTH,
};
use rayon::prelude::*;
use serde::Deserialize;
use std::{collections::VecDeque, fs::File, io::BufReader};
use symphonia::core::{
	audio::{AudioBufferRef, Signal},
	codecs::Decoder,
	errors::Error,
	formats::{FormatReader, Track},
};
use winit_input_helper::WinitInputHelper;

#[derive(Deserialize)]
pub struct Channel {
	pub name: String,
	pub file: String,
	pub colour: RGB,

	#[serde(skip)]
	pub track: Option<Track>,

	#[serde(skip)]
	pub format: Option<Box<dyn FormatReader>>,

	#[serde(skip)]
	pub decoder: Option<Box<dyn Decoder>>,

	#[serde(skip)]
	pub buffer: VecDeque<u8>,

	#[serde(skip)]
	pub play_time_samples: u64,
}

impl Channel {
	pub fn get_frame_samples(&mut self) -> Vec<u8> {
		let format = self.format.as_mut().unwrap();
		let track = self.track.as_mut().unwrap();
		let decoder = self.decoder.as_mut().unwrap();
		let min_samples_required = track.codec_params.sample_rate.unwrap() as usize / 60;
		let mut retries = 100;

		if self.buffer.capacity() < min_samples_required {
			self.buffer
				.reserve(min_samples_required - self.buffer.capacity())
		}

		while self.buffer.len() < min_samples_required || retries > 0 {
			// loop of death prevention measure
			retries -= 1;

			// Get the next packet from the media format.
			let packet = match format.next_packet() {
				Ok(packet) => packet,
				Err(Error::IoError(_err)) => {
					if self.play_time_samples
						>= track.codec_params.n_frames.unwrap() - min_samples_required as u64
					{
						panic!("End of song")
					}
					continue;
				}
				Err(err) => {
					// A unrecoverable error occured, halt decoding.
					panic!("{}", err);
				}
			};

			// Consume any new metadata that has been read since the last packet.
			while !format.metadata().is_latest() {
				// Pop the old head of the metadata queue.
				format.metadata().pop();

				// Consume the new metadata at the head of the metadata queue.
			}

			if packet.track_id() != track.id {
				println!("Track doesn't match, skipping...");
				continue;
			}

			// Decode the packet into audio samples.
			match decoder.decode(&packet) {
				Ok(decoded) => match decoded {
					AudioBufferRef::F32(buf) => {
						let mut samples = buf
							.chan(0)
							.par_iter()
							.map(|sample| ((sample / 65536.0) + 128.0) as u8)
							.collect::<VecDeque<u8>>();
						self.buffer.append(&mut samples);
					}
					AudioBufferRef::S24(buf) => {
						let mut samples = buf
							.chan(0)
							.par_iter()
							.map(|sample| (((sample.0 as f32) / 65536.0) + 128.0) as u8)
							.collect::<VecDeque<u8>>();
						self.buffer.append(&mut samples);
					}
					_ => {
						// Repeat for the different sample formats.
						unimplemented!()
					}
				},
				Err(Error::IoError(_)) => {
					// The packet failed to decode due to an IO error, skip the packet.
					continue;
				}
				Err(Error::DecodeError(_)) => {
					// The packet failed to decode due to invalid data, skip the packet.
					continue;
				}
				Err(err) => {
					// An unrecoverable error occured, halt decoding.
					panic!("{}", err);
				}
			}
		}

		self.play_time_samples += min_samples_required as u64;

		self.buffer.drain(0..min_samples_required).collect()
	}
}

#[derive(Deserialize, Clone, Debug)]
pub struct Window {
	pub width: u32,
	pub height: u32,
	pub scale: u32,
}

impl Window {
	pub fn load_from_file() -> Self {
		let file = File::open("./song/window.json");
		if file.is_err() {
			panic!("Could not open window.json");
		}
		let file = file.unwrap();

		let rdr = BufReader::new(file);
		let song: Window = serde_json::from_reader(rdr).unwrap();

		song
	}
}

#[derive(Deserialize)]
pub struct Song {
	pub channels: Vec<Channel>,
	pub render_images: bool,

	#[serde(skip)]
	pub frame: usize,
}

impl Song {
	pub fn load_from_file() -> Self {
		let file = File::open("./song/song.json");
		if file.is_err() {
			panic!("Could not open song.json");
		}
		let file = file.unwrap();

		let rdr = BufReader::new(file);
		let song: Song = serde_json::from_reader(rdr).unwrap();

		println!("Loaded song with {} channels", song.channels.len());
		for channel in &song.channels {
			println!("- {} ({})", channel.name, channel.file);
		}

		song
	}

	pub fn load_tracks_into_memory(&mut self) {
		for channel in &mut self.channels {
			let (format, track, decoder) = load_track_into_memory(&channel.file);
			channel.format = Some(format);
			channel.track = Some(track);
			channel.decoder = Some(decoder);
		}
	}

	pub fn update(&mut self, _input: &WinitInputHelper) {}

	pub fn draw(&mut self, frame: &mut [u8]) {
		let cols = 2.min(self.channels.len());
		let rows = self.channels.chunks_mut(cols);

		let elm_height = *SCREEN_HEIGHT as usize / rows.len();
		let elm_width = *SCREEN_WIDTH as usize / cols;

		for (row, chunks) in rows.enumerate() {
			let y_off = elm_height * row;

			for (col, channel) in chunks.iter_mut().enumerate() {
				let x_off = elm_width * col;

				// Background Colour
				draw::rect(
					frame,
					x_off,
					y_off,
					x_off + elm_width - 1,
					y_off + elm_height - 1,
					channel.colour,
				);

				// Channel Name
				draw::text(frame, x_off + 4, y_off + 4, &channel.name);

				// Draw samples

				let raw_samples = channel.get_frame_samples();

				// Resample raw vector by lerping between adjacent samples
				let samples: Vec<u8> = (0..elm_width)
					.into_par_iter()
					.map(|index| {
						let pc = (index as f32 / elm_width as f32) * raw_samples.len() as f32;
						let remainder = pc % 1.0;
						let i_low = pc.floor() as usize;
						let i_high = pc.ceil() as usize;

						// (1 - t) * v0 + t * v1;
						let value = (1.0 - remainder) * raw_samples[i_low] as f32
							+ remainder * raw_samples[i_high] as f32;

						value as u8
					})
					.collect();

				for (x, sample) in samples.iter().enumerate() {
					if x == 0 {
						continue;
					}

					let mut prev_y = (samples[x - 1] as usize * elm_height) / 256;
					let mut sample_y = (*sample as usize * elm_height) / 256;

					// Swap samples so it's always drawing downwards
					if prev_y > sample_y {
						(sample_y, prev_y) = (prev_y, sample_y);
					}

					// Connect a line to the previous sample
					draw::rect(
						frame,
						x_off + x - 1,
						y_off + prev_y,
						x_off + x + 1,
						y_off + sample_y,
						[255, 255, 255],
					);

					// Draw the current sample
					draw::pixel(frame, x_off + x - 1, y_off + sample_y, [255, 255, 255]);
				}
			}
		}

		if self.render_images {
			pixels_to_png(frame, &format!("./output/{}.png", self.frame));
			self.frame += 1;
		}
	}
}

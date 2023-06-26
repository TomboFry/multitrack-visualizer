use super::{image::pixels_to_png, track::load_track_into_memory};
use crate::{
	display::{draw, RGB},
	SCREEN_FRAME_RATE, SCREEN_HEIGHT, SCREEN_WIDTH,
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
		let min_samples_required =
			(track.codec_params.sample_rate.unwrap() / *SCREEN_FRAME_RATE) as usize;

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
	pub frame_rate: u32,
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

	pub fn draw(&mut self, frame: &mut [u8]) {
		let cols = if *SCREEN_WIDTH >= *SCREEN_HEIGHT {
			2.min(self.channels.len())
		} else {
			1
		};

		let rows = self.channels.chunks_mut(cols);

		let channel_height = *SCREEN_HEIGHT as usize / rows.len();
		let channel_width = *SCREEN_WIDTH as usize / cols;

		for (row, chunks) in rows.enumerate() {
			let y_offset = channel_height * row;

			for (col, channel) in chunks.iter_mut().enumerate() {
				let x_offset = channel_width * col;

				// Background Colour
				draw::rect(
					frame,
					x_offset,
					y_offset,
					x_offset + channel_width - 1,
					y_offset + channel_height - 1,
					channel.colour,
				);

				// Channel Name
				draw::text(frame, x_offset + 4, y_offset + 4, &channel.name);

				// Draw samples
				let raw_samples = channel.get_frame_samples();

				// Determine a good start sample
				// Loop through the first 10% of samples and find a significant jump in the signal
				let mut start_sample = 0;

				for x in 0..raw_samples.len() / 10 {
					let y_previous = raw_samples[x] as i16;
					let y_current = raw_samples[x + 1] as i16;
					let diff = y_previous - y_current;

					if diff >= 10 {
						start_sample = x;
						break;
					}
				}

				// Resample raw vector by lerping between adjacent samples
				let samples: Vec<u8> = (0..channel_width)
					.into_par_iter()
					.map(|index| {
						let percent = (index as f32 / channel_width as f32)
							* (raw_samples.len() - start_sample) as f32;
						let remainder = percent % 1.0;
						let i_low = percent.floor() as usize;
						let i_high = percent.ceil() as usize;

						// Lerp equation: (1 - t) * v0 + t * v1;
						let value = (1.0 - remainder) * raw_samples[start_sample + i_low] as f32
							+ remainder * raw_samples[start_sample + i_high] as f32;

						value as u8
					})
					.collect();

				for (x, sample) in samples.iter().enumerate() {
					if x == 0 {
						continue;
					}

					let mut y_previous = (samples[x - 1] as usize * channel_height) / 256;
					let mut y_current = (*sample as usize * channel_height) / 256;

					// Swap samples so it's always drawing downwards
					if y_previous > y_current {
						(y_current, y_previous) = (y_previous, y_current);
					}

					// Connect a line to the previous sample
					draw::rect(
						frame,
						x_offset + x - 1,
						y_offset + y_previous,
						x_offset + x,
						y_offset + y_current,
						[255, 255, 255],
					);

					// Draw the current sample
					draw::pixel(
						frame,
						x_offset + x - 1,
						y_offset + y_current,
						[255, 255, 255],
					);
				}
			}
		}

		if self.render_images {
			pixels_to_png(frame, &format!("./output/{}.png", self.frame));
			self.frame += 1;
		}
	}
}

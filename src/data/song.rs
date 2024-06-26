use super::{
	channel::{Channel, SongError},
	defaults::{default_output, default_true},
	video::Encoding,
};
use crate::{display::draw, SCREEN_HEIGHT, SCREEN_WIDTH};
use image::RgbImage;
use rayon::prelude::*;
use serde::Deserialize;
use std::{fs::File, io::BufReader};

#[derive(Deserialize)]
pub struct Song {
	pub channels: Vec<Channel>,

	#[serde(default = "default_output")]
	pub video_file_out: String,

	#[serde(default = "default_true")]
	pub use_gradients: bool,
}

impl Song {
	pub fn load_from_file(song_path: &str) -> Self {
		let file = File::open(song_path);
		if file.is_err() {
			panic!("Could not open song.json");
		}
		let file = file.unwrap();

		let rdr = BufReader::new(file);
		let mut song: Song = serde_json::from_reader(rdr).unwrap();

		assert!(
			song.channels.len() > 0,
			"Please provide at least one channel"
		);

		println!("Loaded song with {} channels", song.channels.len());

		println!("\n{:<16} {}", "Channel Name", "Filename");
		for channel in &song.channels {
			let display_name = if channel.name.len() > 16 {
				format!("{}...", channel.name.split_at(13).0)
			} else {
				channel.name.clone()
			};

			println!("{:<16} {}", display_name, channel.file);
		}

		song.load_tracks_into_memory();

		song
	}

	pub fn load_tracks_into_memory(&mut self) {
		for channel in &mut self.channels {
			channel.load_track_into_memory();
		}
	}

	pub fn draw(&mut self, frame: &mut RgbImage, encoding: &mut Encoding) -> Result<(), SongError> {
		let cols = if *SCREEN_WIDTH >= *SCREEN_HEIGHT {
			2.min(self.channels.len())
		} else {
			1
		};

		let rows = self.channels.chunks_mut(cols);

		let channel_height = *SCREEN_HEIGHT / rows.len() as u32;
		let channel_width = *SCREEN_WIDTH / cols as u32;

		for (row, chunks) in rows.enumerate() {
			let y_offset = channel_height * row as u32;

			for (col, channel) in chunks.iter_mut().enumerate() {
				let x_offset = channel_width * col as u32;

				// Background Colour
				draw::rect(
					frame,
					x_offset,
					y_offset + channel_height - 1,
					x_offset + channel_width - 1,
					y_offset + channel_height,
					[0, 0, 0],
				);

				if self.use_gradients {
					draw::rect_gradient(
						frame,
						x_offset,
						y_offset,
						x_offset + channel_width - 1,
						y_offset + channel_height - 1,
						channel.colour,
					);
				} else {
					draw::rect(
						frame,
						x_offset,
						y_offset,
						x_offset + channel_width - 1,
						y_offset + channel_height - 1,
						channel.colour,
					);
				}

				// Channel Name
				draw::text(frame, x_offset + 4, y_offset + 4, &channel.name);

				// Draw samples
				let raw_samples = channel.get_frame_samples();

				if let Err(err) = raw_samples {
					return Err(err);
				}

				let raw_samples = raw_samples.unwrap();

				// Determine a good start sample
				// Loop through the first ~6% of samples and find a significant jump in the signal
				let search_sample_max = raw_samples.len() / 15;
				let mut start_sample = 0;

				if channel.use_alignment {
					for x in 0..search_sample_max {
						let y_previous = raw_samples[x] as i16;
						let y_current = raw_samples[x + 1] as i16;
						let diff = y_previous - y_current;

						if diff >= 8 {
							start_sample = x;
							break;
						}
					}
				}

				// Resample raw vector by lerping between adjacent samples
				let samples: Vec<u8> = (0..channel_width)
					.into_par_iter()
					.map(|index| {
						let percent = (index as f32 / channel_width as f32)
							* (raw_samples.len() - search_sample_max) as f32;
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

					let x_position = x_offset + x as u32;

					let mut y_previous = (samples[x - 1] as u32 * channel_height) / 256;
					let mut y_current = (*sample as u32 * channel_height) / 256;

					// Swap samples so it's always drawing downwards
					if y_previous > y_current {
						(y_current, y_previous) = (y_previous, y_current);
					}

					// Connect a line to the previous sample
					draw::rect(
						frame,
						x_position - 1,
						y_offset + y_previous,
						x_position,
						y_offset + y_current,
						[255, 255, 255],
					);

					// Draw the current sample
					draw::pixel(frame, x_position - 1, y_offset + y_current, [255, 255, 255]);
				}
			}
		}

		// Render frame to video
		encoding.render_frame(frame);

		Ok(())
	}
}

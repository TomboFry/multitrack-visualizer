use super::defaults::default_true;
use crate::{display::RGB, SCREEN_FRAME_RATE};
use rayon::prelude::*;
use serde::Deserialize;
use std::collections::VecDeque;
use symphonia::core::{
	audio::{AudioBufferRef, Signal},
	codecs::{Decoder, CODEC_TYPE_NULL},
	errors::Error,
	formats::{FormatOptions, FormatReader, Track},
	io::MediaSourceStream,
	meta::MetadataOptions,
	probe::Hint,
};

#[derive(Deserialize)]
pub struct Channel {
	pub name: String,
	pub file: String,

	/// Colour is optional, and will default to black, ie. [0,0,0]
	#[serde(default)]
	pub colour: RGB,

	#[serde(default = "default_true")]
	pub use_alignment: bool,

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

	#[serde(skip)]
	pub play_time_samples_total: u64,
}

#[derive(Debug)]
pub enum SongError {
	Error(symphonia::core::errors::Error),
	End,
}

impl Channel {
	pub fn load_track_into_memory(&mut self) {
		// Open the media source.
		let src = std::fs::File::open(&self.file);

		let print_error =
			|error: &str| format!("Could not load track \"{}\" - Error: {}", &self.file, error);

		if let Err(err) = src {
			println!("\n{}\n", print_error(&err.to_string()));
			std::process::exit(1);
		}

		let src = src.unwrap();

		// Create the media source stream.
		let mss = MediaSourceStream::new(Box::new(src), Default::default());

		// Create a probe hint using the file's extension. [Optional]
		let ext = self.file.split(".").collect::<Vec<&str>>();
		let mut hint = Hint::new();
		hint.with_extension(ext[ext.len() - 1]);

		// Use the default options for metadata and format readers.
		let meta_opts: MetadataOptions = Default::default();
		let fmt_opts: FormatOptions = Default::default();

		// Probe the media source.
		let probed = symphonia::default::get_probe()
			.format(&hint, mss, &fmt_opts, &meta_opts)
			.expect(&print_error("Unsupported format"));

		// Get the instantiated format reader.
		let format = probed.format;

		// Find the first audio track with a known (decodeable) codec.
		let track = format
			.tracks()
			.iter()
			.find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
			.expect(&print_error("No supported audio tracks"))
			.clone();

		let codec = track.codec_params.codec.to_string();

		let decoder = symphonia::default::get_codecs()
			.make(&track.codec_params, &Default::default())
			.expect(&print_error(&format!("{codec} is an unsupported codec")));

		self.play_time_samples_total = track.codec_params.n_frames.unwrap();
		self.format = Some(format);
		self.track = Some(track);
		self.decoder = Some(decoder);
	}

	pub fn get_frame_samples(&mut self) -> Result<Vec<u8>, SongError> {
		let format = self.format.as_mut().unwrap();
		let track = self.track.as_mut().unwrap();
		let decoder = self.decoder.as_mut().unwrap();
		let min_samples_required =
			track.codec_params.sample_rate.unwrap() as usize / *SCREEN_FRAME_RATE;

		let mut retries = 100;

		if self.buffer.capacity() < min_samples_required {
			self.buffer
				.reserve(min_samples_required - self.buffer.capacity())
		}

		let print_error = |err: &str| println!("Error rendering \"{}\": {}", &self.file, err);

		while self.buffer.len() < min_samples_required || retries > 0 {
			// loop of death prevention measure
			retries -= 1;

			// Get the next packet from the media format.
			let packet = match format.next_packet() {
				Ok(packet) => packet,
				Err(Error::IoError(_err)) => {
					if self.play_time_samples
						>= self.play_time_samples_total - min_samples_required as u64
					{
						return Err(SongError::End);
					}
					continue;
				}
				Err(err) => {
					// A unrecoverable error occured, halt decoding.
					return Err(SongError::Error(err));
				}
			};

			// Consume any new metadata that has been read since the last packet.
			while !format.metadata().is_latest() {
				// Pop the old head of the metadata queue.
				format.metadata().pop();

				// Consume the new metadata at the head of the metadata queue.
			}

			if packet.track_id() != track.id {
				print_error("Track doesn't match, skipping...");
				continue;
			}

			// Decode the packet into audio samples.
			match decoder.decode(&packet) {
				Ok(decoded) => match decoded {
					AudioBufferRef::F32(buf) => {
						let mut samples = buf
							.chan(0)
							.par_iter()
							.map(|sample| ((sample * 128.0) + 128.0) as u8)
							.collect::<VecDeque<u8>>();
						self.buffer.append(&mut samples);
					}
					AudioBufferRef::F64(buf) => {
						let mut samples = buf
							.chan(0)
							.par_iter()
							.map(|sample| ((sample * 128.0) + 128.0) as u8)
							.collect::<VecDeque<u8>>();
						self.buffer.append(&mut samples);
					}
					AudioBufferRef::S16(buf) => {
						let mut samples = buf
							.chan(0)
							.par_iter()
							.map(|sample| ((*sample / 2i16.pow(8)) + 128) as u8)
							.collect::<VecDeque<u8>>();
						self.buffer.append(&mut samples);
					}
					AudioBufferRef::S24(buf) => {
						let mut samples = buf
							.chan(0)
							.par_iter()
							.map(|sample| ((sample.0 / 2i32.pow(16)) + 128) as u8)
							.collect::<VecDeque<u8>>();
						self.buffer.append(&mut samples);
					}
					AudioBufferRef::S32(buf) => {
						let mut samples = buf
							.chan(0)
							.par_iter()
							.map(|sample| ((*sample / 2i32.pow(24)) + 128) as u8)
							.collect::<VecDeque<u8>>();
						self.buffer.append(&mut samples);
					}
					AudioBufferRef::U8(buf) => {
						// Format already u8, just copy directly
						self.buffer.extend(buf.chan(0).iter());
					}
					_ => {
						// Repeat for the different sample formats.
						print_error("format not supported");
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
					return Err(SongError::Error(err));
				}
			}
		}

		self.play_time_samples += min_samples_required as u64;

		Ok(self.buffer.drain(0..min_samples_required).collect())
	}
}

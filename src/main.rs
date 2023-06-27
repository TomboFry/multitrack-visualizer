use crate::data::{cli::Args, video::Encoding};
use clap::Parser;
use data::song::{Song, Window};
use image::RgbImage;
use lazy_static::lazy_static;
use std::path::PathBuf;
use video_rs::{Encoder, EncoderSettings, Locator};

mod data;
mod display;

lazy_static! {
	pub static ref WINDOW: Window = Window::load_from_file(&Args::parse().window);
	pub static ref SCREEN_WIDTH: u32 = WINDOW.width;
	pub static ref SCREEN_HEIGHT: u32 = WINDOW.height;
	pub static ref SCREEN_SCALE: u32 = WINDOW.scale;
	pub static ref SCREEN_FRAME_RATE: usize = WINDOW.frame_rate;
}

fn main() {
	video_rs::init().expect("Could not initialise FFMPEG");

	let cmd = Args::parse();

	// Step 1: Set up project and encoder
	let mut song = Song::load_from_file(&cmd.song);
	song.load_tracks_into_memory();

	let width = (*SCREEN_WIDTH * *SCREEN_SCALE) as usize;
	let height = (*SCREEN_HEIGHT * *SCREEN_SCALE) as usize;

	let destination: Locator = PathBuf::from(&song.video_file_out).into();
	let settings = EncoderSettings::for_h264_yuv420p(width, height, false);
	let mut frame = RgbImage::new(*SCREEN_WIDTH, *SCREEN_HEIGHT);
	let mut encoding =
		Encoding::new(Encoder::new(&destination, settings).expect("failed to create encoder"));

	println!(
		"\nGenerated frame of size {}x{}",
		frame.width(),
		frame.height()
	);

	// Step 2: Render waveforms
	loop {
		let result = song.draw(&mut frame, &mut encoding);

		if result.is_err() {
			println!("{:?}", result.err().unwrap());
			break;
		}
	}

	// Step 3: Flush MP4 to file
	encoding.flush();
}

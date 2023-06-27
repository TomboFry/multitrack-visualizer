use crate::data::{cli::Args, video::Encoding};
use clap::Parser;
use data::song::{Song, SongError, Window};
use image::RgbImage;
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;

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
	let mut encoding = Encoding::new(&song);
	let mut frame = RgbImage::new(*SCREEN_WIDTH, *SCREEN_HEIGHT);

	// Step 1.5: Setup audio decoders for each track
	song.load_tracks_into_memory();

	let pb = ProgressBar::new(song.channels[0].play_time_samples_total);
	pb.set_style(
		ProgressStyle::with_template("[{eta_precise}]  {wide_bar:.green/black}  {percent}%  ")
			.unwrap()
			.progress_chars("#>-"),
	);

	// Step 2: Render waveforms
	println!("Starting render");
	loop {
		let result = song.draw(&mut frame, &mut encoding);

		pb.set_position(song.channels[0].play_time_samples);

		// `err` can either be the end of the song, or a genuine fault.
		// Either way, stop execution.
		if result.is_err() {
			// Step 3: Flush MP4 to file
			encoding.flush();
			pb.finish();

			match result.err().unwrap() {
				SongError::End => println!("Finished rendering to {}", &song.video_file_out),
				SongError::Error(err) => println!("{:?}", err),
			}

			break;
		}
	}
}

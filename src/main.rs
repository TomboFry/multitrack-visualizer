use clap::Parser;
use data::{
	channel::SongError, cli::Args, midi::MidiSong, song::Song, video::Encoding, window::Window,
};
use image::RgbImage;
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;

mod data;
mod display;

lazy_static! {
	pub static ref WINDOW: Window = Window::load_from_args(&Args::parse());
	pub static ref SCREEN_WIDTH: u32 = WINDOW.width;
	pub static ref SCREEN_HEIGHT: u32 = WINDOW.height;
	pub static ref SCREEN_SCALE: u32 = WINDOW.scale;
	pub static ref SCREEN_FRAME_RATE: usize = WINDOW.frame_rate;
	pub static ref SCREEN_DURATION_SECS: f64 = WINDOW.duration_secs;
}

fn generate_progressbar(total: u64) -> ProgressBar {
	let pb = ProgressBar::new(total);

	pb.set_style(
		ProgressStyle::with_template("[{eta_precise}]  [{wide_bar:.green/black}]  {percent}%  ")
			.unwrap()
			.progress_chars("#>-"),
	);

	pb
}

fn encode_wavs(cmd: &Args) {
	// Step 1: Set up project and encoder
	let mut song = Song::load_from_file(&cmd.song);
	let mut encoding = Encoding::new(&song.video_file_out);
	let mut frame = RgbImage::new(*SCREEN_WIDTH, *SCREEN_HEIGHT);
	let pb = generate_progressbar(song.channels[0].play_time_samples_total);

	// Step 2: Render waveforms
	println!("\nStarting render");
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

fn encode_midi(cmd: &Args) {
	// Step 1: Set up project and encoder
	let mut midi = MidiSong::load_from_file(&cmd.midi);
	let mut encoding = Encoding::new(&midi.config.video_file_out);
	let mut frame = RgbImage::new(*SCREEN_WIDTH, *SCREEN_HEIGHT);

	let pb = generate_progressbar(
		((midi.get_song_duration() + (*SCREEN_DURATION_SECS / 2.0)) * 1000.0) as u64,
	);

	// Step 2: Render waveforms
	println!("\nStarting render");
	loop {
		let result = midi.draw(&mut frame, &mut encoding);

		pb.set_position(((midi.playhead_secs + (*SCREEN_DURATION_SECS / 2.0)) * 1000.0) as u64);

		// `err` can either be the end of the song, or a genuine fault.
		// Either way, stop execution.
		if result.is_err() {
			// Step 3: Flush MP4 to file
			encoding.flush();
			pb.finish();

			match result.err().unwrap() {
				SongError::End => println!("Finished rendering to {}", &midi.config.video_file_out),
				SongError::Error(err) => println!("{:?}", err),
			}

			break;
		}
	}
}

fn main() {
	video_rs::init().expect("Could not initialise FFMPEG");

	let mut cmd = Args::parse();

	if cmd.midi.is_empty() && cmd.song.is_empty() {
		cmd.song = String::from("./song.json");
		return encode_wavs(&cmd);
	}

	if !cmd.song.is_empty() {
		return encode_wavs(&cmd);
	}

	if !cmd.midi.is_empty() {
		encode_midi(&cmd)
	}
}

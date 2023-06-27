use clap::Parser;

/// Generate a video based on the waveforms of multiple audio tracks
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
	/// JSON config file for all the tracks, colours, and audio files
	#[arg(short, long, default_value_t = String::from("./song.json"))]
	pub song: String,

	/// JSON config file for the size and scaling for the output video file
	#[arg(short, long, default_value_t = String::from("./window.json"))]
	pub window: String,
}

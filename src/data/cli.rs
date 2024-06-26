use clap::Parser;

/// Generate a video based on the waveforms of multiple audio tracks
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
	/// JSON config file for all the tracks, colours, and audio files
	#[arg(short, long, default_value_t = String::from(""))]
	pub song: String,

	/// JSON config file for loading a MIDI file instead of WAVs
	#[arg(short, long, default_value_t = String::from(""))]
	pub midi: String,

	/// JSON config file for the size and scaling for the output video file
	#[arg(short, long, default_value_t = String::from("./window.json"))]
	pub window: String,

	/// Choose from preset window options: "16x9" (16:9 at 1080p), "9x16" (9:16 at 1080p), "9x18" (9:18 at 1080p)
	#[arg(short = 'p', long)]
	pub window_preset: Option<String>,
}

use data::{
	image::clear_output_folder,
	song::{Song, Window},
};
use lazy_static::lazy_static;

mod data;
mod display;

lazy_static! {
	pub static ref WINDOW: Window = Window::load_from_file();
	pub static ref SCREEN_WIDTH: u32 = WINDOW.width;
	pub static ref SCREEN_HEIGHT: u32 = WINDOW.height;
	pub static ref SCREEN_SCALE: u32 = WINDOW.scale;
	pub static ref SCREEN_FRAME_RATE: u32 = WINDOW.frame_rate;
}

fn main() {
	// Step 1: Clear output folder of images
	clear_output_folder().unwrap();

	// Step 2: Render waveforms
	let mut song = Song::load_from_file();
	song.load_tracks_into_memory();

	let size = (*SCREEN_WIDTH * *SCREEN_HEIGHT * 3) as usize;
	let mut frame = (0..size).map(|_| 0).collect::<Vec<u8>>();

	loop {
		let result = song.draw(&mut frame);

		if result.is_err() {
			println!("{:?}", result.err().unwrap());
			break;
		}
	}

	// Step 3: Save PNGs to video?
}

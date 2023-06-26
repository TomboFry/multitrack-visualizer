use crate::{SCREEN_HEIGHT, SCREEN_WIDTH};
use image::{ImageBuffer, RgbImage};

pub fn pixels_to_png(frame: &mut [u8], filename: &str) {
	let mut img: RgbImage = ImageBuffer::new(*SCREEN_WIDTH, *SCREEN_HEIGHT);

	img.copy_from_slice(frame);

	img.save(filename)
		.expect(&format!("Could not save file to {}", filename));
}

pub fn clear_output_folder() -> std::io::Result<()> {
	let path = "./output";
	std::fs::create_dir_all(path)?;
	let paths = std::fs::read_dir(path)?;

	for path in paths {
		let p = &path?;
		if let Some(ext) = p.path().extension() {
			if ext == "png" {
				std::fs::remove_file(p.path())?;
			}
		}
	}

	Ok(())
}

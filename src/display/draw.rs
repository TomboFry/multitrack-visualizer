use super::{font::*, RGB};
use image::RgbImage;

/// Draw a single pixel, with a given colour, to the screen at a given point
pub fn pixel(frame: &mut RgbImage, x: u32, y: u32, colour: RGB) {
	let p = frame.get_pixel_mut(x as u32, y as u32);
	p.0[0] = colour[0];
	p.0[1] = colour[1];
	p.0[2] = colour[2];
}

/// Draw a single letter to the screen based on the blit32 font
fn letter(frame: &mut RgbImage, x: u32, y: u32, letter: u32, colour: RGB) {
	for line_offset in 0..FONT_HEIGHT {
		for letter_offset in 0..FONT_WIDTH {
			let shift = (line_offset * FONT_WIDTH) + letter_offset;
			// Shift the bits and mask everything but the smallest bit
			// (essentially a boolean at this point)
			let chr = (letter >> shift) & 0b00000001;
			if chr == 1 {
				pixel(frame, x + letter_offset, y + line_offset, colour);
			}
		}
	}
}

/// Draw a string of text to the screen.
/// This will ignore any characters outside of the range of valid characters.
pub fn text(frame: &mut RgbImage, x: u32, y: u32, text: &str) {
	text_colour(frame, x, y, text, [0xff, 0xff, 0xff]);
}

pub fn text_colour(frame: &mut RgbImage, x: u32, y: u32, text: &str, colour: RGB) {
	text.chars()
		.filter_map(|letter| {
			let code = letter as usize;
			if code < 32 {
				return None;
			}
			let index = code - 32;
			if index > 95 {
				return None;
			}
			Some(FONT[index])
		})
		.enumerate()
		.for_each(|(tx, index)| {
			letter(frame, (tx as u32 * FONT_SEPARATION) + x, y, index, colour);
		});
}

pub fn rect(frame: &mut RgbImage, x1: u32, y1: u32, x2: u32, y2: u32, colour: RGB) {
	for y in y1..y2 {
		for x in x1..x2 {
			pixel(frame, x, y, colour);
		}
	}
}

pub fn rect_gradient(frame: &mut RgbImage, x1: u32, y1: u32, x2: u32, y2: u32, colour: RGB) {
	let mut col_prev = colour;

	for y in y1..y2 {
		for x in x1..x2 {
			pixel(frame, x, y, col_prev);
		}

		col_prev = [
			col_prev[0] - (y % 3 == 0 && col_prev[0] > 0) as u8,
			col_prev[1] - (y % 3 == 1 && col_prev[1] > 0) as u8,
			col_prev[2] - (y % 3 == 2 && col_prev[2] > 0) as u8,
		];
	}
}

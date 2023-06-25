use crate::SCREEN_WIDTH;

use super::{font::*, RGB};
use rayon::prelude::*;

/// Set every pixel on the screen to black. This task is parallelised
pub fn clear(frame: &mut [u8]) {
	frame.into_par_iter().for_each(|pixel| {
		*pixel = 0x00;
	});
}

/// Determine the index for any given point on the screen.
/// This factors in the fact that each pixel uses 4 bytes for colour (rgba).
fn get_index(x: usize, y: usize) -> usize {
	(x + (y * *SCREEN_WIDTH as usize)) * 4
}

/// Draw a single pixel, with a given colour, to the screen at a given point
pub fn pixel(frame: &mut [u8], x: usize, y: usize, colour: RGB) {
	let idx = get_index(x, y);

	if idx >= frame.len() {
		return;
	}

	frame[idx] = colour[0];
	frame[idx + 1] = colour[1];
	frame[idx + 2] = colour[2];
	frame[idx + 3] = 0xff;
}

/// Draw a single letter to the screen based on the blit32 font
fn letter(frame: &mut [u8], x: usize, y: usize, letter: u32, colour: RGB) {
	for line_offset in 0..FONT_HEIGHT {
		for letter_offset in 0..FONT_WIDTH {
			let shift = (line_offset * FONT_WIDTH) + letter_offset;
			// Shift the bits and mask everything but the smallest bit
			// (essentially a boolean at this point)
			let chr = (letter >> shift) & 0b00000001;
			if chr == 1 {
				pixel(
					frame,
					x + letter_offset as usize,
					y + line_offset as usize,
					colour,
				);
			}
		}
	}
}

/// Draw a string of text to the screen.
/// This will ignore any characters outside of the range of valid characters.
pub fn text(frame: &mut [u8], x: usize, y: usize, text: &str) {
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
			letter(
				frame,
				(tx * FONT_SEPARATION) + x,
				y,
				index,
				[0xff, 0xff, 0xff],
			);
		});
}

pub fn rect(frame: &mut [u8], x1: usize, y1: usize, x2: usize, y2: usize, colour: RGB) {
	for x in x1..x2 {
		for y in y1..y2 {
			pixel(frame, x, y, colour);
		}
	}
}

use crate::{SCREEN_FRAME_RATE, SCREEN_HEIGHT, SCREEN_SCALE, SCREEN_WIDTH};
use fast_image_resize as fr;
use image::RgbImage;
use ndarray::Array3;
use std::{num::NonZeroU32, path::PathBuf};
use video_rs::{Encoder, EncoderSettings, Locator, Time};

pub struct Encoding {
	pub encoder: Encoder,
	pub position: Time,
	pub frame_duration: Time,
	pub resizer: fr::Resizer,
}

impl Encoding {
	pub fn new(video_file_out: &str) -> Self {
		let width = *SCREEN_WIDTH * *SCREEN_SCALE;
		let height = *SCREEN_HEIGHT * *SCREEN_SCALE;
		let destination: Locator = PathBuf::from(video_file_out).into();
		let settings = EncoderSettings::for_h264_yuv420p(width as usize, height as usize, false);
		let encoder = Encoder::new(&destination, settings).expect("Failed to create encoder");

		Encoding {
			encoder,
			position: Time::zero(),
			frame_duration: Time::from_nth_of_a_second(*SCREEN_FRAME_RATE),
			resizer: fr::Resizer::new(fr::ResizeAlg::Nearest),
		}
	}

	pub fn resize_frame(&mut self, frame: &mut RgbImage) -> Vec<u8> {
		if *SCREEN_SCALE == 1 {
			return frame.as_raw().to_vec();
		}

		let src_image = fr::Image::from_slice_u8(
			NonZeroU32::new(*SCREEN_WIDTH).unwrap(),
			NonZeroU32::new(*SCREEN_HEIGHT).unwrap(),
			frame,
			fr::PixelType::U8x3,
		)
		.unwrap();

		let mut dst_image = fr::Image::new(
			NonZeroU32::new(*SCREEN_WIDTH * *SCREEN_SCALE).unwrap(),
			NonZeroU32::new(*SCREEN_HEIGHT * *SCREEN_SCALE).unwrap(),
			fr::PixelType::U8x3,
		);

		// Get mutable view of destination image data
		let mut dst_view = dst_image.view_mut();
		self.resizer
			.resize(&src_image.view(), &mut dst_view)
			.unwrap();

		dst_image.buffer().to_vec()
	}

	pub fn render_frame(&mut self, frame: &mut RgbImage) {
		let pixels = self.resize_frame(frame);
		let ef: Array3<u8> = ndarray::Array3::from_shape_vec(
			(
				(*SCREEN_HEIGHT * *SCREEN_SCALE) as usize,
				(*SCREEN_WIDTH * *SCREEN_SCALE) as usize,
				3,
			),
			pixels,
		)
		.unwrap();

		self.encoder.encode(&ef, &self.position).unwrap();
		self.update_position();
	}

	pub fn update_position(&mut self) {
		self.position = self.position.aligned_with(&self.frame_duration).add();
	}

	pub fn flush(&mut self) {
		self.encoder.finish().unwrap();
	}
}

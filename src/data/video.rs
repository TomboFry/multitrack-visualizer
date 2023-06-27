use crate::{SCREEN_FRAME_RATE, SCREEN_HEIGHT, SCREEN_SCALE, SCREEN_WIDTH};
use fast_image_resize as fr;
use image::RgbImage;
use ndarray::Array3;
use std::num::NonZeroU32;
use video_rs::{Encoder, Time};

pub struct Encoding {
	pub encoder: Encoder,
	pub position: Time,
	pub frame_duration: Time,
	pub resizer: fr::Resizer,
	pub size_src: (NonZeroU32, NonZeroU32),
	pub size_dst: (NonZeroU32, NonZeroU32),
}

impl Encoding {
	pub fn new(encoder: Encoder) -> Self {
		Encoding {
			encoder,
			position: Time::zero(),
			frame_duration: Time::from_nth_of_a_second(*SCREEN_FRAME_RATE),
			size_src: (
				NonZeroU32::new(*SCREEN_WIDTH).unwrap(),
				NonZeroU32::new(*SCREEN_HEIGHT).unwrap(),
			),
			size_dst: (
				NonZeroU32::new(*SCREEN_WIDTH * *SCREEN_SCALE).unwrap(),
				NonZeroU32::new(*SCREEN_HEIGHT * *SCREEN_SCALE).unwrap(),
			),
			resizer: fr::Resizer::new(fr::ResizeAlg::Nearest),
		}
	}

	pub fn resize_frame(&mut self, frame: &mut RgbImage) -> Vec<u8> {
		if *SCREEN_SCALE == 1 {
			return frame.as_raw().to_vec();
		}

		let src_image =
			fr::Image::from_slice_u8(self.size_src.0, self.size_src.1, frame, fr::PixelType::U8x3)
				.unwrap();

		let mut dst_image = fr::Image::new(self.size_dst.0, self.size_dst.1, fr::PixelType::U8x3);

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
				self.size_dst.1.get() as usize,
				self.size_dst.0.get() as usize,
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

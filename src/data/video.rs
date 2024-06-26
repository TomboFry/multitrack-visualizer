use crate::{SCREEN_FRAME_RATE, SCREEN_HEIGHT, SCREEN_SCALE, SCREEN_WIDTH};
use fast_image_resize::{
	images::{Image, ImageRef},
	PixelType, ResizeAlg, ResizeOptions, Resizer,
};
use image::RgbImage;
use ndarray::Array3;
use std::path::PathBuf;
use video_rs::{encode::Settings, Encoder, Location, Time};

pub struct Encoding {
	pub encoder: Encoder,
	pub position: Time,
	pub frame_duration: Time,
	pub resizer: Resizer,
	pub resize_options: ResizeOptions,
}

impl Encoding {
	pub fn new(video_file_out: &str) -> Self {
		let width = *SCREEN_WIDTH * *SCREEN_SCALE;
		let height = *SCREEN_HEIGHT * *SCREEN_SCALE;
		let destination: Location = PathBuf::from(video_file_out).into();
		let settings = Settings::preset_h264_yuv420p(width as usize, height as usize, false);
		let encoder = Encoder::new(&destination, settings).expect("Failed to create encoder");
		let resize_options = ResizeOptions::new().resize_alg(ResizeAlg::Nearest);

		Encoding {
			encoder,
			position: Time::zero(),
			frame_duration: Time::from_nth_of_a_second(*SCREEN_FRAME_RATE),
			resizer: Resizer::new(),
			resize_options,
		}
	}

	pub fn resize_frame(&mut self, frame: &mut RgbImage) -> Vec<u8> {
		if *SCREEN_SCALE == 1 {
			return frame.as_raw().to_vec();
		}

		let src_image =
			ImageRef::new(*SCREEN_WIDTH, *SCREEN_HEIGHT, frame, PixelType::U8x3).unwrap();

		let mut dst_image = Image::new(
			*SCREEN_WIDTH * *SCREEN_SCALE,
			*SCREEN_HEIGHT * *SCREEN_SCALE,
			PixelType::U8x3,
		);

		self.resizer
			.resize(&src_image, &mut dst_image, &self.resize_options)
			.unwrap();

		dst_image.into_vec()
	}

	pub fn render_frame(&mut self, buffer: &mut RgbImage) {
		let pixels = self.resize_frame(buffer);
		let frame: Array3<u8> = ndarray::Array3::from_shape_vec(
			(
				(*SCREEN_HEIGHT * *SCREEN_SCALE) as usize,
				(*SCREEN_WIDTH * *SCREEN_SCALE) as usize,
				3,
			),
			pixels,
		)
		.unwrap();

		self.encoder.encode(&frame, self.position).unwrap();
		self.update_position();
	}

	pub fn update_position(&mut self) {
		self.position = self.position.aligned_with(self.frame_duration).add();
	}

	pub fn flush(&mut self) {
		self.encoder.finish().unwrap();
	}
}

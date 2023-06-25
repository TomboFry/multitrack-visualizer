use crate::{SCREEN_HEIGHT, SCREEN_SCALE, SCREEN_WIDTH};
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::PhysicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use winit_input_helper::WinitInputHelper;

pub fn create_window() -> Result<(Window, WinitInputHelper, EventLoop<()>, Pixels), Error> {
	let event_loop = EventLoop::new();
	let input = WinitInputHelper::new();

	let width = *SCREEN_WIDTH;
	let height = *SCREEN_HEIGHT;
	let scale = *SCREEN_SCALE;

	// Create Window
	let size = PhysicalSize::new(width * scale, height * scale);
	let window = WindowBuilder::new()
		.with_inner_size(size)
		.with_title("Multitrack Visualizer")
		.with_resizable(true)
		.build(&event_loop)
		.unwrap();

	let surface_texture = SurfaceTexture::new(width, height, &window);

	// Create world and display for world
	let mut pixels = Pixels::new(width, height, surface_texture)?;

	pixels.resize_surface(width * scale, height * scale)?;

	Ok((window, input, event_loop, pixels))
}

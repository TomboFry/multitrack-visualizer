use data::{
	image::clear_output_folder,
	song::{Song, Window},
};
use display::draw;
use lazy_static::lazy_static;
use winit::{
	event::{Event, VirtualKeyCode},
	event_loop::ControlFlow,
};

mod data;
mod display;

lazy_static! {
	pub static ref WINDOW: Window = Window::load_from_file();
	pub static ref SCREEN_WIDTH: u32 = WINDOW.width;
	pub static ref SCREEN_HEIGHT: u32 = WINDOW.height;
	pub static ref SCREEN_SCALE: u32 = WINDOW.scale;
}

fn main() {
	// Step 1: Clear output folder of images
	clear_output_folder().unwrap();

	// Step 2: Render waveforms
	let (window, mut input, event_loop, mut pixels) = display::window::create_window().unwrap();
	let mut song = Song::load_from_file();
	song.load_tracks_into_memory();

	event_loop.run(move |event, _, control_flow| {
		if let Event::RedrawRequested(_) = event {
			let frame = pixels.frame_mut();
			draw::clear(frame);
			song.draw(frame);

			if pixels
				.render()
				.map_err(|e| eprintln!("pixels.render() failed: {}", e))
				.is_err()
			{
				*control_flow = ControlFlow::Exit;
				return;
			}
		}

		if input.update(&event) {
			// Close events
			if input.key_pressed(VirtualKeyCode::Escape) || input.close_requested() {
				*control_flow = ControlFlow::Exit;
				return;
			}

			song.update(&input);
			window.request_redraw();
		}
	});
}

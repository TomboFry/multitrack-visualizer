use symphonia::core::{
	codecs::{Decoder, CODEC_TYPE_NULL},
	formats::{FormatOptions, FormatReader, Track},
	io::MediaSourceStream,
	meta::MetadataOptions,
	probe::Hint,
};

pub fn load_track_into_memory(path: &str) -> (Box<dyn FormatReader>, Track, Box<dyn Decoder>) {
	// Open the media source.
	let src = std::fs::File::open(&path);

	let print_error = |error: &str| format!("Could not load track \"{path}\" - Error: {error}");

	if let Err(err) = src {
		println!("\n{}\n", print_error(&err.to_string()));
		std::process::exit(1);
	}

	let src = src.unwrap();

	// Create the media source stream.
	let mss = MediaSourceStream::new(Box::new(src), Default::default());

	// Create a probe hint using the file's extension. [Optional]
	let ext = path.split(".").collect::<Vec<&str>>();
	let mut hint = Hint::new();
	hint.with_extension(ext[ext.len() - 1]);

	// Use the default options for metadata and format readers.
	let meta_opts: MetadataOptions = Default::default();
	let fmt_opts: FormatOptions = Default::default();

	// Probe the media source.
	let probed = symphonia::default::get_probe()
		.format(&hint, mss, &fmt_opts, &meta_opts)
		.expect(&print_error("Unsupported format"));

	// Get the instantiated format reader.
	let format = probed.format;

	// Find the first audio track with a known (decodeable) codec.
	let track = format
		.tracks()
		.iter()
		.find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
		.expect(&print_error("No supported audio tracks"))
		.clone();

	let codec = track.codec_params.codec.to_string();

	let decoder = symphonia::default::get_codecs()
		.make(&track.codec_params, &Default::default())
		.expect(&print_error(&format!("{codec} is an unsupported codec")));

	(format, track, decoder)
}

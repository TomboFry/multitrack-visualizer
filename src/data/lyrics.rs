#[derive(Debug)]
pub struct LyricLine {
	time_start: f64,
	time_end: f64,
	text: String,
}

#[derive(Debug)]
pub struct Lyrics {
	lines: Vec<LyricLine>,
}

impl Lyrics {
	pub fn new(path: &Option<String>) -> Option<Self> {
		if path.is_none() {
			return None;
		}

		let file = std::fs::read_to_string(path.as_ref().unwrap());
		if file.is_err() {
			panic!("Could not load lyrics file");
		}
		let file = file.unwrap();
		let lines_raw = file.lines();

		let mut lines: Vec<LyricLine> = vec![];
		let re = regex::Regex::new(r"^\[(\d\d):(\d\d.\d\d)\](.*)$").unwrap();
		for haystack in lines_raw {
			if let Some(capture) = re.captures(haystack) {
				let (_, [minute, second, lyric]) = capture.extract();
				let time_mins = minute.parse::<f64>().unwrap();
				let time_secs = second.parse::<f64>().unwrap();
				let time = (time_mins * 60.0) + time_secs;
				if lines.len() > 0 {
					let len = lines.len();
					lines[len - 1].time_end = time;
				}
				if lyric.is_empty() {
					continue;
				}
				lines.push(LyricLine {
					time_start: time,
					time_end: 999999.0,
					text: lyric.trim().to_uppercase().to_owned(),
				});
			}
		}

		Some(Lyrics { lines })
	}

	pub fn find_line(&self, time: f64) -> Option<&str> {
		for line in &self.lines {
			if line.time_start <= time && line.time_end >= time {
				return Some(&line.text);
			}
		}

		None
	}
}

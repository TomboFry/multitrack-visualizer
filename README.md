# Multitrack Visualizer

A small tool that loads multiple audio files at once and simultaneously renders
each of their waveforms to PNG.

I plan on using this for [my YouTube channel](https://youtube.com/TomboFry),
where I upload chiptune music. I figured it would be a good visualisation tool,
and different from my usual screen capture of FL Studio.

## Screenshot

I've uploaded [a full-song to YouTube using this software](https://www.youtube.com/watch?v=9mGbqnYR_UI), so you can see what the final output looks like!

![](./screenshot.png)

## Usage

```
multitrack-visualizer.exe [OPTIONS]

Options:
  -s, --song /path/to/song.json       JSON config file for all tracks, colours, and audio files (default: ./song.json)
  -w, --window /path/to/window.json   JSON config file for size and scaling of the output video (default: ./window.json)
  -h, --help                          Print this help
  -V, --version                       Print version
```

## JSON Config Format

### Song.json

* `channels`, an array of objects, where:
  * `name` is the channel name, displayed on screen
  * `file` is a path name to the audio file, and
  * `colour` (optional) - contains the Red, Green, and Blue colour values
    (0-255). Defaults to black, ie. `[0, 0, 0]`
  * `use_alignment`: (optional) - Attempt to align the waveform on each frame.
    Non-tonal channels or low frequency audio might look better displayed when
    this is turned off. Defaults to `true`
* `video_file_out` is a path name to the video that will be output.

```json
{
  "channels": [
    {
      "name": "Channel Name",
      "file": "/path/to/audio-file.wav",
      "colour": [0, 2, 255],
      "use_alignment": false
    }
  ],
  "video_file_out": "/path/to/output.mp4"
}
```

### Window.json

* `width` and `height` are the base resolution for the video file.
* `scale` is how many times the resolution should be scaled (integer).
  * For example, the default is 480x270 at a scale of 4, which means the final
    output resolution is 1920x1080.
* `frame_rate` the frame rate of the output video

```json
{
  "width": 480,
  "height": 270,
  "scale": 4,
  "frame_rate": 30
}
```

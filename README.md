# Pixelbomber
A program to nuke pixelflut servers

This client is largely inspired by [pixelpwnr](https://github.com/timvisee/pixelpwnr), although I made heavy modifications to decrease the bottlenecks.

# Installation

Either using cargo:

```commandline
cargo install pixelbomber
```

or by cloning and then building:

```commandline
git clone https://github.com/fabi321/pixelbomber.git
cd pixelbomber
cargo build --release
```

# Features
- Concurrent writing pipes
- Animated images with consecutive images
- Control over render sizes and offsett
- Faster than [pixelpwnr](https://github.com/timvisee/pixelpwnr) (in my case by more than a Factor of 8)
- Linux, Windows and MacOS
- Same cli as pixelpwnr
- Support for both gray pixel command as well as offset command, enable with `--offset` and `--gray`
- Support for automated feature and size detection, on by default
- Support for binary pixel commands in the `PBxyrgba` format (x and y are u16 le encoded)
- Support for input streams
- Suopport for v4l cameras
- Fast image to pixel commands encoder

# Get images from stream

By using `-` as sole image file path, you can pipe in images from stdin. Pixelbomber expects bitmap files as input.
You can specify, how many images can be processed in parallel with the `--workers` flag.

## Example using ffmpeg
```commandline
ffmpeg -re -i <video_file> -f image2pipe -c:v bmp - | cargo run --release -- <host> -
```

Some ffmpeg tips:
- use `-re` if the input is a video file. This forces ffmpeg to play it at most at realtime
- use `-stream_loop -1` before `-i` to repeat a video indefinitely

## Tradeoff
If you want to only loop a static video, specifying the video frames as images is faster, as it only encodes them once.

# Get images from camera

By using `/dev/video*` as sole image file path, you can use v4l cameras as input.
You can also use `--test-green-screen` to get the current pixel value on position 0,0,
and use that value with `--green-screen` to filter out the green screen.

Tip: OBS has a virtual camera feature.

# Get images from video

By only supplying one file, and using the `--video` flag, pixelbomber can read all frames from that video as images.
This will result in pixelbomber precompiling all images into commands, and fluting them afterward. WARNING: This may
consume large amounts of RAM (~50GB for 1min FullHD 30fps video). Pixelbombewr will stop encoding new frames if the
free system memory drops below 1GB. The number of frames processed in parallel is configured via the `--workers` flag.

## Comparison with stream method

Compared to reading from stdin, this method uses less cpu while fluting and doesn't drop any frames. It does use large
amounts of RAM though.

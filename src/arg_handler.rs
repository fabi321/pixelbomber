use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    /// The host to pwn "host:port"
    pub host: String,

    /// Image paths
    pub image: Vec<String>,

    /// Draw width [default: screen width]
    #[arg(short, long, value_name = "PIXELS")]
    pub width: Option<u32>,

    /// Draw height [default: screen height]
    #[arg(short = 'q', long, value_name = "PIXELS")]
    pub height: Option<u32>,

    /// Draw X offset
    #[arg(short, long, value_name = "PIXELS", default_value = "0")]
    pub x: u32,

    /// Draw Y offset
    #[arg(short, long, value_name = "PIXELS", default_value = "0")]
    pub y: u32,

    /// Number of concurrent threads [default: CPUs]
    #[arg(short, long, value_name = "THREADS")]
    pub count: Option<u32>,

    /// Frames per second with multiple images
    #[arg(short = 'r', long, value_name = "FPS", default_value = "1")]
    pub fps: f32,

    /// Disable automatic detection of supported features
    #[arg(short, long)]
    pub feature_detection: bool,

    /// Enable usage of offset command
    #[arg(short, long)]
    pub offset: bool,

    /// Enable usage of `PX X Y gg` command
    #[arg(short, long)]
    pub gray: bool,

    /// Enable usage of alpha command for pixels with alpha > 0 and < 255
    #[arg(short, long)]
    pub alpha: bool,

    /// Bind address to use for communication
    #[arg(long)]
    pub bind_addr: Option<String>,

    /// Disable shuffling of draw commands (recommended for video streams)
    #[arg(short, long)]
    pub shuffle: bool,

    /// Number of workers for turning stream images into pixel commands
    #[arg(long, default_value = "5")]
    pub workers: u32,

    /// Resize images rather than cropping them
    #[arg(long)]
    pub resize: bool,

    /// input is a video (only works with one input file, and requires ffmpeg in $PATH)
    #[arg(short, long)]
    pub video: bool,

    /// Use the PBxxyyrgba format with le encoding
    #[arg(long)]
    pub le_rgba: bool,

    /// Run continuously (ignore EOF if using stdin)
    #[arg(long)]
    pub continuous: bool,
}

pub fn parse() -> Args {
    Args::parse()
}

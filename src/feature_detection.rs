use crate::client::Client;
use std::io::Result;
use std::net::TcpStream;

/// Detected feature set of a pixelflut server.
pub struct Features {
    /// Canvas width
    pub width: u32,
    /// Canvas height
    pub height: u32,
    /// If the `OFFSET` command is supported. NOTE: this is derived from the HELP command
    pub offset: bool,
    /// If the `PX x y gg` command is supported. NOTE: this is derived from the HELP command
    pub px_gray: bool,
    /// If the binary `PB` command is supported. NOTE: this is derived from the HELP command
    pub binary: bool,
}

/// Detect the features supported by a server
/// NOTE: command detection is based on the `HELP` command, and might not work
/// If you do notice that a server has a certain feature, but this is not reflected in the result,
/// feel free to open an issue
pub fn feature_detection(stream: TcpStream) -> Result<Features> {
    let mut client = Client::new(stream);
    let (width, height) = client.read_screen_size()?;
    let mut features = Features {
        width,
        height,
        offset: false,
        px_gray: false,
        binary: false,
    };
    let help_text = client.read_help()?;
    for line in help_text.split('\n') {
        let lowered = line.to_lowercase();
        let trimmed = lowered.trim_start();
        // breakwater format
        if trimmed.starts_with("offset") {
            features.offset = true
        // breakwater format
        } else if trimmed.starts_with("px x y gg") {
            features.px_gray = true
        // wellenbrecher format
        } else if trimmed.starts_with("grayscale") {
            features.px_gray = true
        } else if trimmed.contains("pbxyrgba") {
            features.binary = true
        }
    }
    Ok(features)
}

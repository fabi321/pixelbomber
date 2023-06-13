use crate::client::Client;
use std::io::Result;

pub struct Features {
    pub width: u32,
    pub height: u32,
    pub offset: bool,
    pub px_gray: bool,
}

pub fn feature_detection(host: &str) -> Result<Features> {
    let mut client = Client::connect(host)?;
    let (width, height) = client.read_screen_size()?;
    let help_text = client.read_help()?;
    let mut offset = false;
    let mut px_gray = false;
    for line in help_text.split('\n') {
        if line.to_lowercase().starts_with("offset") {
            offset = true
        } else if line.to_lowercase().starts_with("px x y gg") {
            px_gray = true
        }
    }
    Ok(Features {
        width,
        height,
        offset,
        px_gray,
    })
}

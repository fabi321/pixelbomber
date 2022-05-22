use std::path::Path;

use image::DynamicImage;
use rand::{thread_rng, prelude::SliceRandom};

pub type Command = Vec<u8>;
pub type Commands = Vec<Command>;
pub type CommandLib = Vec<Commands>;

pub struct ImageConfig {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub x_offset: u32,
    pub y_offset: u32,
}

fn image_to_commands(mut image: DynamicImage, config: &ImageConfig) -> Commands {
    let cropped_image = if let (Some(width), Some(height)) = (config.width, config.height) {
        image.crop(0, 0, width, height)
    } else {
        image
    };
    let rgba_image = cropped_image.to_rgba8();
    let mut result = Vec::new();
    for (x, y, pixel) in rgba_image.enumerate_pixels() {
        if pixel.0[3] > 0 {
            let mut rgba = String::new();
            for (i, c) in pixel.0.into_iter().enumerate() {
                if i < 3 || c != 255 {
                    rgba += &format!("{:02x}", c);
                }
            }
            let x_pos = x + config.x_offset;
            let y_pos = y + config.y_offset;
            let command_string = format!("PX {} {} {}\n", x_pos, y_pos, rgba);
            result.push(command_string.into_bytes())
        }
    }
    // shuffle all entries
    let mut rng = thread_rng();
    result.shuffle(&mut rng);
    // merge 70 pixel commands into one batch commands until there are no pixel commands left
    let mut combined_results = Vec::new();
    while !result.is_empty() {
        let mut current_combined = Vec::new();
        for _ in 0..70 {
            if let Some(cmd) = result.pop() {
                current_combined.extend(cmd)
            }
        }
        combined_results.push(current_combined)
    }
    combined_results
}

fn from_images(images: Vec<DynamicImage>, config: &ImageConfig) -> CommandLib {
    images
        .into_iter()
        .map(|image| image_to_commands(image, config))
        .collect()
}

pub fn load(paths: Vec<&str>, config: &ImageConfig) -> CommandLib {
    let images = paths
        .into_iter()
        .map(|path_str| {
            let path = Path::new(path_str);
            if !path.is_file() {
                panic!("The path \"{}\" either doesn't exist or isn't a file", path_str)
            }
            image::open(path).expect("coudn't load image")
        })
        .collect();
    from_images(images, config)
}

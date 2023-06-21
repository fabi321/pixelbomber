use std::path::Path;

use image::DynamicImage;
use rand::{prelude::SliceRandom, thread_rng};

pub type Command = Vec<u8>;
pub type CommandLib = Vec<Command>;

/// Configuration for how to place a picture, and what features to use
pub struct ImageConfig {
    /// Largest width of the image.
    /// NOTE: this needs to be `canvas_width - x_offset` to crop at the canvas edges
    pub width: Option<u32>,
    /// Largest height of the image.
    /// NOTE: this needs to be `canvas_height - y_offset` to crop at the canvas edges
    pub height: Option<u32>,
    /// At what x offset to place the image
    pub x_offset: u32,
    /// At what y offset to place the image
    pub y_offset: u32,
    /// If the `OFFSET` command should be used
    pub offset_usage: bool,
    /// If the `PX x y gg` command should be used
    pub gray_usage: bool,
    /// Shuffle draw commands (RECOMMENDED)
    pub shuffle: bool,
}

const CHUNK_SIZE: u32 = 10;

fn id_for_chunk_x_y(x: u32, y: u32, width: u32) -> usize {
    (x + y * width / CHUNK_SIZE) as usize
}

fn id_for_px(x: u32, y: u32, width: u32) -> usize {
    id_for_chunk_x_y(x / CHUNK_SIZE, y / CHUNK_SIZE, width)
}

fn image_to_commands(mut image: DynamicImage, config: &ImageConfig) -> Command {
    if config.width.is_some() != config.height.is_some() {
        println!("Warning: Only setting width or height doesn't crop the image!")
    }
    let cropped_image = if let (Some(width), Some(height)) = (config.width, config.height) {
        image.crop(0, 0, width, height)
    } else {
        image
    };
    let rgba_image = cropped_image.to_rgba8();
    let width = cropped_image.width();
    let height = cropped_image.height();
    let mut full_result = Vec::with_capacity((width * height) as usize);
    let mut offset_result =
        Vec::with_capacity(((width / CHUNK_SIZE) * (height / CHUNK_SIZE)) as usize);
    for row in 0..(cropped_image.height() + CHUNK_SIZE - 1) / CHUNK_SIZE {
        for column in 0..(width + CHUNK_SIZE - 1) / CHUNK_SIZE {
            offset_result.push(
                format!(
                    "OFFSET {} {}\n",
                    column + config.x_offset,
                    row + config.y_offset
                )
                .into_bytes(),
            )
        }
    }
    let mut relevant_pixels = 0;
    for (x, y, pixel) in rgba_image.enumerate_pixels() {
        if pixel.0[3] > 0 {
            let mut rgba = String::new();
            for (i, c) in pixel.0.into_iter().enumerate() {
                if i < 3 || c != 255 {
                    rgba += &format!("{:02x}", c);
                }
            }
            if config.gray_usage && pixel.0[0] == pixel.0[1] && pixel.0[1] == pixel.0[2] {
                rgba = format!("{:02x}", pixel.0[0]);
                if pixel.0[3] != 255 {
                    rgba += &format!("{:02x}", pixel.0[3]);
                }
            }
            let x_pos = x + config.x_offset;
            let y_pos = y + config.y_offset;
            let command_string = format!("PX {} {} {}\n", x_pos, y_pos, rgba);
            full_result.push(command_string.into_bytes());
            let offset_vec = offset_result.get_mut(id_for_px(x, y, width)).unwrap();
            offset_vec.extend(
                format!("PX {} {} {}\n", x % CHUNK_SIZE, y % CHUNK_SIZE, rgba).into_bytes(),
            );
            relevant_pixels += 1;
        }
    }
    if config.shuffle {
        // shuffle all entries
        let mut rng = thread_rng();
        full_result.shuffle(&mut rng);
        offset_result.shuffle(&mut rng);
    }
    let combined_full_results: Command = full_result.into_iter().flatten().collect();
    let combined_offset_result: Command = offset_result
        .into_iter()
        .filter(|v| v.len() > 18)
        .flatten()
        .collect();
    let final_result =
        if !config.offset_usage || combined_full_results.len() < combined_offset_result.len() {
            combined_full_results
        } else {
            combined_offset_result
        };
    let optimizations = if config.gray_usage && config.offset_usage {
        "using both gray and offset optimizations"
    } else if config.gray_usage {
        "using only gray optimization"
    } else if config.offset_usage {
        "using only offset optimization"
    } else {
        "using no optimizations"
    };
    println!(
        "Processed image, pixel commands bytes: {}, {} bytes per pixel, {optimizations}",
        final_result.len(),
        final_result.len() as f32 / relevant_pixels as f32
    );
    final_result
}

fn from_images(images: Vec<DynamicImage>, config: &ImageConfig) -> CommandLib {
    images
        .into_iter()
        .map(|image| image_to_commands(image, config))
        .collect()
}

/// Load image(s) from paths, parsing them into ready to use command chains
pub fn load(paths: Vec<&str>, config: &ImageConfig) -> CommandLib {
    let images = paths
        .into_iter()
        .map(|path_str| {
            let path = Path::new(path_str);
            if !path.is_file() {
                panic!(
                    "The path \"{}\" either doesn't exist or isn't a file",
                    path_str
                )
            }
            image::open(path).expect("coudn't load image")
        })
        .collect();
    from_images(images, config)
}

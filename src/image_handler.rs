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
    pub offset_usage: bool,
    pub gray_usage: bool,
}

const CHUNK_SIZE: u32 = 10;

fn id_for_chunk_x_y(x: u32, y: u32, width: u32) -> usize {
    (x + y * width / CHUNK_SIZE) as usize
}

fn id_for_px(x: u32, y: u32, width: u32) -> usize {
    id_for_chunk_x_y(x / CHUNK_SIZE, y / CHUNK_SIZE, width)
}

fn image_to_commands(mut image: DynamicImage, config: &ImageConfig) -> Commands {
    let cropped_image = if let (Some(width), Some(height)) = (config.width, config.height) {
        image.crop(0, 0, width, height)
    } else {
        image
    };
    let rgba_image = cropped_image.to_rgba8();
    let width = cropped_image.width();
    let height = cropped_image.height();
    let mut full_result = Vec::with_capacity((width * height) as usize);
    let mut offset_result = Vec::with_capacity(((width / CHUNK_SIZE) * (height / CHUNK_SIZE)) as usize);
    for row in 0..(cropped_image.height() + CHUNK_SIZE - 1) / CHUNK_SIZE {
        for column in 0..(width + CHUNK_SIZE - 1) / CHUNK_SIZE {
            offset_result.push(format!("OFFSET {} {}\n", column + config.x_offset, row + config.y_offset).into_bytes())
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
            offset_vec.extend(format!("PX {} {} {}\n", x % CHUNK_SIZE, y % CHUNK_SIZE, rgba).into_bytes());
            relevant_pixels += 1;
        }
    }
    // shuffle all entries
    let mut rng = thread_rng();
    full_result.shuffle(&mut rng);
    offset_result.shuffle(&mut rng);
    // merge 70 pixel commands into one batch commands until there are no pixel commands left
    let mut combined_results = Vec::new();
    while !full_result.is_empty() {
        let mut current_combined = Vec::new();
        for _ in 0..70 {
            if let Some(cmd) = full_result.pop() {
                current_combined.extend(cmd)
            }
        }
        combined_results.push(current_combined)
    }
    offset_result = offset_result.into_iter().filter(|v| v.len() > 18).collect();
    let combined_len: usize = combined_results.iter().map(|v| v.len()).sum();
    let offset_len: usize = offset_result.iter().map(|v| v.len()).sum();
    let (final_result, final_len) = if combined_len < offset_len || !config.offset_usage {
        (combined_results, combined_len)
    } else {
        (offset_result, offset_len)
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
    println!("Processed image, pixel commands bytes: {final_len}, {} bytes per pixel, {optimizations}", final_len as f32 / relevant_pixels as f32);
    final_result
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

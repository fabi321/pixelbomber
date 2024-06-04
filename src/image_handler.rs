use std::path::Path;

use image::DynamicImage;
use rand::{prelude::SliceRandom, thread_rng};

pub type Command = Vec<u8>;
pub type CommandLib = Vec<Command>;

pub struct ImageConfigBuilder {
    width: Option<u32>,
    height: Option<u32>,
    x_offset: u32,
    y_offset: u32,
    offset_usage: bool,
    gray_usage: bool,
    alpha_usage: bool,
    binary_usage: bool,
    shuffle: bool,
}

impl ImageConfigBuilder {
    pub fn new() -> ImageConfigBuilder {
        ImageConfigBuilder {
            width: None,
            height: None,
            x_offset: 0,
            y_offset: 0,
            offset_usage: false,
            gray_usage: false,
            alpha_usage: false,
            binary_usage: false,
            shuffle: true,
        }
    }

    /// Largest width of the image.
    /// NOTE: this needs to be `canvas_width - x_offset` to crop at the canvas edges
    pub fn width(mut self, width: u32) -> ImageConfigBuilder {
        self.width = Some(width);
        self
    }

    /// Largest height of the image.
    /// NOTE: this needs to be `canvas_height - y_offset` to crop at the canvas edges
    pub fn height(mut self, height: u32) -> ImageConfigBuilder {
        self.height = Some(height);
        self
    }

    /// At what x offset to place the image
    pub fn x_offset(mut self, x_offset: u32) -> ImageConfigBuilder {
        self.x_offset = x_offset;
        self
    }

    /// At what y offset to place the image
    pub fn y_offset(mut self, y_offset: u32) -> ImageConfigBuilder {
        self.y_offset = y_offset;
        self
    }

    /// If the `OFFSET` command should be used
    pub fn offset_usage(mut self, offset_usage: bool) -> ImageConfigBuilder {
        self.offset_usage = offset_usage;
        self
    }

    /// If the `PX x y gg` command should be used
    pub fn gray_usage(mut self, gray_usage: bool) -> ImageConfigBuilder {
        self.gray_usage = gray_usage;
        self
    }

    /// If the `PX x y rrggbbaa` command should be used (or alpha ignored)
    pub fn alpha_usage(mut self, alpha_usage: bool) -> ImageConfigBuilder {
        self.gray_usage = alpha_usage;
        self
    }

    /// If the `PBxyrgba` binary command should be used
    pub fn binary_usage(mut self, binary_usage: bool) -> ImageConfigBuilder {
        self.binary_usage = binary_usage;
        self
    }

    /// Shuffle draw commands (RECOMMENDED)
    pub fn shuffle(mut self, shuffle: bool) -> ImageConfigBuilder {
        self.shuffle = shuffle;
        self
    }

    /// Build the image config
    pub fn build(self) -> ImageConfig {
        ImageConfig {
            width: self.width,
            height: self.height,
            x_offset: self.x_offset,
            y_offset: self.y_offset,
            offset_usage: self.offset_usage,
            gray_usage: self.gray_usage,
            alpha_usage: self.alpha_usage,
            binary_usage: self.binary_usage,
            shuffle: self.shuffle,
        }
    }
}

impl Default for ImageConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for how to place a picture, and what features to use
#[derive(Copy, Clone)]
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
    /// Use alpha data (not recommended)
    pub alpha_usage: bool,
    /// Shuffle draw commands (RECOMMENDED)
    pub shuffle: bool,
    /// Use binary representation (Recommended if supported)
    pub binary_usage: bool,
}

const CHUNK_SIZE: u32 = 10;

fn id_for_chunk_x_y(x: u32, y: u32, width: u32) -> usize {
    (x + y * width / CHUNK_SIZE) as usize
}

fn id_for_px(x: u32, y: u32, width: u32) -> usize {
    id_for_chunk_x_y(x / CHUNK_SIZE, y / CHUNK_SIZE, width)
}

fn image_to_commands(mut image: DynamicImage, config: ImageConfig) -> Command {
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
                    column * CHUNK_SIZE + config.x_offset,
                    row * CHUNK_SIZE + config.y_offset
                )
                .into_bytes(),
            )
        }
    }
    let mut relevant_pixels = 0;
    for (x, y, pixel) in rgba_image.enumerate_pixels().filter(|(_, _, p)| p.0[3] > 0) {
        relevant_pixels += 1;
        let x_pos = x + config.x_offset;
        let y_pos = y + config.y_offset;
        if config.binary_usage {
            let x_pos = (x_pos as u16).to_le_bytes();
            let y_pos = (y_pos as u16).to_le_bytes();
            let mut command = vec![b'P', b'B'];
            command.reserve(8);
            command.extend_from_slice(&x_pos);
            command.extend_from_slice(&y_pos);
            command.extend_from_slice(&pixel.0);
            full_result.push(command);
            continue;
        }
        let mut rgba = String::new();
        for (i, c) in pixel.0.into_iter().enumerate() {
            if i < 3 || c != 255 && config.alpha_usage {
                rgba += &format!("{:02x}", c);
            }
        }
        // alpha is not supported if gray is used
        if config.gray_usage
            && (!config.alpha_usage || pixel.0[3] == 255)
            && pixel.0[0] == pixel.0[1]
            && pixel.0[1] == pixel.0[2]
        {
            rgba = format!("{:02x}", pixel.0[0]);
        }
        let x_pos = x + config.x_offset;
        let y_pos = y + config.y_offset;
        let command_string = format!("PX {} {} {}\n", x_pos, y_pos, rgba);
        full_result.push(command_string.into_bytes());
        let offset_vec = offset_result.get_mut(id_for_px(x, y, width)).unwrap();
        offset_vec
            .extend(format!("PX {} {} {}\n", x % CHUNK_SIZE, y % CHUNK_SIZE, rgba).into_bytes());
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
    let final_result = if config.binary_usage
        || !config.offset_usage
        || combined_full_results.len() < combined_offset_result.len()
    {
        combined_full_results
    } else {
        combined_offset_result
    };
    let optimizations = if config.binary_usage {
        "using binary optimization"
    } else if config.gray_usage && config.offset_usage {
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

/// Load image(s) from paths, parsing them into ready to use command chains
pub fn load(paths: Vec<&str>, config: ImageConfig) -> CommandLib {
    let images: Vec<_> = paths
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
    images
        .into_iter()
        .map(|image| image_to_commands(image, config))
        .collect()
}

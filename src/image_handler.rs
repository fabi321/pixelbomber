use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use image::{DynamicImage, ImageError, ImageFormat, Rgba, RgbaImage};
use log::{info, warn};
use rand::rngs::SmallRng;
use rand::{prelude::SliceRandom, SeedableRng};

/// A single image, parsed into commands. Consists of multiple chunks of commands
pub type Command = Vec<Vec<u8>>;
/// A collection of image commands
pub type CommandLib = Vec<Arc<Command>>;

pub use image::imageops::FilterType;

use crate::feature_detection::Features;

/// Format for binary encoded images
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BinaryFormat {
    CoordLERGBA,
}

pub struct ImageConfigBuilder {
    width: Option<u32>,
    height: Option<u32>,
    x_offset: u32,
    y_offset: u32,
    offset_usage: bool,
    gray_usage: bool,
    alpha_usage: bool,
    shuffle: bool,
    binary: Option<BinaryFormat>,
    chunks: usize,
    resize: bool,
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
            shuffle: true,
            binary: None,
            chunks: 1,
            resize: false,
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

    /// If the `PBxyrgba` binary command should be used with 2b LE coordinates and rgba
    pub fn binary_usage(mut self, binary_usage: bool) -> ImageConfigBuilder {
        self.binary = if binary_usage {
            Some(BinaryFormat::CoordLERGBA)
        } else {
            None
        };
        self
    }

    /// Shuffle draw commands (RECOMMENDED)
    pub fn shuffle(mut self, shuffle: bool) -> ImageConfigBuilder {
        self.shuffle = shuffle;
        self
    }

    /// Number of chunks to split the image into
    pub fn chunks(mut self, chunks: usize) -> ImageConfigBuilder {
        if chunks == 0 {
            panic!("Image config chunks have to be greater than 0")
        }
        self.chunks = chunks;
        self
    }

    /// Resize rather than crop the image
    pub fn resize(mut self, resize: bool) -> ImageConfigBuilder {
        self.resize = resize;
        self
    }

    pub fn apply_features(mut self, features: Features) -> ImageConfigBuilder {
        self.width = Some(self.width.unwrap_or(features.width));
        self.height = Some(self.height.unwrap_or(features.height));
        self.offset_usage = self.offset_usage || features.offset;
        self.gray_usage = self.gray_usage || features.px_gray;
        if self.binary.is_none() {
            self.binary = features.binary;
        }
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
            shuffle: self.shuffle,
            binary: self.binary,
            chunks: self.chunks,
            resize: self.resize,
        }
    }
}

impl Default for ImageConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for how to place a picture, and what features to use
#[derive(Copy, Clone, Debug)]
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
    /// Number of chunks
    pub chunks: usize,
    /// Resize rather than crop the image
    pub resize: bool,
    /// Use binary representation (Recommended if supported)
    pub binary: Option<BinaryFormat>,
}

impl Default for ImageConfig {
    fn default() -> Self {
        ImageConfigBuilder::default().build()
    }
}

const CHUNK_SIZE: u32 = 10;
// Longest command: PX xxxx yyyy rrggbbaa\n
const NORMAL_SIZE: usize = 22;
// Longest offset command: PX x y rrggbbaa\n
// assumes chunk size of 10
const OFFSET_SIZE: usize = 16;

#[inline(always)]
fn id_for_chunk_x_y(x: u32, y: u32, chunk_width: u32) -> usize {
    (x + y * chunk_width) as usize
}

#[inline(always)]
fn id_for_px(x: u32, y: u32, chunk_width: u32) -> usize {
    id_for_chunk_x_y(x / CHUNK_SIZE, y / CHUNK_SIZE, chunk_width)
}

pub(crate) fn image_to_commands(mut image: DynamicImage, config: ImageConfig) -> Command {
    if config.width.is_some() != config.height.is_some() {
        warn!("Warning: Only setting width or height doesn't crop the image!")
    }
    let start = Instant::now();
    let cropped_image = if let (Some(width), Some(height)) = (config.width, config.height) {
        #[allow(clippy::if_same_then_else)]
        if width == image.width() && height == image.height() {
            image
        } else if !config.resize && width >= image.width() && height >= image.height() {
            image
        } else if config.resize {
            // Triangle is the fastest, yet reasonably good algorithm
            image.resize_exact(width, height, FilterType::Triangle)
        } else {
            image.crop(0, 0, width, height)
        }
    } else {
        image
    };
    let rgba_image = cropped_image.to_rgba8();
    let (final_result, relevant_pixels) = if config.binary.is_some() {
        get_binary_encoded(&rgba_image, config)
    } else if config.offset_usage {
        // encoding as offset is significantly faster than a full encoding
        // This might result in a less optimized image for sparse images, but the odds are
        // relatively low
        get_offset_encoded(&rgba_image, config)
    } else {
        get_full_encoded(&rgba_image, config)
    };
    let optimizations = if config.binary.is_some() {
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
    let size: usize = final_result.iter().map(|v| v.len()).sum();
    info!(
        "Processed image in {}ms, pixel commands bytes: {}, {} bytes per pixel, {optimizations}",
        start.elapsed().as_millis(),
        size,
        size as f32 / relevant_pixels as f32
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
        .map(|image| Arc::new(image_to_commands(image, config)))
        .collect()
}

/// Load an image from memory and parse it into pixel commands
pub fn load_from_memory(
    input: &[u8],
    config: ImageConfig,
    format: ImageFormat,
) -> Result<Command, ImageError> {
    let image = image::load_from_memory_with_format(input, format)?;
    Ok(image_to_commands(image, config))
}

fn shuffle_collect<T, F: Fn(&T) -> Option<&[u8]>>(
    mut input: Vec<T>,
    size_hint: usize,
    config: ImageConfig,
    conversion: F,
) -> Command {
    if config.shuffle {
        let mut rng = SmallRng::from_entropy();
        input.shuffle(&mut rng)
    }

    let mut result = Vec::with_capacity(config.chunks);

    for _ in 0..config.chunks {
        result.push(Vec::with_capacity(size_hint / config.chunks))
    }

    for (i, entry) in input.into_iter().enumerate() {
        if let Some(extension) = conversion(&entry) {
            // If not shuffled, this will result in all painters painting everywhere,
            // skipping a few pixels (or chunks) all the time
            // This way of splitting ensures an even distribution, even if shuffle is off,
            // offset mode is used and only parts of the canvas are painted
            result[i % config.chunks].extend_from_slice(extension)
        }
    }

    result
}

fn binary_encode(format: &BinaryFormat, x: u32, y: u32, px: &Rgba<u8>) -> Vec<u8> {
    match format {
        BinaryFormat::CoordLERGBA => {
            let x = (x as u16).to_le_bytes();
            let y = (y as u16).to_le_bytes();
            vec![
                b'P', b'B', x[0], x[1], y[0], y[1], px.0[0], px.0[1], px.0[2], px.0[3],
            ]
        }
    }
}

fn get_binary_encoded(rgba_image: &RgbaImage, config: ImageConfig) -> (Command, usize) {
    let mut intermediate =
        Vec::with_capacity(rgba_image.width() as usize * rgba_image.height() as usize);
    let mut relevant_pixels = 0;
    let Some(format) = config.binary else {
        panic!("Binary encode without binary format")
    };
    for (x, y, pixel) in rgba_image.enumerate_pixels() {
        if pixel.0[3] == 0 {
            continue;
        }
        relevant_pixels += 1;
        let x_pos = x + config.x_offset;
        let y_pos = y + config.y_offset;
        intermediate.push(binary_encode(&format, x_pos, y_pos, pixel));
    }
    let result = shuffle_collect(intermediate, relevant_pixels * 10, config, |c| Some(c));
    (result, relevant_pixels)
}

fn get_full_encoded(rgba_image: &RgbaImage, config: ImageConfig) -> (Command, usize) {
    let mut intermediate =
        Vec::with_capacity(rgba_image.width() as usize * rgba_image.height() as usize);
    let mut relevant_pixels = 0;
    let mut size = 0;
    for (x, y, pixel) in rgba_image.enumerate_pixels() {
        let Some(pixel) = get_pixel(pixel, config) else {
            continue;
        };
        relevant_pixels += 1;
        let cmd = pixel_to_command(x + config.x_offset, y + config.y_offset, pixel);
        size += cmd.1;
        intermediate.push(cmd);
    }
    let result = shuffle_collect(intermediate, size, config, |(cmd, len)| Some(&cmd[..*len]));
    (result, relevant_pixels)
}

fn get_offset_encoded(rgba_image: &RgbaImage, config: ImageConfig) -> (Command, usize) {
    let width = rgba_image.width();
    let height = rgba_image.height();
    let chunk_width = width.div_ceil(CHUNK_SIZE);
    let mut intermediate = Vec::with_capacity(id_for_px(width, height, chunk_width) + 1);
    let mut size = 0;
    for row in 0..height.div_ceil(CHUNK_SIZE) {
        for column in 0..width.div_ceil(CHUNK_SIZE) {
            let command = format!(
                "OFFSET {} {}\n",
                column * CHUNK_SIZE + config.x_offset,
                row * CHUNK_SIZE + config.y_offset
            );
            size += command.len();
            intermediate.push(command.into_bytes())
        }
    }
    let mut relevant_pixels = 0;
    for (x, y, pixel) in rgba_image.enumerate_pixels() {
        let Some(pixel) = get_pixel(pixel, config) else {
            continue;
        };
        relevant_pixels += 1;
        let cmd = pixel_to_offset_command(x, y, pixel);
        size += cmd.1;
        let offset_vec = intermediate.get_mut(id_for_px(x, y, chunk_width)).unwrap();
        offset_vec.extend(&cmd.0[..cmd.1]);
    }
    let result = shuffle_collect(intermediate, size, config, |cmd| {
        if cmd.len() > 18 {
            Some(cmd.as_slice())
        } else {
            None
        }
    });
    (result, relevant_pixels)
}

const TO_HEX: &[u8; 16] = b"0123456789abcdef";

#[inline(always)]
fn to_hex(number: u8) -> [u8; 2] {
    unsafe {
        [
            *TO_HEX.get_unchecked(number as usize >> 4),
            *TO_HEX.get_unchecked(number as usize & 15),
        ]
    }
}

#[inline(always)]
fn to_decimal(number: u32) -> ([u8; 4], usize) {
    assert!(number < 10_000, "Too large coordinates");
    if number >= 1000 {
        (
            [
                (number / 1000) as u8 + b'0',
                (number / 100 % 10) as u8 + b'0',
                (number / 10 % 10) as u8 + b'0',
                (number % 10) as u8 + b'0',
            ],
            4,
        )
    } else if number >= 100 {
        (
            [
                (number / 100) as u8 + b'0',
                (number / 10 % 10) as u8 + b'0',
                (number % 10) as u8 + b'0',
                0,
            ],
            3,
        )
    } else if number >= 10 {
        (
            [(number / 10) as u8 + b'0', (number % 10) as u8 + b'0', 0, 0],
            2,
        )
    } else {
        ([number as u8 + b'0', 0, 0, 0], 1)
    }
}

#[inline(always)]
fn get_pixel(pixel: &Rgba<u8>, config: ImageConfig) -> Option<([u8; 8], usize)> {
    if pixel.0[3] == 0 {
        None
    } else if config.gray_usage
        && (!config.alpha_usage || pixel.0[3] == 255)
        && pixel.0[0] == pixel.0[1]
        && pixel.0[1] == pixel.0[2]
    {
        let number = to_hex(pixel.0[0]);
        Some(([number[0], number[1], 0, 0, 0, 0, 0, 0], 2))
    } else {
        let mut result = [0u8; 8];
        for i in 0..4 {
            let pixel = to_hex(pixel.0[i]);
            result[i * 2] = pixel[0];
            result[i * 2 + 1] = pixel[1];
        }
        let len = if !config.alpha_usage || pixel.0[3] == 255 {
            6
        } else {
            8
        };
        Some((result, len))
    }
}

#[inline(always)]
fn pixel_to_command(x: u32, y: u32, pixel: ([u8; 8], usize)) -> ([u8; NORMAL_SIZE], usize) {
    let mut result = [
        b'P', b'X', b' ', 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let mut size = 3;
    let coordinate = to_decimal(x);
    result[size..size + coordinate.1].copy_from_slice(&coordinate.0[..coordinate.1]);
    size += coordinate.1 + 1;
    result[size - 1] = b' ';
    let coordinate = to_decimal(y);
    result[size..size + coordinate.1].copy_from_slice(&coordinate.0[..coordinate.1]);
    size += coordinate.1 + 1;
    result[size - 1] = b' ';
    result[size..size + pixel.1].copy_from_slice(&pixel.0[..pixel.1]);
    size += pixel.1 + 1;
    result[size - 1] = b'\n';
    (result, size)
}

#[inline(always)]
fn pixel_to_offset_command(x: u32, y: u32, pixel: ([u8; 8], usize)) -> ([u8; OFFSET_SIZE], usize) {
    let mut result = [
        b'P',
        b'X',
        b' ',
        (x % CHUNK_SIZE) as u8 + b'0',
        b' ',
        (y % CHUNK_SIZE) as u8 + b'0',
        b' ',
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];
    let mut size = 7;
    result[size..size + pixel.1].copy_from_slice(&pixel.0[..pixel.1]);
    size += pixel.1 + 1;
    result[size - 1] = b'\n';
    (result, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_hex() {
        assert_eq!(to_hex(0xff), [b'f', b'f']);
        assert_eq!(to_hex(0x1f), [b'1', b'f'])
    }

    #[test]
    fn test_to_decimal() {
        assert_eq!(to_decimal(1234), ([b'1', b'2', b'3', b'4'], 4));
        assert_eq!(to_decimal(123), ([b'1', b'2', b'3', 0], 3));
        assert_eq!(to_decimal(12), ([b'1', b'2', 0, 0], 2));
        assert_eq!(to_decimal(1), ([b'1', 0, 0, 0], 1));
    }

    #[test]
    fn test_to_offset() {
        assert_eq!(
            pixel_to_offset_command(123, 456, ([b'f', b'f', b'0', b'0', b'1', b'1', 0, 0], 6)),
            (
                [
                    b'P', b'X', b' ', b'3', b' ', b'6', b' ', b'f', b'f', b'0', b'0', b'1', b'1',
                    b'\n', 0, 0
                ],
                14
            )
        )
    }

    #[test]
    fn test_network_encoder() {
        let x = 0x1234;
        let y = 0x9876;
        let pixel = Rgba([0x01, 0x23, 0x45, 0x67]);
        let expected = vec![b'P', b'B', 0x34, 0x12, 0x76, 0x98, 0x01, 0x23, 0x45, 0x67];
        assert_eq!(
            binary_encode(&BinaryFormat::CoordLERGBA, x, y, &pixel),
            expected
        );
    }
}

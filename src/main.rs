use image_handler::ImageConfig;
use manager::manage;

mod arg_handler;
mod client;
mod feature_detection;
mod image_handler;
mod manager;
mod painter;

fn main() {
    let args = arg_handler::parse();
    let mut width = args
        .value_of("width")
        .map(|w| w.parse().expect("Could not parse width"));
    let mut height = args
        .value_of("height")
        .map(|h| h.parse().expect("Could not parse height"));
    let x_offset = args
        .value_of("x")
        .map(|x| x.parse().expect("Could not parse x offset"))
        .unwrap_or(0);
    let y_offset = args
        .value_of("y")
        .map(|y| y.parse().expect("Could not parse y offset"))
        .unwrap_or(0);
    let mut offset_usage = args.get_flag("offset");
    let mut gray_usage = args.get_flag("gray");
    let host = args
        .value_of("HOST")
        .expect("Please specify a host")
        .to_string();
    if args.get_flag("feature_detection") {
        let features = feature_detection::feature_detection(&host).unwrap();
        width = width.min(Some(features.width - x_offset));
        height = height.min(Some(features.height - y_offset));
        offset_usage = offset_usage || features.offset;
        gray_usage = gray_usage || features.px_gray;
        println!("Canvas size: {} x {}", features.width, features.height);
        if features.px_gray {
            println!("PX x y gg command supported")
        }
        if features.offset {
            println!("OFFSET command supported")
        }
    }
    let image_config = ImageConfig {
        width,
        height,
        x_offset,
        y_offset,
        offset_usage,
        gray_usage,
    };
    let paths = args
        .values_of("image")
        .expect("Please specify an image paths")
        .collect();
    let command_lib = image_handler::load(paths, &image_config);
    let threads = args
        .value_of("count")
        .map(|c| c.parse().expect("Invalid count specified"))
        .unwrap_or(4);
    let fps = args
        .value_of("fps")
        .map(|c| c.parse().expect("Invalid fps specified"))
        .unwrap_or(1.0);
    manage(command_lib, threads, host, fps);
}

use image_handler::ImageConfig;
use manager::manage;

mod image_handler;
mod painter;
mod client;
mod manager;
mod arg_handler;

fn main() {
    let args = arg_handler::parse();
    let width = args.value_of("width").map(|w| w.parse().expect("Could not parse width"));
    let height = args.value_of("height").map(|h| h.parse().expect("Could not parse height"));
    let x_offset = args.value_of("x").map(|x| x.parse().expect("Could not parse x offsett")).unwrap_or(0);
    let y_offset = args.value_of("y").map(|y| y.parse().expect("Could not parse y offsett")).unwrap_or(0);
    let image_config = ImageConfig {
        width,
        height,
        x_offset,
        y_offset,
    };
    let paths = args.values_of("image").expect("Please specify an image paths").collect();
    let command_lib = image_handler::load(paths, &image_config);
    let threads = args.value_of("count").map(|c| c.parse().expect("Invalid count specified")).unwrap_or(4);
    let host = args.value_of("HOST").expect("Please specify a host").to_string();
    let fps = args.value_of("fps").map(|c| c.parse().expect("Invalid fps specified")).unwrap_or(1.0);
    manage(command_lib, threads, host, fps);
}

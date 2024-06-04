use crate::host::Host;
use manager::manage;
use pixelbomber::{feature_detection, image_handler};

mod arg_handler;
mod manager;

mod host;

fn main() {
    let args = arg_handler::parse();
    let mut image_config = image_handler::ImageConfig {
        width: args.width,
        height: args.height,
        x_offset: args.x,
        y_offset: args.y,
        offset_usage: args.offset,
        gray_usage: args.gray,
        alpha_usage: args.alpha,
        binary_usage: args.binary,
        shuffle: true,
    };
    let host = Host::new(args.host, args.bind_addr).unwrap();
    if !args.feature_detection {
        let features = feature_detection::feature_detection(host.new_stream().unwrap()).unwrap();
        let max_width = features.width - args.x;
        image_config.width = Some(image_config.width.unwrap_or(max_width).min(max_width));
        let max_height = features.height - args.y;
        image_config.height = Some(image_config.height.unwrap_or(max_height).min(max_height));
        image_config.offset_usage = image_config.offset_usage || features.offset;
        image_config.gray_usage = image_config.gray_usage || features.px_gray;
        image_config.binary_usage = image_config.binary_usage || features.binary;
        println!("Canvas size: {} x {}", features.width, features.height);
        if features.px_gray {
            println!("PX x y gg command supported")
        }
        if features.offset {
            println!("OFFSET command supported")
        }
        if features.binary {
            println!("PBxyrgba command supported (binary pixel)")
        }
    }
    if args.image.is_empty() {
        println!("Please specify at least one image path!");
        return;
    }
    let paths = args.image.iter().map(|v| v.as_str()).collect();
    let command_lib = image_handler::load(paths, image_config);
    manage(command_lib, args.count.unwrap_or(4), host, args.fps);
}

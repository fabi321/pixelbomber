use manager::manage;
use pixelbomber::{feature_detection, image_handler};

mod arg_handler;
mod manager;

fn main() {
    let args = arg_handler::parse();
    let mut width = args.width;
    let mut height = args.height;
    let mut offset_usage = args.offset;
    let mut gray_usage = args.gray;
    if args.feature_detection {
        let features = feature_detection::feature_detection(&args.host).unwrap();
        let max_width = features.width - args.x;
        width = Some(width.unwrap_or(max_width).min(max_width));
        let max_height = features.height - args.y;
        height = Some(width.unwrap_or(max_height).min(max_height));
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
    let image_config = image_handler::ImageConfig {
        width,
        height,
        x_offset: args.x,
        y_offset: args.y,
        offset_usage,
        gray_usage,
        alpha_usage: args.alpha,
        shuffle: true,
    };
    if args.image.len() == 0 {
        println!("Please specify at least one image path!");
        return;
    }
    let paths = args.image.iter().map(|v| v.as_str()).collect();
    let command_lib = image_handler::load(paths, &image_config);
    manage(command_lib, args.count.unwrap_or(4), args.host, args.fps);
}

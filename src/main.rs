use crate::host::Host;
use crate::manager::{load_from_video, manage_dynamic};
use manager::manage;
use pixelbomber::{
    feature_detection,
    image_handler::{self, BinaryFormat},
};

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
        binary: None,
        shuffle: !args.shuffle,
        chunks: args.count.unwrap_or(4) as usize,
        resize: args.resize,
    };
    let host = Host::new(args.host, args.bind_addr).unwrap();
    if args.le_rgba {
        image_config.binary = Some(BinaryFormat::CoordLERGBA)
    }
    if !args.feature_detection {
        let features = feature_detection::feature_detection(host.new_stream().unwrap()).unwrap();
        let max_width = features.width - args.x;
        image_config.width = Some(image_config.width.unwrap_or(max_width).min(max_width));
        let max_height = features.height - args.y;
        image_config.height = Some(image_config.height.unwrap_or(max_height).min(max_height));
        image_config.offset_usage = image_config.offset_usage || features.offset;
        image_config.gray_usage = image_config.gray_usage || features.px_gray;
        if image_config.binary.is_none() {
            image_config.binary = features.binary;
        }
        println!("Canvas size: {} x {}", features.width, features.height);
        if features.px_gray {
            println!("PX x y gg command supported")
        }
        if features.offset {
            println!("OFFSET command supported")
        }
        if features.binary.is_some() {
            println!("PBxyrgba command supported (binary pixel)")
        }
    }
    if args.image.is_empty() {
        println!("Please specify at least one image path!");
        return;
    }
    if args.image.len() == 1 && &args.image[0] == "-" {
        manage_dynamic(
            args.count.unwrap_or(4),
            host,
            image_config,
            args.workers,
            args.continuous,
        );
    } else if args.video {
        if !args.image.len() == 1 {
            println!("--video only works with exactly one input file");
            return;
        }
        let Some(images) = load_from_video(&args.image[0], image_config, args.workers as usize)
        else {
            return;
        };
        manage(images, args.count.unwrap_or(4), host, args.fps);
    } else {
        let paths = args.image.iter().map(|v| v.as_str()).collect();
        let command_lib = image_handler::load(paths, image_config);
        manage(command_lib, args.count.unwrap_or(4), host, args.fps);
    }
}

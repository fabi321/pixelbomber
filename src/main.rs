use crate::manager::{load_from_video, manage_dynamic};
use manager::manage;
use pixelbomber::{
    feature_detection,
    image_handler::{self, BinaryFormat},
    service::{Host, Service, ServiceBuilder},
    Client,
};

mod arg_handler;
mod camera;
mod manager;

fn main() {
    env_logger::init();
    let mut args = arg_handler::parse();
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

    if args.test_green_screen {
        camera::test_green_screen(&args.image[0]);
        return;
    }

    let mut host = Host::new(
        &args.host,
        if args.listen_manager {None} else {args.bind_addr.take()}
    ).unwrap();
    if args.le_rgba {
        image_config.binary = Some(BinaryFormat::CoordLERGBA)
    }
    if !args.feature_detection && !args.listen_manager {
        let mut client = Client::new(host.new_stream().unwrap());
        let features = feature_detection::feature_detection(&mut client).unwrap();
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
    if args.image.is_empty() && !args.listen_manager {
        println!("Please specify at least one image path!");
        return;
    }
    let mut converter_threads = 0;
    let mut threads = args.count.unwrap_or(10) as usize;
    let mut closure: Box<dyn FnMut(&mut Service)> =
        if args.image.len() == 1 && (&args.image[0] == "-" || args.image[0] == "/dev/stdin") {
            converter_threads = args.workers;
            Box::new(manage_dynamic(args.continuous))
        } else if args.image.len() == 1 && args.image[0].starts_with("/dev/video") {
            converter_threads = args.workers;
            Box::new(camera::get_callback(
                &args.image[0],
                args.green_screen.clone(),
            ))
        } else if args.video {
            if !args.image.len() == 1 {
                println!("--video only works with exactly one input file");
                return;
            }
            let Some(images) = load_from_video(&args.image[0], image_config, args.workers as usize)
            else {
                return;
            };
            Box::new(manage(images, args.fps))
        }else if args.listen_manager {
            let server = pixelbomber::service::moderator::Client::new(host, args.bind_addr).unwrap();
            host = server.target_host.clone();
            threads = server.threads;
            Box::new(server.start())
        } else {
            let paths = args.image.iter().map(|v| v.as_str()).collect();
            let command_lib = image_handler::load(paths, image_config);
            Box::new(manage(command_lib, args.fps))
        };
    let mut service = ServiceBuilder::new(host)
        .channel_limit(10)
        .converter_threads(converter_threads as usize)
        .image_config(image_config)
        .threads(threads);
    if let Some(port) = args.serve_manager {
        service = service.listen_port(port);
    }
    let mut service = service.build();
    service.loop_callback(closure.as_mut());
    service.stop();
}

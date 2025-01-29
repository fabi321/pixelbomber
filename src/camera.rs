use image::{DynamicImage, Rgb, RgbImage, Rgba, RgbaImage};
use pixelbomber::service::Service;
use rscam::{Camera, Config, ResolutionInfo};

#[inline(always)]
fn frame_to_image(frame: rscam::Frame) -> RgbImage {
    let width = frame.resolution.0;
    let height = frame.resolution.1;
    let raw_data = frame.to_vec();
    RgbImage::from_raw(width, height, raw_data).expect("Unable to create image")
}

#[inline(always)]
fn filter_green_screen(input: RgbImage, green_screen: Rgb<u8>) -> RgbaImage {
    let mut new_image = RgbaImage::new(input.width(), input.height());

    for (x, y, pixel) in input.enumerate_pixels() {
        let alpha = if pixel == &green_screen { 0 } else { 255 };
        new_image.put_pixel(x, y, Rgba([pixel[0], pixel[1], pixel[2], alpha]));
    }

    new_image
}

fn get_resolution_for_cam(cam: &Camera) -> (u32, u32) {
    let resolutions = cam
        .resolutions(b"RGB3")
        .expect("Unable to get resolutions for camera");
    match resolutions {
        ResolutionInfo::Discretes(discretes) => {
            let mut max_res = (0, 0);
            for (width, height) in discretes {
                if width * height > max_res.0 * max_res.1 {
                    max_res = (width, height);
                }
            }
            max_res
        }
        ResolutionInfo::Stepwise { max, .. } => max,
    }
}

pub fn test_green_screen(camera: &str) {
    let mut camera = Camera::new(camera).expect("Unable to initialize camera");
    let resolution = get_resolution_for_cam(&camera);
    camera
        .start(&Config {
            interval: (1, 30),
            resolution,
            format: b"RGB3",
            ..Default::default()
        })
        .expect("Unable to start camera");
    let image = frame_to_image(camera.capture().expect("Unable to capture image"));
    let pixel = image.get_pixel(0, 0);
    println!(
        "Green value (use with --green-screen): {:x}{:x}{:x}",
        pixel[0], pixel[1], pixel[2]
    )
}

pub fn get_callback(camera: &str, green_screen: Option<String>) -> impl FnMut(&mut Service) {
    let mut camera = Camera::new(camera).expect("Unable to initialize camera");
    // ababab -> [0xab, 0xab, 0xab]
    let green_screen = green_screen.map(|s| {
        let number = u32::from_str_radix(&s, 16).expect("Invalid green screen value");
        let r = ((number >> 16) & 0xff) as u8;
        let g = ((number >> 8) & 0xff) as u8;
        let b = (number & 0xff) as u8;
        [r, g, b]
    });
    let resolution = get_resolution_for_cam(&camera);
    camera
        .start(&Config {
            interval: (1, 30),
            resolution,
            format: b"RGB3",
            ..Default::default()
        })
        .expect("Unable to start camera");
    move |service: &mut Service| {
        let image = frame_to_image(camera.capture().expect("Unable to capture image"));
        if let Some(green_screen) = green_screen {
            let image = filter_green_screen(image, Rgb(green_screen));
            let image = DynamicImage::ImageRgba8(image);
            service.send_image(image);
        } else {
            let image = DynamicImage::ImageRgb8(image);
            service.send_image(image);
        }
    }
}

use std::{
    collections::HashMap,
    thread::sleep,
    time::{Duration, Instant},
};

use image::DynamicImage;
use lazy_static::lazy_static;
use pixelbomber::{
    image_handler::ImageConfigBuilder,
    service::{Service, ServiceBuilder},
    Client,
};
use rand::{seq::IndexedRandom, rng};

const THREAD_COUNT: usize = 10;
const TARGET: &str = "localhost:1234";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Direction {
    NorthEast,
    SouthEast,
    SouthWest,
    NorthWest,
}

fn get_points_for_image(image: &DynamicImage) -> Vec<(u32, u32)> {
    image
        .as_rgba8()
        .expect("Invalid image")
        .enumerate_pixels()
        .filter_map(|(x, y, pixel)| if pixel[3] > 0 { Some((x, y)) } else { None })
        .collect()
}

fn get_collision_points() -> HashMap<Direction, Vec<(u32, u32)>> {
    let image = image::load_from_memory_with_format(
        include_bytes!("football_ne.png"),
        image::ImageFormat::Png,
    )
    .expect("Unable to load image");
    let mut points_map = HashMap::new();
    points_map.insert(Direction::NorthEast, get_points_for_image(&image));
    points_map.insert(
        Direction::SouthEast,
        get_points_for_image(&image.rotate90()),
    );
    points_map.insert(
        Direction::SouthWest,
        get_points_for_image(&image.rotate180()),
    );
    points_map.insert(
        Direction::NorthWest,
        get_points_for_image(&image.rotate270()),
    );
    points_map
}

lazy_static! {
    static ref COLLISION_POINTS: HashMap<Direction, Vec<(u32, u32)>> = get_collision_points();
}

struct Ball {
    width: u32,
    height: u32,
    img: DynamicImage,
}

struct BouncyBall {
    x: u32,
    y: u32,
    direction: Direction,
    ball: Ball,
    width: u32,
    height: u32,
}

fn initialize_ball() -> Ball {
    let image = image::load_from_memory_with_format(
        include_bytes!("football.png"),
        image::ImageFormat::Png,
    )
    .expect("Unable to load image");
    Ball {
        width: image.width(),
        height: image.height(),
        img: image,
    }
}

fn is_colliding(bouncy_ball: &BouncyBall, client: &mut Client) -> bool {
    let points = COLLISION_POINTS.get(&bouncy_ball.direction).unwrap();
    let points = points
        .iter()
        .map(|(x, y)| (x + bouncy_ball.x, y + bouncy_ball.y))
        .filter(|(x, y)| x < &bouncy_ball.width && y < &bouncy_ball.height)
        .collect::<Vec<_>>();
    client
        .read_pixel_multi(&points)
        .expect("Unable to read pixels")
        .into_iter()
        .any(|pixel| pixel[0] > 245 && pixel[1] < 50 && pixel[2] < 50)
}

fn change_direction(bouncy_ball: &mut BouncyBall) {
    let mut rng = rng();
    let directions = vec![
        Direction::NorthEast,
        Direction::SouthEast,
        Direction::SouthWest,
        Direction::NorthWest,
    ];

    let new_direction = directions
        .into_iter()
        .filter(|&d| d != bouncy_ball.direction)
        .collect::<Vec<_>>()
        .choose(&mut rng)
        .unwrap()
        .to_owned();

    bouncy_ball.direction = new_direction;
}

fn move_ball(bouncy_ball: &mut BouncyBall, client: &mut Client) {
    if is_colliding(bouncy_ball, client) {
        change_direction(bouncy_ball);
    }

    match bouncy_ball.direction {
        Direction::NorthEast => {
            bouncy_ball.x = bouncy_ball.x.saturating_add(1);
            bouncy_ball.y = bouncy_ball.y.saturating_sub(1);
        }
        Direction::SouthEast => {
            bouncy_ball.x = bouncy_ball.x.saturating_add(1);
            bouncy_ball.y = bouncy_ball.y.saturating_add(1);
        }
        Direction::SouthWest => {
            bouncy_ball.x = bouncy_ball.x.saturating_sub(1);
            bouncy_ball.y = bouncy_ball.y.saturating_add(1);
        }
        Direction::NorthWest => {
            bouncy_ball.x = bouncy_ball.x.saturating_sub(1);
            bouncy_ball.y = bouncy_ball.y.saturating_sub(1);
        }
    };

    if bouncy_ball.x + bouncy_ball.ball.width >= bouncy_ball.width
        || bouncy_ball.y + bouncy_ball.ball.height >= bouncy_ball.height
        || bouncy_ball.x == 0
        || bouncy_ball.y == 0
    {
        change_direction(bouncy_ball);
    }
}

pub fn main() {
    env_logger::init();

    let mut service = ServiceBuilder::new_from_host_str(TARGET)
        .threads(THREAD_COUNT)
        .build();
    let client = service.get_client().expect("Unable to get client");
    let ball = initialize_ball();
    let (width, height) = client
        .read_screen_size()
        .expect("Unable to read screen size");
    let features = pixelbomber::feature_detection::feature_detection(client)
        .expect("Unable to detect features");
    let mut image_config = ImageConfigBuilder::new()
        .apply_features(features)
        .width(ball.width)
        .height(ball.height)
        .chunks(THREAD_COUNT)
        .build();
    service.change_image_config(image_config);
    let mut bouncy_ball = BouncyBall {
        x: width / 2,
        y: height / 2,
        direction: Direction::NorthEast,
        ball,
        width,
        height,
    };

    let mut last_time = Instant::now();

    service.loop_callback(move |service: &mut Service| {
        move_ball(
            &mut bouncy_ball,
            service.get_client().expect("Unable to get client"),
        );
        image_config.x_offset = bouncy_ball.x;
        image_config.y_offset = bouncy_ball.y;
        service.change_image_config(image_config);
        service.send_image(bouncy_ball.ball.img.clone());
        if let Some(dur) = Duration::from_millis(1000 / 120).checked_sub(last_time.elapsed()) {
            sleep(dur);
        }
        last_time = Instant::now();
    });
}

use std::sync::{
    mpsc::{Receiver, SyncSender, TrySendError},
    Arc,
};

use image::DynamicImage;

use crate::image_handler::{image_to_commands, Command, ImageConfig};

pub enum ConverterChange {
    Config(ImageConfig),
    Image(DynamicImage, usize),
}

pub fn get_converter(
    mut image_config: ImageConfig,
    source: Receiver<ConverterChange>,
    sink: SyncSender<(Arc<Command>, usize)>,
) -> impl FnMut() {
    move || loop {
        match source.recv() {
            Ok(ConverterChange::Config(config)) => {
                image_config = config;
            }
            Ok(ConverterChange::Image(image, count)) => {
                let res = image_to_commands(image, image_config);
                if let Err(TrySendError::Disconnected(_)) = sink.try_send((Arc::new(res), count)) {
                    break;
                }
            }
            Err(_) => {
                break;
            }
        }
    }
}

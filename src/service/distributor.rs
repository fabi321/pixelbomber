use std::sync::{
    mpsc::{Receiver, SyncSender, TrySendError},
    Arc,
};

use image::DynamicImage;

use crate::image_handler::{Command, ImageConfig};

use super::converter::ConverterChange;

pub enum DistributorChange {
    Image(DynamicImage),
    Config(ImageConfig),
}

pub fn get_converter_distributor(
    source: Receiver<DistributorChange>,
    sinks: Vec<SyncSender<ConverterChange>>,
) -> impl FnMut() {
    let mut count = 0;
    move || loop {
        match source.recv() {
            Ok(DistributorChange::Image(image)) => {
                let res = sinks[count % sinks.len()].try_send(ConverterChange::Image(image, count));
                if let Err(TrySendError::Disconnected(_)) = res {
                    break;
                }
                count += 1;
            }
            Ok(DistributorChange::Config(config)) => {
                for sink in &sinks {
                    if sink.send(ConverterChange::Config(config)).is_err() {
                        break;
                    }
                }
            }
            Err(_) => {
                break;
            }
        }
    }
}

pub fn get_painter_distributor(
    source: Receiver<Arc<Command>>,
    sinks: Vec<SyncSender<Arc<Command>>>,
) -> impl FnMut() {
    move || {
        while let Ok(command) = source.recv() {
            for sink in &sinks {
                if let Err(TrySendError::Disconnected(_)) = sink.try_send(command.clone()) {
                    break;
                }
            }
        }
    }
}

use std::{
    sync::{
        mpsc::{Receiver, TryRecvError},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

use log::warn;

use crate::{image_handler::Command, painter, Client};

use super::Host;

pub fn get_painter(
    source: Receiver<Arc<Command>>,
    host: Host,
    painter_id: usize,
    max_frame: usize,
) -> impl FnMut() {
    move || {
        let mut current_commands = source.recv().unwrap();
        loop {
            match host.new_stream() {
                Ok(stream) => {
                    let client = Client::new(stream);
                    current_commands =
                        painter(&source, client, painter_id, max_frame, current_commands);
                    // this might discard an animation frame, if the connection is dropped, and it is an
                    // ongoing animation, but that is mostly irrelevant, since there will be new
                    // frames later, and the connection is timed out anyway
                    if let Err(TryRecvError::Disconnected) = source.try_recv() {
                        break;
                    }
                }
                Err(err) => {
                    warn!("Could not connect to host! ({err:?})");
                }
            }
            warn!("Thread stopped working, restarting");
            sleep(Duration::from_secs(5))
        }
    }
}

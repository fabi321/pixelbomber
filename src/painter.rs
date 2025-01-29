use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;

use crate::client::Client;
use crate::image_handler::Command;

/// Paint an image to the canvas, can receive image ids to change between frames of an animation
pub fn painter(
    rx: &Receiver<Arc<Command>>,
    mut client: Client,
    painter_id: usize,
    max_frame: usize,
    mut current_commands: Arc<Command>,
) -> Arc<Command> {
    // Waits for first frame
    let mut frame = painter_id;
    // loop over frames
    'outer: loop {
        // loop over drawings of a single frame
        if client.send_pixel(&current_commands[frame]).is_err() {
            break 'outer;
        }
        frame = (frame + 1) % max_frame;
        'inner: loop {
            // ordered by likelihood
            match rx.try_recv() {
                Err(TryRecvError::Empty) => break 'inner,
                Ok(command) => {
                    current_commands = command;
                    frame = painter_id;
                }
                Err(TryRecvError::Disconnected) => {
                    // cleanly exit in case all senders are dropped
                    break 'outer;
                }
            }
        }
    }
    current_commands
}

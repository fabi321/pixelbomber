use std::sync::Arc;
use std::{io::Result, sync::mpsc::Receiver};

use crate::client::Client;
use crate::image_handler::Command;

/// Paint an image to the canvas, can receive image ids to change between frames of an animation
pub fn painter(
    rx: &Receiver<Arc<Command>>,
    mut client: Client,
    painter_id: usize,
    max_frame: usize,
) -> Result<()> {
    // Waits for first frame
    let mut current_commands = rx.recv().unwrap();
    let mut frame = painter_id;
    // loop over frames
    loop {
        // loop over drawings of a single frame
        client.send_pixel(&current_commands[frame])?;
        frame = (frame + 1) % max_frame;
        while let Ok(command) = rx.try_recv() {
            current_commands = command;
            frame = painter_id
        }
    }
}

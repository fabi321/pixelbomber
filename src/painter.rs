use std::{io::Result, sync::mpsc::Receiver};
use std::sync::Arc;

use crate::{client::Client, image_handler::CommandLib};

/// Paint an image to the canvas, can receive image ids to change between frames of an animation
pub fn painter(command_lib: Arc<CommandLib>, rx: Receiver<usize>, mut client: Client) -> Result<()> {
    let mut current_commands = &command_lib[0];
    // loop over frames
    loop {
        // loop over drawings of a single frame
        client.send_pixel(current_commands)?;
        client.flush()?;
        if let Ok(id) = rx.try_recv() {
            current_commands = &command_lib[id];
        }
    }
}

use std::{io::Result, sync::mpsc::Receiver};

use crate::{client::Client, image_handler::CommandLib};

pub fn painter(command_lib: CommandLib, rx: Receiver<usize>, mut client: Client) -> Result<()> {
    let mut current = 0;
    let mut current_commands = &command_lib[current];
    // loop over frames
    loop {
        // loop over drawings of a single frame
        client.send_pixel(current_commands)?;
        client.flush()?;
        if let Ok(id) = rx.try_recv() {
            current = id;
            current_commands = &command_lib[current];
        }
    }
}

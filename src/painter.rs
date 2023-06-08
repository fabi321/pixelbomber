use std::{sync::mpsc::Receiver, io::Result};

use crate::{image_handler::CommandLib, client::Client};

pub fn painter(command_lib: CommandLib, rx: Receiver<usize>, mut client: Client) -> Result<()> {
    let mut current = 0;
    let mut current_commands = &command_lib[current];
    // loop over frames
    loop {
        // loop over drawings of a single frame
        for command in current_commands {
            client.send_pixel(command)?;
            client.flush()?;
        }
        if let Ok(id) = rx.try_recv() {
            current = id;
            current_commands = &command_lib[current];
        }
    }
}
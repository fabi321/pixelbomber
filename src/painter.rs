use std::sync::mpsc::Receiver;

use crate::{image_handler::CommandLib, client::Client};

pub fn painter(command_lib: CommandLib, rx: Receiver<usize>, mut client: Client) {
    let mut current = 0;
    let mut current_commands = &command_lib[current];
    let mut current_range = 0..current_commands.len();
    // loop over frames
    loop {
        // loop over drawings of a single frame
        loop {
            if let Some(i) = current_range.next() {
                client.send_pixel(&current_commands[i]).unwrap();
                if i % 70 == 0 {
                    client.flush().unwrap();
                    if i % 140 == 70 {
                        if let Ok(id) = rx.try_recv() {
                            current = id;
                            current_commands = &command_lib[current];
                            break;
                        }
                    }
                }
            } else {
                break;
            }
        }
        if let Ok(id) = rx.try_recv() {
            current = id;
            current_commands = &command_lib[current];
        }
        current_range = 0..current_commands.len();
    }
}
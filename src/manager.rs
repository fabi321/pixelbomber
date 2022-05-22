use std::{thread::{self, sleep}, sync::mpsc::channel, time::Duration};

use crate::{image_handler::CommandLib, client::Client, painter::painter};

pub fn manage(commands: CommandLib, threads: u32, host: String, fps: f32) {
    let mut handles = Vec::new();
    println!("Starting threads");
    for _ in 0..threads {
        let client = Client::connect(&host).expect("Could not connect to host");
        let commands_cloned = commands.clone();
        let (tx, rx) = channel();
        thread::spawn(|| painter(commands_cloned, rx, client));
        handles.push(tx);
    }
    if commands.len() > 1 {
        loop {
            for i in 0..commands.len() {
                for tx in &handles {
                    let _ = tx.send(i);
                }
                sleep(Duration::from_secs_f32(1.0 / fps))
            }
        }
    } else {
        loop {
            sleep(Duration::from_secs(10))
        }
    }
}
use std::{
    sync::mpsc::channel,
    thread::{self, sleep},
    time::Duration,
};
use std::sync::Arc;
use std::sync::mpsc::Receiver;

use pixelbomber::{image_handler::CommandLib, painter, Client};

fn recreate_connection(host: Arc<String>, commands: Arc<CommandLib>, rx: Receiver<usize>) {
    loop {
        if let Ok(client) = Client::connect(host.as_str()) {
            let _ = painter(commands.clone(), &rx, client);
        }
        sleep(Duration::from_secs(5))
    }
}

pub fn manage(commands: CommandLib, threads: u32, host: String, fps: f32) {
    let mut handles = Vec::new();
    let mut thread_handles = Vec::new();
    let commands = Arc::new(commands);
    let host = Arc::new(host);
    println!("Starting threads");
    for _ in 0..threads {
        let commands_cloned = commands.clone();
        let host_cloned = host.clone();
        let (tx, rx) = channel();
        thread_handles.push(thread::spawn(|| {
            recreate_connection(host_cloned, commands_cloned, rx)
        }));
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
            thread_handles.retain(|v| !v.is_finished());
            if thread_handles.len() == 0 {
                break
            }
        }
    } else {
        for handle in thread_handles {
            let _ = handle.join();
        }
    }
}

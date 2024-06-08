use std::io::Read;
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::Arc;
use std::{
    io,
    sync::mpsc::channel,
    thread::{self, sleep},
    time::Duration,
};

use crate::host::Host;
use pixelbomber::image_handler::{load_from_memory, Command, CommandLib, ImageConfig};
use pixelbomber::{painter, Client};

fn recreate_connection(
    host: Host,
    rx: Receiver<Arc<Command>>,
    painter_id: usize,
    max_frame: usize,
) {
    loop {
        if let Ok(stream) = host.new_stream() {
            let client = Client::new(stream);
            let _ = painter(&rx, client, painter_id, max_frame);
        } else {
            println!("Could not connect!")
        }
        println!("Thread stopped working, restarting");
        sleep(Duration::from_secs(5))
    }
}

pub fn manage(commands: CommandLib, threads: u32, host: Host, fps: f32) {
    let mut handles = Vec::new();
    let mut thread_handles = Vec::new();
    println!("Starting threads");
    for i in 0..threads {
        let (tx, rx) = channel();
        thread_handles.push(thread::spawn(move || {
            recreate_connection(host, rx, i as usize, threads as usize)
        }));
        handles.push(tx);
    }
    if commands.len() > 1 {
        loop {
            for command in &commands {
                for tx in &handles {
                    let _ = tx.send(command.clone());
                }
                sleep(Duration::from_secs_f32(1.0 / fps))
            }
            thread_handles.retain(|v| !v.is_finished());
            if thread_handles.is_empty() {
                break;
            }
        }
    } else {
        for tx in &handles {
            let _ = tx.send(commands[0].clone());
        }
        for handle in thread_handles {
            let _ = handle.join();
        }
    }
}

pub fn manage_dynamic(threads: u32, host: Host, config: ImageConfig, workers: u32) {
    let mut painter_senders = Vec::new();
    for i in 0..threads {
        let (tx, rx) = channel();
        thread::spawn(move || recreate_connection(host, rx, i as usize, threads as usize));
        painter_senders.push(tx);
    }
    let mut worker_handles = Vec::new();
    for _ in 0..workers {
        let (tx, rx) = sync_channel::<Vec<u8>>(1);
        let painter_clone = painter_senders.clone();
        thread::spawn(move || {
            loop {
                // Fails if no sender
                let Ok(image) = rx.recv() else { break };
                let Ok(result) = load_from_memory(&image, config) else {
                    println!("Error loading image");
                    continue;
                };
                for painter in &painter_clone {
                    let _ = painter.send(result.clone());
                }
            }
        });
        worker_handles.push(tx)
    }
    let mut stdin = io::stdin();
    let mut buf = [0u8; 16384];
    let mut image = Vec::new();
    let mut current_worker = 0;
    while let Ok(data) = stdin.read(&mut buf) {
        if data == 0 {
            break;
        }
        image.extend_from_slice(&buf[..data]);

        // BMP header size is 54 bytes
        if image.len() < 54 {
            continue;
        }
        if let Some(pos) = image.windows(2).position(|w| w == b"BM") {
            image.drain(..pos);
        } else {
            image.drain(..);
            continue;
        }
        let size = u32::from_le_bytes((&image[2..6]).try_into().unwrap()) as usize;
        if image.len() >= size {
            let bmp_data: Vec<u8> = image.drain(..size).collect();

            if worker_handles[current_worker].try_send(bmp_data).is_err() {
                println!("Dropping frame");
            };
            current_worker = (current_worker + 1) % workers as usize;
        }
    }
}

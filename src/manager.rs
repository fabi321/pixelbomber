use image::ImageFormat;
use std::collections::HashMap;
use std::io::Read;
use std::process::Stdio;
use std::sync::mpsc::{sync_channel, Receiver, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex};
use std::{
    io,
    thread::{self, sleep},
    time::Duration,
};
use sysinfo::System;

use crate::host::Host;
use pixelbomber::image_handler::{load_from_memory, Command, CommandLib, ImageConfig};
use pixelbomber::{painter, Client};

const CHANNEL_LIMIT: usize = 10;

fn recreate_connection(
    host: Host,
    rx: Receiver<Arc<Command>>,
    painter_id: usize,
    max_frame: usize,
) {
    let Ok(mut current_command) = rx.recv() else {
        return;
    };
    loop {
        if let Ok(stream) = host.new_stream() {
            let client = Client::new(stream);
            current_command = painter(&rx, client, painter_id, max_frame, current_command);
            // this might discard an animation frame, if the connection is dropped, and it is an
            // ongoing animation, but that is mostly irrelevant, since there will be new
            // frames later, and the connection is timed out anyway
            if let Err(TryRecvError::Disconnected) = rx.try_recv() {
                break;
            }
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
        let (tx, rx) = sync_channel(CHANNEL_LIMIT);
        let _ = tx.send(commands[0].clone());
        thread_handles.push(thread::spawn(move || {
            recreate_connection(host, rx, i as usize, threads as usize)
        }));
        handles.push(tx);
    }
    if commands.len() > 1 {
        loop {
            for command in &commands {
                for tx in &handles {
                    let _ = tx.try_send(command.clone());
                }
                sleep(Duration::from_secs_f32(1.0 / fps))
            }
            thread_handles.retain(|v| !v.is_finished());
            if thread_handles.is_empty() {
                break;
            }
        }
    } else {
        for handle in thread_handles {
            let _ = handle.join();
        }
    }
}

pub fn manage_dynamic(threads: u32, host: Host, config: ImageConfig, workers: u32) {
    let mut painter_senders = Vec::new();
    let mut handles = Vec::new();
    for i in 0..threads {
        let (tx, rx) = sync_channel(CHANNEL_LIMIT);
        handles.push(thread::spawn(move || {
            recreate_connection(host, rx, i as usize, threads as usize)
        }));
        painter_senders.push(tx);
    }
    let (manager_tx, manager_rx) = sync_channel::<(Arc<Command>, usize)>(CHANNEL_LIMIT);
    handles.push(thread::spawn(move || {
        let mut last_frame = 0;
        'outer: while let Ok((result, result_frame)) = manager_rx.recv() {
            // Throw away frames older than the last one
            // This ensures that the image won't "jerk backward",
            // but instead just stand still for a bit
            if result_frame >= last_frame {
                last_frame = result_frame;
                for painter in &painter_senders {
                    if let Err(TrySendError::Disconnected(_)) = painter.try_send(result.clone()) {
                        break 'outer;
                    }
                }
            }
        }
    }));
    let mut worker_handles = Vec::new();
    for _ in 0..workers {
        let (tx, rx) = sync_channel::<(Vec<u8>, usize)>(1);
        let manager_clone = manager_tx.clone();
        handles.push(thread::spawn(move || {
            while let Ok((image, frame)) = rx.recv() {
                let Ok(result) = load_from_memory(&image, config, ImageFormat::Bmp) else {
                    println!("Error loading image");
                    continue;
                };
                if manager_clone.send((Arc::new(result), frame)).is_err() {
                    break;
                }
            }
        }));
        worker_handles.push(tx)
    }
    let reader = BitmapReader::new(io::stdin());
    let mut frame = 0;
    for bmp_data in reader {
        if worker_handles[frame % workers as usize]
            .try_send((bmp_data, frame))
            .is_err()
        {
            println!("Dropping frame");
        };
        frame += 1;
    }
    // By dropping the worker handles, this tells the rest of the pipe that the task is done
    drop(worker_handles);
    drop(manager_tx);
    println!("Processed {frame} frames");
    for handle in handles {
        let _ = handle.join();
    }
}

pub fn load_from_video(path: &str, config: ImageConfig, workers: usize) -> Option<CommandLib> {
    let mut cmd = std::process::Command::new("ffmpeg")
        .arg("-v")
        .arg("error")
        .arg("-i")
        .arg(path)
        .arg("-f")
        .arg("image2pipe")
        .arg("-c")
        .arg("bmp")
        .arg("-")
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Unable to execute ffmpeg");
    let reader = BitmapReader::new(cmd.stdout.take().unwrap());
    let mut worker_txs = Vec::with_capacity(workers);
    let mut handles = Vec::with_capacity(workers);
    let result_map = Arc::new(Mutex::new(HashMap::new()));
    for _ in 0..workers {
        let result_clone = result_map.clone();
        let (worker_tx, worker_rx) = sync_channel::<(Vec<u8>, usize)>(1);
        worker_txs.push(worker_tx);
        handles.push(thread::spawn(move || {
            while let Ok((image, frame)) = worker_rx.recv() {
                let Ok(image) = load_from_memory(&image, config, ImageFormat::Bmp) else {
                    continue;
                };
                {
                    let mut results = result_clone.lock().expect("Unable to lock results");
                    results.insert(frame, image);
                }
            }
        }))
    }
    let mut frame = 0;
    let mut system = System::new();
    for bmp_data in reader {
        system.refresh_memory();
        if system.available_memory() < 1_000_000_000 {
            println!("WARNING: Less than 1GB memory, stopping at frame {frame}");
            let _ = cmd.kill();
            break;
        }
        worker_txs[frame % workers]
            .send((bmp_data, frame))
            .expect("Worker thread stopped working");
        frame += 1;
    }
    drop(worker_txs);
    for worker in handles {
        let _ = worker.join();
    }
    if frame == 0 {
        let mut error = String::new();
        cmd.stderr
            .unwrap()
            .read_to_string(&mut error)
            .expect("Unable to read error message");
        print!("ffmpeg error:\n{}", error);
        return None;
    }
    let mut result = Vec::with_capacity(frame);
    let mut result_map = result_map.lock().expect("Unable to lock result_map");
    let largest_key = result_map.keys().max().expect("No frames in results");
    for frame in 0..*largest_key {
        if let Some(entry) = result_map.remove(&frame) {
            result.push(Arc::new(entry))
        }
    }
    Some(result)
}

pub struct BitmapReader<R: Read> {
    pipe: R,
    image: Vec<u8>,
    /// position of last written byte in buffer
    content_end: usize,
}

impl<R: Read> BitmapReader<R> {
    pub fn new(pipe: R) -> BitmapReader<R> {
        BitmapReader {
            pipe,
            image: vec![0; 4096],
            content_end: 0,
        }
    }
}

impl<R: Read> Iterator for BitmapReader<R> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Ok(data) = self.pipe.read(&mut self.image[self.content_end..]) {
            if data == 0 {
                return None;
            }

            self.content_end += data;

            // This discards any random junk that might be passed before an image
            if let Some(pos) = self.image.windows(2).position(|w| w == b"BM") {
                self.content_end -= pos;
                self.image.drain(..pos);
            } else {
                self.content_end = 0;
                continue;
            }

            // BMP header size is 54 bytes
            if self.content_end < 54 {
                self.image.resize(4096, 0);
                continue;
            }

            // Read image size from bmp header
            let size = u32::from_le_bytes((&self.image[2..6]).try_into().unwrap()) as usize;
            if self.content_end == size {
                // assume that the next image is roughly as big as this one
                let bmp_data = std::mem::replace(&mut self.image, Vec::with_capacity(size));
                self.content_end = 0;
                self.image.resize(4096, 0);

                return Some(bmp_data);
            } else {
                self.image.resize(size, 0);
            }
        }
        None
    }
}

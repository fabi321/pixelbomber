use image::ImageFormat;
use pixelbomber::service::Service;
use std::collections::HashMap;
use std::io::Read;
use std::process::Stdio;
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::{
    io,
    thread::{self, sleep},
    time::Duration,
};
use sysinfo::System;

use pixelbomber::image_handler::{load_from_memory, CommandLib, ImageConfig};

pub fn manage(commands: CommandLib, mut fps: f32) -> impl FnMut(&mut Service) {
    let mut frame = 0;
    if commands.len() == 1 {
        fps = 0.000001;
    }
    move |service: &mut Service| {
        service.send_command(commands[frame].clone());
        frame = (frame + 1) % commands.len();

        sleep(Duration::from_secs_f32(1.0 / fps))
    }
}

pub fn manage_dynamic(continuous: bool) -> impl FnMut(&mut Service) {
    let mut reader = ContinuousReader::new(continuous);
    move |service: &mut Service| {
        let Some(img) = reader.next() else {
            return;
        };
        service.send_image(image::load_from_memory_with_format(&img, ImageFormat::Bmp).unwrap());
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
    _ = cmd.wait();
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

pub struct ContinuousReader {
    reader: BitmapReader<io::Stdin>,
    continuous: bool,
}

impl ContinuousReader {
    pub fn new(continuous: bool) -> ContinuousReader {
        ContinuousReader {
            reader: BitmapReader::new(io::stdin()),
            continuous,
        }
    }
}

impl Iterator for ContinuousReader {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(res) = self.reader.next() {
            return Some(res);
        }
        if !self.continuous {
            return None;
        }
        loop {
            self.reader = BitmapReader::new(io::stdin());
            if let Some(res) = self.reader.next() {
                return Some(res);
            }
        }
    }
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

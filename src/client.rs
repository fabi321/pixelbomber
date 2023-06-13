use std::{net::TcpStream, io::{Result, Write, BufRead}};

use bufstream::BufStream;

use crate::image_handler::Command;

const CMD_READ_BUFFER_SIZE: usize = 32;

pub struct Client {
    stream: BufStream<TcpStream>
}

impl Client {
    pub fn new(stream: TcpStream) -> Client {
        Client {
            stream: BufStream::new(stream)
        }
    }

    pub fn connect(host: &str) -> Result<Client> {
        Ok(Client::new(TcpStream::connect(host)?))
    }

    #[inline(always)]
    pub fn send_pixel(&mut self, command: &Command) -> Result<()> {
        self.stream.write_all(command)
    }

    #[inline(always)]
    pub fn flush(&mut self) -> Result<()> {
        self.stream.flush()
    }

    #[allow(dead_code)]
    pub fn read_screen_size(&mut self) -> Result<(u32, u32)> {
        self.stream.write_all("SIZE".to_string().as_bytes())?;
        self.stream.flush()?;
        let mut buffer = String::with_capacity(CMD_READ_BUFFER_SIZE);
        self.stream.read_line(&mut buffer)?;
        let parts: Vec<&str> = buffer.split(" ").collect();
        let width = parts[1].parse::<u32>().expect("Could not parse screen width");
        let height = parts[2].parse::<u32>().expect("Could not parse screen height");
        Ok((width, height))
    }
}
use std::{
    io::{BufRead, Result, Write},
    net::TcpStream,
};

use bufstream::BufStream;

use crate::image_handler::Command;

const CMD_READ_BUFFER_SIZE: usize = 1024;

/// A pixelflut client, supporting most pixelflut commands
/// This is a sync implementation
pub struct Client {
    stream: BufStream<TcpStream>,
}

impl Client {
    pub fn new(stream: TcpStream) -> Client {
        Client {
            stream: BufStream::new(stream),
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

    pub fn read_screen_size(&mut self) -> Result<(u32, u32)> {
        self.stream.write_all("SIZE\n".as_bytes())?;
        self.stream.flush()?;
        let mut buffer = String::with_capacity(CMD_READ_BUFFER_SIZE);
        self.stream.read_line(&mut buffer)?;
        let parts: Vec<&str> = buffer.trim_end().split(' ').collect();
        let width = parts[1]
            .parse::<u32>()
            .expect("Could not parse screen width");
        let height = parts[2]
            .parse::<u32>()
            .expect("Could not parse screen height");
        Ok((width, height))
    }

    pub fn read_help(&mut self) -> Result<String> {
        self.stream.write_all("HELP\nPX 0 0\n".as_bytes())?;
        self.stream.flush()?;
        let mut result = String::new();
        let mut buffer = String::with_capacity(CMD_READ_BUFFER_SIZE);
        loop {
            self.stream.read_line(&mut buffer)?;
            if buffer.starts_with("PX 0 0") {
                break;
            } else {
                result.push_str(&buffer);
                buffer.clear();
            }
        }
        Ok(result)
    }
}

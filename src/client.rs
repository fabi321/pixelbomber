use std::{
    io::{BufRead, Error, ErrorKind, Result, Write},
    net::TcpStream,
    str::Split,
};

use bufstream::BufStream;
use image::Rgb;

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
    pub fn send_pixel(&mut self, command: &[u8]) -> Result<()> {
        self.stream.write_all(command)
    }

    #[inline(always)]
    pub fn flush(&mut self) -> Result<()> {
        self.stream.flush()
    }

    pub(crate) fn is_error(&self) -> bool {
        self.stream
            .get_ref()
            .take_error()
            .is_ok_and(|v| v.is_some())
    }

    pub fn shutdown(&self) -> Result<()> {
        self.stream.get_ref().shutdown(std::net::Shutdown::Both)?;
        Ok(())
    }

    #[inline(always)]
    fn get_next_u32(split: &mut Split<char>, radix: u32) -> Result<u32> {
        split
            .next()
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "No data"))
            .and_then(|data| {
                u32::from_str_radix(data, radix)
                    .map_err(|err| Error::new(ErrorKind::InvalidData, err))
            })
    }

    pub fn read_screen_size(&mut self) -> Result<(u32, u32)> {
        self.stream.write_all("SIZE\n".as_bytes())?;
        self.stream.flush()?;
        let mut buffer = String::with_capacity(CMD_READ_BUFFER_SIZE);
        self.stream.read_line(&mut buffer)?;
        let mut parts = buffer.trim_end().split(' ');
        _ = parts.next(); // SIZE
        let width = Self::get_next_u32(&mut parts, 10)?;
        let height = Self::get_next_u32(&mut parts, 10)?;
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

    #[inline(always)]
    fn parse_px_response(response: &str) -> Result<Rgb<u8>> {
        let mut parts = response.trim_end().split(' ');
        _ = parts.next(); // PX
        _ = parts.next(); // x
        _ = parts.next(); // y
        let color: u32 = Self::get_next_u32(&mut parts, 16)?;
        Ok(Rgb([(color >> 16) as u8, (color >> 8) as u8, color as u8]))
    }

    pub fn read_pixel_multi(&mut self, pixel: &[(u32, u32)]) -> Result<Vec<Rgb<u8>>> {
        for (x, y) in pixel.iter() {
            self.stream.write_all(format!("PX {x} {y}\n").as_bytes())?;
        }
        self.stream.flush()?;
        let mut buffer = String::with_capacity(CMD_READ_BUFFER_SIZE);
        let mut result = Vec::with_capacity(pixel.len());
        for _ in 0..pixel.len() {
            self.stream.read_line(&mut buffer)?;
            result.push(Self::parse_px_response(&buffer)?);
            buffer.clear();
        }
        Ok(result)
    }

    pub fn read_pixel(&mut self, x: u32, y: u32) -> Result<Rgb<u8>> {
        Ok(self.read_pixel_multi(&[(x, y)])?[0])
    }
}

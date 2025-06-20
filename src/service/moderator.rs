use std::error::Error;
use std::io;
use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::thread::sleep;
use std::time::Duration;
use bincode::{encode_to_vec, Decode, Encode};
use bincode::config::standard;
use log::warn;
use crate::image_handler::Command;
use crate::service::{Host, Service};

pub struct Server {
    listen_port: u16,
    host: Host,
    threads: usize,
    clients: Vec<TcpStream>,
    data: Receiver<Arc<Command>>
}

#[derive(Decode, Encode, Debug)]
struct Target {
    addr: Vec<IpAddr>,
    port: u16,
    threads: usize,
}

fn read<R: Decode<()>>(stream: &mut TcpStream) -> Result<R, Box<dyn Error>> {
    let mut length = [0u8; 4];
    stream.read_exact(&mut length)?;
    let length = u32::from_be_bytes(length);
    let mut data = vec![0u8; length as usize];
    stream.read_exact(&mut data)?;
    let decompressed = zstd::decode_all(&mut &data[..])?;
    let (result, _) = bincode::decode_from_slice(&decompressed[..], standard())?;
    Ok(result)
}

// this write length encodes and ensures that everything or nothing is written
fn write<S: Encode>(stream: &mut TcpStream, data: S) -> Result<(), Box<dyn Error>> {
    let encoded = encode_to_vec(data, standard())?;
    let compressed = zstd::encode_all(&encoded[..], 3)?;
    let length = (compressed.len() as u32).to_be_bytes();
    match stream.write(&length[..]) {
        Ok(n) if n == 4 => {
            let mut written = 0;
            while written != compressed.len() {
                match stream.write(&compressed[written..]) {
                    Ok(n) => written += n,
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {},
                    Err(e) => Err(e)?,
                }
            }
            Ok(())
        },
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Ok(()),
        Err(e) => Err(e)?,
        Ok(_) => panic!("Impossible to send less than 4 bytes"),
    }
}

impl Server {
    pub fn new(listen_port: u16, host: Host, threads: usize, data: Receiver<Arc<Command>>) -> Self {
        Server {
            listen_port,
            host,
            threads,
            clients: Vec::new(),
            data,
        }
    }

    pub fn listen(mut self) {
        let listener = TcpListener::bind(("0.0.0.0", self.listen_port)).expect("Server Error");
        listener.set_nonblocking(true).expect("Server Error");
        loop {
            if let Ok((mut stream, _)) = listener.accept() {
                let target = Target {
                    addr: self.host.addr.clone(),
                    port: self.host.port,
                    threads: self.threads,
                };
                let _ = write(&mut stream, target);
                stream.set_nonblocking(true).expect("Server Error");
                self.clients.push(stream);
            }
            match self.data.try_recv() {
                Ok(update) => {
                    let mut to_remove = Vec::new();
                    for (i, client) in self.clients.iter_mut().enumerate() {
                        if let Err(_) = write(client, update.as_ref()) {
                            to_remove.push(i);
                        }
                    }
                    to_remove.reverse();
                    for to_remove in to_remove {
                        self.clients.remove(to_remove);
                    }
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => { break }
            }
        }
    }
}

pub struct Client {
    mod_host: Host,
    pub target_host: Host,
    pub threads: usize,
}

impl Client {
    pub fn new(mod_host: Host, bind_addr: Option<String>) -> Result<Self, Box<dyn Error>> {
        let def: Target = read(&mut mod_host.new_stream()?)?;
        println!("Connected to server, def:{:?}", def);
        Ok(Client {
            mod_host,
            target_host: Host::from_raw(def.addr, def.port, bind_addr)?,
            threads: def.threads,
        })
    }

    pub fn start(self) -> impl FnMut(&mut Service) {
        let mut stream = self.mod_host.new_stream().expect("Server Error");
        let _: Target = read(&mut stream).expect("Server Error");
        move |service: &mut Service | {
            if let Ok(data) = read(&mut stream) {
                let arced: Arc<Command> = Arc::new(data);
                let _ = service.painter_input.as_ref().unwrap().try_send(arced);
            } else {
                warn!("Connection to manager lost, reconnecting");
                sleep(Duration::from_secs(1));
                // using return here ensures that the process can be exited after at most 1s
                let Ok(new_stream) = self.mod_host.new_stream() else { return };
                stream = new_stream;
                let Ok(_) = read::<Target>(&mut stream) else { return };
                let Ok(command) = read::<Command>(&mut stream) else { return };
                let _ = service.painter_input.as_ref().unwrap().try_send(Arc::new(command));
                warn!("Reconnected to manager");
            }
        }
    }
}

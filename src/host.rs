use net2::TcpBuilder;
use std::io;
use std::net::{IpAddr, TcpStream};
use std::str::FromStr;

#[derive(Copy, Clone, Debug)]
pub struct Host {
    pub addr: IpAddr,
    pub bind: Option<IpAddr>,
    pub port: u16,
}

impl Host {
    pub fn new(host_str: String, bind_addr: Option<String>) -> Result<Host, String> {
        let mut host_parts = host_str.split(':');
        let addr: IpAddr = host_parts
            .next()
            .ok_or_else(|| "No address specified".to_string())
            .and_then(|v| IpAddr::from_str(v).map_err(|e| e.to_string()))?;
        let port: u16 = host_parts
            .next()
            .ok_or_else(|| "No port specified".to_string())
            .and_then(|v| u16::from_str(v).map_err(|e| e.to_string()))?;
        let bind: Option<IpAddr> = if let Some(v) = bind_addr {
            let bind_addr = IpAddr::from_str(&v).map_err(|e| e.to_string())?;
            if addr.is_ipv4() != bind_addr.is_ipv4() {
                return Err(
                    "Host and bind address must be in the same address class (both v4 or both v6)"
                        .to_string(),
                );
            }
            Some(bind_addr)
        } else {
            None
        };
        Ok(Host { addr, port, bind })
    }

    pub fn new_stream(&self) -> io::Result<TcpStream> {
        let builder = if self.addr.is_ipv4() {
            TcpBuilder::new_v4()
        } else {
            TcpBuilder::new_v6()
        }?;
        if let Some(bind) = self.bind {
            builder.bind((bind, 0u16))?;
        };
        builder.connect((self.addr, self.port))
    }
}

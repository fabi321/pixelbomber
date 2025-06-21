use std::ffi::OsString;
use rand::seq::IndexedRandom;
use rand::{rng};
use std::io;
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::str::FromStr;
use nix::sys::socket::{setsockopt, sockopt::BindToDevice};
use socket2::{SockAddr, Domain, Socket, Type};
use trust_dns_resolver::Resolver;
use url::Url;

#[derive(Clone, Debug)]
pub struct Host {
    pub addr: Vec<IpAddr>,
    pub bind: Option<String>,
    pub port: u16,
}

impl Host {
    pub fn new(host_str: &str, bind_addr: Option<String>) -> Result<Host, String> {
        // this ensures that it is a valid url
        let host_str = format!("http://{}", host_str);
        let url = Url::parse(&host_str).map_err(|err| err.to_string())?;
        let host = url
            .host_str()
            .ok_or_else(|| "No host specified".to_string())?;
        let host = host.trim_start_matches('[').trim_end_matches(']');
        let addr: Vec<_> = if let Ok(res) = IpAddr::from_str(host) {
            vec![res]
        } else {
            let resolver = Resolver::default().map_err(|err| err.to_string())?;
            let addr: Vec<_> = resolver
                .lookup_ip(host)
                .map_err(|err| err.to_string())?
                .iter()
                .collect();
            // if ipv6 is available, only use ipv6
            if addr.iter().any(|a| a.is_ipv6()) {
                addr.into_iter().filter(|a| a.is_ipv6()).collect()
            } else {
                addr
            }
        };
        if addr.is_empty() {
            return Err("No address found".to_string());
        }
        let port = url.port().ok_or_else(|| "No port specified".to_string())?;
        Self::from_raw(addr, port, bind_addr)
    }

    pub fn from_raw(addr: Vec<IpAddr>, port: u16, bind: Option<String>) -> Result<Host, String> {
        Ok(Host { addr, port, bind })

    }

    pub fn new_stream(&self) -> io::Result<TcpStream> {
        let addr = *self.addr.choose(&mut rng()).unwrap();
        let socket_addr = SocketAddr::new(addr, self.port);
        let socket = Socket::new(Domain::for_address(socket_addr), Type::STREAM, None)?;
        if let Some(bind) = &self.bind {
            let name = OsString::from(bind);
            setsockopt(&socket, BindToDevice, &name)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        };
        socket.connect(&SockAddr::from(socket_addr))?;
        Ok(socket.into())
    }
}

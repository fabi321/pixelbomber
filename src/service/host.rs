use net2::TcpBuilder;
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::io;
use std::net::{IpAddr, TcpStream};
use std::str::FromStr;
use trust_dns_resolver::Resolver;
use url::Url;

#[derive(Clone, Debug)]
pub struct Host {
    pub addr: Vec<IpAddr>,
    pub bind: Option<IpAddr>,
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

    pub fn from_raw(addr: Vec<IpAddr>, port: u16, bind_addr: Option<String>) -> Result<Host, String> {
        let bind: Option<IpAddr> = if let Some(v) = bind_addr {
            let bind_addr = IpAddr::from_str(&v).map_err(|e| e.to_string())?;
            if addr[0].is_ipv4() != bind_addr.is_ipv4() {
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
        let builder = if self.addr[0].is_ipv4() {
            TcpBuilder::new_v4()
        } else {
            TcpBuilder::new_v6()
        }?;
        if let Some(bind) = self.bind {
            builder.bind((bind, 0u16))?;
        };
        let addr = *self.addr.choose(&mut thread_rng()).unwrap();
        builder.connect((addr, self.port))
    }
}

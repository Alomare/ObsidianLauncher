//! Finding wars on the local network. A Commander server with a war open to LAN broadcasts a small
//! JSON packet to UDP port 17493 every 3 seconds (`docs/spec-m1.md`, section 2); this listens for a
//! while and lists what it heard. The address is the packet's source, never a field in it.

use std::collections::BTreeMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

pub const DISCOVERY_PORT: u16 = 17493;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub war: String,
    pub name: String,
    pub model: String,
    pub season: u32,
    /// `http://<source address>:<port>`.
    pub base: String,
    pub computer: String,
    pub players: u32,
    pub version: String,
}

#[derive(Debug, Deserialize)]
struct Packet {
    commander: u32,
    war: String,
    name: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    season: u32,
    port: u16,
    #[serde(default)]
    host: String,
    #[serde(default)]
    players: u32,
    #[serde(default)]
    version: String,
}

fn clip(s: &str, n: usize) -> String {
    s.chars().filter(|c| !c.is_control()).take(n).collect()
}

/// A packet from `from`, checked: protocol 1, a plain id and a real port.
pub fn read_packet(bytes: &[u8], from: SocketAddr) -> Option<Found> {
    let p: Packet = serde_json::from_slice(bytes).ok()?;
    let id_ok = !p.war.is_empty() && p.war.len() <= 32 && p.war.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if p.commander != 1 || !id_ok || p.port == 0 {
        return None;
    }
    Some(Found {
        war: p.war,
        name: clip(&p.name, 60),
        model: clip(&p.model, 16),
        season: p.season,
        base: format!("http://{}:{}", from.ip(), p.port),
        computer: clip(&p.host, 40),
        players: p.players,
        version: clip(&p.version, 20),
    })
}

/// Listens for `wait` and answers every war heard, one entry per war and address.
pub fn scan(wait: Duration) -> std::io::Result<Vec<Found>> {
    let socket = UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT))?;
    socket.set_read_timeout(Some(Duration::from_millis(250)))?;
    let end = Instant::now() + wait;
    let mut found: BTreeMap<(String, String), Found> = BTreeMap::new();
    let mut buf = [0u8; 2048];
    while Instant::now() < end {
        match socket.recv_from(&mut buf) {
            Ok((n, from)) => {
                if let Some(f) = read_packet(&buf[..n], from) {
                    found.insert((f.war.clone(), f.base.clone()), f);
                }
            }
            Err(e) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(found.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from() -> SocketAddr {
        "192.168.0.7:51000".parse().unwrap()
    }

    #[test]
    fn reads_a_packet_with_the_address_from_its_source() {
        let p = br#"{"commander":1,"war":"k3x9q2","name":"Night War","model":"vanilla","season":1042,"port":17494,"host":"DESK","players":3,"version":"0.1.0"}"#;
        let f = read_packet(p, from()).unwrap();
        assert_eq!(f.base, "http://192.168.0.7:17494");
        assert_eq!((f.war.as_str(), f.name.as_str(), f.players), ("k3x9q2", "Night War", 3));
    }

    #[test]
    fn refuses_odd_packets() {
        for p in [
            &br#"{"commander":2,"war":"a","name":"x","port":1}"#[..],
            br#"{"commander":1,"war":"../x","name":"x","port":1}"#,
            br#"{"commander":1,"war":"a","name":"x","port":0}"#,
            b"not json",
        ] {
            assert!(read_packet(p, from()).is_none(), "{}", String::from_utf8_lossy(p));
        }
        let long = format!(r#"{{"commander":1,"war":"a","name":"{}\n","port":5}}"#, "x".repeat(200));
        assert_eq!(read_packet(long.as_bytes(), from()).unwrap().name.len(), 60, "names are clipped and cleaned");
    }
}

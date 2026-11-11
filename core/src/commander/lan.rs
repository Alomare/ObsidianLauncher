//! Finding wars on the local network. A Commander server with a war open to LAN broadcasts a small
//! JSON packet to UDP port 17493 every 3 seconds (`docs/spec-m1.md`, section 2); this listens for a
//! while and lists what it heard. The address is the packet source, never a field in it.

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

/// The port a war server listens on for other PCs unless the host says otherwise.
pub const DEFAULT_WAR_PORT: u16 = 17494;

/// `http://<host>:<port>` from what a player typed: a name or an address, with or without a port
/// and with or without `http://`. None for anything else, so nothing typed reaches a request as
/// a path, a user name or another scheme.
pub fn base_of(typed: &str) -> Option<String> {
    let t = typed.trim().trim_end_matches('/');
    let t = t.strip_prefix("http://").unwrap_or(t);
    let (host, port) = match t.rsplit_once(':') {
        Some((h, p)) if !h.contains(':') => (h, p.parse::<u16>().ok().filter(|p| *p != 0)?),
        Some(_) => return None,
        None => (t, DEFAULT_WAR_PORT),
    };
    let ok = !host.is_empty() && host.len() <= 100 && !host.starts_with(['-', '.']) && host.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-');
    ok.then(|| format!("http://{host}:{port}"))
}

#[derive(Debug, Deserialize)]
struct Info {
    #[serde(default)]
    version: String,
    #[serde(default)]
    wars: Vec<InfoWar>,
}

#[derive(Debug, Deserialize)]
struct InfoWar {
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    season: u32,
    #[serde(default)]
    players: u32,
}

/// The wars in a war server answer to `GET /api/info`, as found at `base`.
pub fn read_info(body: &[u8], base: &str) -> Vec<Found> {
    let Ok(info) = serde_json::from_slice::<Info>(body) else { return Vec::new() };
    let computer = base.strip_prefix("http://").unwrap_or(base).to_string();
    info.wars
        .into_iter()
        .filter(|w| !w.id.is_empty() && w.id.len() <= 32 && w.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
        .take(64)
        .map(|w| Found { war: w.id, name: clip(&w.name, 60), model: clip(&w.model, 16), season: w.season, base: base.to_string(), computer: clip(&computer, 40), players: w.players, version: clip(&info.version, 20) })
        .collect()
}

/// Asks the war server at an address a player typed which wars it has open: the way into a war
/// over the internet, where the announcements of `scan` do not reach.
pub fn ask(typed: &str) -> std::io::Result<Vec<Found>> {
    let bad = |why: String| std::io::Error::new(std::io::ErrorKind::InvalidInput, why);
    let base = base_of(typed).ok_or_else(|| bad(format!("not an address: {}", clip(typed, 60))))?;
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .user_agent("Obsidian")
        .max_redirects(0)
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_recv_response(Some(Duration::from_secs(8)))
        .timeout_recv_body(Some(Duration::from_secs(8)))
        .build()
        .into();
    let mut res = agent.get(format!("{base}/api/info")).call().map_err(|e| std::io::Error::other(format!("no war server answered at {base}: {e}")))?;
    if res.status().as_u16() != 200 {
        return Err(std::io::Error::other(format!("no war open at {base} (the server answered {})", res.status().as_u16())));
    }
    let body = res.body_mut().with_config().limit(256 * 1024).read_to_vec().map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(read_info(&body, &base))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_typed_address_becomes_a_plain_base_or_nothing() {
        assert_eq!(base_of("203.0.113.7").as_deref(), Some("http://203.0.113.7:17494"));
        assert_eq!(base_of(" http://203.0.113.7:40000/ ").as_deref(), Some("http://203.0.113.7:40000"));
        assert_eq!(base_of("my-war.example.org:17495").as_deref(), Some("http://my-war.example.org:17495"));
        for no in ["", "https://x.org", "x.org/path", "user@x.org", "x.org:0", "x.org:port", "a b", "[::1]:17494", "x.org?y", "-x.org", "x.org:1:2"] {
            assert_eq!(base_of(no), None, "{no}");
        }
        // What `base_of` gives passes the check a selected war goes through.
        let war = crate::commander::WarRef { id: "k3x9q2".into(), name: "W".into(), host: crate::commander::WarHost::Lan, base: base_of("203.0.113.7"), save: None, computer: None, model: None };
        assert!(crate::commander::valid_pick(&war));
    }

    #[test]
    fn the_wars_of_an_answer_are_checked_like_announced_ones() {
        let body = br#"{"version":"0.5.0","protocol":1,"wars":[{"id":"k3x9q2","name":"Night War","model":"evolved","season":3,"players":2},{"id":"../x","name":"bad"},{"id":"ok_2"}]}"#;
        let found = read_info(body, "http://203.0.113.7:17494");
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].war.as_str(), found[0].name.as_str(), found[0].model.as_str(), found[0].season, found[0].players), ("k3x9q2", "Night War", "evolved", 3, 2));
        assert_eq!((found[0].base.as_str(), found[0].computer.as_str(), found[0].version.as_str()), ("http://203.0.113.7:17494", "203.0.113.7:17494", "0.5.0"));
        assert!(read_info(b"<html>", "http://x:1").is_empty());
    }

    #[test]
    fn asking_reads_a_real_answer_and_names_a_silent_address() {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let h = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut buf = [0u8; 2048];
            let n = s.read(&mut buf).unwrap();
            let body = r#"{"version":"0.5.0","wars":[{"id":"abc","name":"Home","season":1,"players":0}]}"#;
            write!(s, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            String::from_utf8_lossy(&buf[..n]).into_owned()
        });
        let found = ask(&format!("127.0.0.1:{port}")).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].base, format!("http://127.0.0.1:{port}"));
        assert!(h.join().unwrap().starts_with("GET /api/info "));
        assert!(ask(&format!("127.0.0.1:{port}")).is_err(), "nothing listens there now");
        assert_eq!(ask("not an address").unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
    }

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

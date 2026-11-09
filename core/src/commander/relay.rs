//! The relay: a small HTTP server on loopback that the game sends its Galactic War requests to while
//! a custom war is selected. It carries each one to the war's server and brings the answer back.
//!
//! On the way it removes the `auth` field, which holds the player's Steam session ticket: a war's
//! host never receives a ticket that could be replayed against the official servers. The player's
//! Steam ID from that field goes on as the `X-Commander-Player` header instead. The `sign` header is
//! dropped too. Only a path with the launch token is served, so another program on the PC that
//! finds the port gets nothing from it.

use std::io::Read;
use std::sync::{Arc, RwLock};
use std::time::Duration;

/// Where requests go now: the token the game's URL carries and the war's API.
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    pub token: String,
    pub upstream: String,
}

/// The game's requests are small form posts; a war answer is a few KB.
const MAX_BODY: usize = 1024 * 1024;

pub struct Relay {
    route: Arc<RwLock<Option<Route>>>,
    server: Arc<tiny_http::Server>,
    pub port: u16,
}

impl Relay {
    /// Listens on `127.0.0.1:port` and serves on a thread of its own until dropped.
    pub fn start(port: u16) -> std::io::Result<Relay> {
        let server = tiny_http::Server::http(("127.0.0.1", port)).map_err(|e| std::io::Error::other(e.to_string()))?;
        let port = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(port);
        let server = Arc::new(server);
        let route: Arc<RwLock<Option<Route>>> = Arc::new(RwLock::new(None));
        let (s, r) = (server.clone(), route.clone());
        std::thread::Builder::new().name("commander-relay".into()).spawn(move || serve(&s, &r))?;
        Ok(Relay { route, server, port })
    }

    /// Changes where requests go. None: every request is refused, so the game goes offline.
    pub fn set_route(&self, route: Option<Route>) {
        *self.route.write().unwrap_or_else(|p| p.into_inner()) = route;
    }

    pub fn route(&self) -> Option<Route> {
        self.route.read().unwrap_or_else(|p| p.into_inner()).clone()
    }
}

impl Drop for Relay {
    fn drop(&mut self) {
        self.server.unblock();
    }
}

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .user_agent("Obsidian-Relay")
        .timeout_connect(Some(Duration::from_secs(5)))
        .timeout_recv_response(Some(Duration::from_secs(20)))
        .timeout_recv_body(Some(Duration::from_secs(20)))
        .build()
        .into()
}

fn serve(server: &tiny_http::Server, route: &RwLock<Option<Route>>) {
    let agent = agent();
    for request in server.incoming_requests() {
        let current = route.read().unwrap_or_else(|p| p.into_inner()).clone();
        let agent = agent.clone();
        // One thread per request: a slow war server must not hold up the next poll.
        let _ = std::thread::Builder::new().name("commander-relay-req".into()).spawn(move || handle(request, current, &agent));
    }
}

fn text(code: u16, body: &str) -> tiny_http::Response<std::io::Cursor<Vec<u8>>> {
    tiny_http::Response::from_string(body).with_status_code(code)
}

fn handle(mut request: tiny_http::Request, route: Option<Route>, agent: &ureq::Agent) {
    let Some(route) = route else {
        let _ = request.respond(text(503, "no war selected"));
        return;
    };
    let Some(rest) = request.url().strip_prefix(&format!("/{}/", route.token)).map(str::to_string) else {
        let _ = request.respond(text(404, "not found"));
        return;
    };
    if !(rest == "1.0/" || rest == "1.0") || *request.method() != tiny_http::Method::Post {
        let _ = request.respond(text(404, "not found"));
        return;
    }
    let mut body = Vec::new();
    if request.as_reader().take(MAX_BODY as u64 + 1).read_to_end(&mut body).is_err() || body.len() > MAX_BODY {
        let _ = request.respond(text(413, "too large"));
        return;
    }
    let content_type = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Content-Type"))
        .map(|h| h.value.as_str().to_string())
        .unwrap_or_else(|| "application/x-www-form-urlencoded;charset=UTF-8".to_string());
    let (form, player) = strip_auth(&String::from_utf8_lossy(&body));
    let mut req = agent.post(&route.upstream).header("Content-Type", &content_type);
    if let Some(p) = &player {
        req = req.header("X-Commander-Player", p);
    }
    match req.send(form.as_bytes()) {
        Ok(mut res) => {
            let code = res.status().as_u16();
            let bytes = res.body_mut().with_config().limit(MAX_BODY as u64).read_to_vec().unwrap_or_default();
            let ct = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).expect("static header");
            let _ = request.respond(tiny_http::Response::from_data(bytes).with_status_code(code).with_header(ct));
        }
        Err(e) => {
            let _ = request.respond(text(502, &format!("war server unreachable: {e}")));
        }
    }
}

/// The form without its `auth` field, and the Steam ID that field named. The other fields stay
/// exactly as the game encoded them.
pub fn strip_auth(form: &str) -> (String, Option<String>) {
    let mut player = None;
    let kept: Vec<&str> = form
        .split('&')
        .filter(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            if k != "auth" {
                return true;
            }
            let json = percent_decode(v);
            player = serde_json::from_str::<serde_json::Value>(&json)
                .ok()
                .and_then(|j| j.get("online_id").and_then(|o| o.as_str().map(str::to_string)))
                .filter(|id| !id.is_empty() && id.len() <= 32 && id.chars().all(|c| c.is_ascii_hexdigit()));
            false
        })
        .collect();
    (kept.join("&"), player)
}

fn hex(c: u8) -> Option<u8> {
    (c as char).to_digit(16).map(|d| d as u8)
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < b.len() => match (hex(b[i + 1]), hex(b[i + 2])) {
                (Some(h), Some(l)) => {
                    out.push(h * 16 + l);
                    i += 3;
                }
                _ => {
                    out.push(b'%');
                    i += 1;
                }
            },
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;

    #[test]
    fn strips_the_ticket_and_keeps_the_rest_as_sent() {
        let auth = "%7B%22data%22%3A%22TICKET%22%2C%22online_id%22%3A%221100001066537b3%22%2C%22issuer_id%22%3A256%2C%22platform%22%3A%22win32%22%7D";
        let form = format!("action=complete_planets&auth={auth}&planets=%5B%7B%22index%22%3A1%7D%5D");
        let (out, player) = strip_auth(&form);
        assert_eq!(out, "action=complete_planets&planets=%5B%7B%22index%22%3A1%7D%5D");
        assert_eq!(player.as_deref(), Some("1100001066537b3"));
        assert!(!out.contains("TICKET"));
        let (out, player) = strip_auth("action=get_campaign_status");
        assert_eq!((out.as_str(), player), ("action=get_campaign_status", None));
        let (_, player) = strip_auth("auth=%7B%22online_id%22%3A%22not+hex%22%7D");
        assert_eq!(player, None, "only a hex Steam ID goes on");
    }

    /// A one-shot war server: answers the first request with `answer` and hands back what it got.
    fn fake_upstream(answer: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let h = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = s.read(&mut buf).unwrap_or(0);
                if n == 0 {
                    break;
                }
                got.extend_from_slice(&buf[..n]);
                let t = String::from_utf8_lossy(&got);
                if let Some(head_end) = t.find("\r\n\r\n") {
                    let len = t[..head_end].lines().find_map(|l| l.to_ascii_lowercase().strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))).unwrap_or(0);
                    if got.len() >= head_end + 4 + len {
                        break;
                    }
                }
            }
            write!(s, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", answer.len(), answer).unwrap();
            String::from_utf8_lossy(&got).into_owned()
        });
        (format!("http://127.0.0.1:{port}/w/war1/1.0/"), h)
    }

    #[test]
    fn carries_a_request_to_the_war_and_back() {
        let (upstream, seen) = fake_upstream(r#"{"time":1,"error_code":0}"#);
        let relay = Relay::start(0).unwrap();
        relay.set_route(Some(Route { token: "tok123".into(), upstream }));
        let agent = agent();
        let url = format!("http://127.0.0.1:{}/tok123/1.0/", relay.port);
        let mut res = agent
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded;charset=UTF-8")
            .send("action=upload_statistics&auth=%7B%22data%22%3A%22T%22%2C%22online_id%22%3A%22ab12%22%7D&statistics=%5B%5D")
            .unwrap();
        assert_eq!(res.status().as_u16(), 200);
        assert_eq!(res.body_mut().read_to_string().unwrap(), r#"{"time":1,"error_code":0}"#);
        let got = seen.join().unwrap();
        assert!(got.starts_with("POST /w/war1/1.0/ "), "{got}");
        assert!(got.to_ascii_lowercase().contains("x-commander-player: ab12"), "{got}");
        assert!(got.ends_with("action=upload_statistics&statistics=%5B%5D"), "{got}");
        assert!(!got.contains("%22T%22"), "the ticket never leaves");
    }

    /// Against a real Commander server: `COMMANDER_E2E` is a war's API URL. Run by ObsidianCommander's
    /// tools/e2e.py, which starts the server and creates the war.
    #[test]
    #[ignore]
    fn relay_reaches_a_real_war() {
        let upstream = std::env::var("COMMANDER_E2E").expect("COMMANDER_E2E: a war API URL");
        let relay = Relay::start(0).unwrap();
        relay.set_route(Some(Route { token: "e2e0".into(), upstream }));
        let agent = agent();
        let url = format!("http://127.0.0.1:{}/e2e0/1.0/", relay.port);
        let form = "application/x-www-form-urlencoded;charset=UTF-8";
        let mut res = agent.post(&url).header("Content-Type", form).send("action=get_campaign_status").unwrap();
        let status: serde_json::Value = serde_json::from_str(&res.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(status["error_code"], 0);
        assert_eq!(status["campaign_status"].as_array().map(Vec::len), Some(3));
        let auth = "%7B%22data%22%3A%22TICKET%22%2C%22online_id%22%3A%221100001066537b3%22%2C%22issuer_id%22%3A256%2C%22platform%22%3A%22win32%22%7D";
        let season = status["campaign_status"][0]["season"].as_u64().unwrap();
        let planets = format!("%5B%7B%22index%22%3A1%2C%22enemy%22%3A2%2C%22season%22%3A{season}%2C%22difficulty%22%3A4%2C%22success%22%3A0.7%7D%5D");
        let mut res = agent.post(&url).header("Content-Type", form).send(format!("action=complete_planets&auth={auth}&planets={planets}")).unwrap();
        let done: serde_json::Value = serde_json::from_str(&res.body_mut().read_to_string().unwrap()).unwrap();
        assert_eq!(done["error_code"], 0, "{done}");
        assert_eq!(done["completed_planets"], 1, "{done}");
        println!("E2E {done}");
    }

    #[test]
    fn refuses_without_the_token_or_a_route() {
        let relay = Relay::start(0).unwrap();
        let agent = agent();
        let url = format!("http://127.0.0.1:{}/tok123/1.0/", relay.port);
        assert_eq!(agent.post(&url).send("action=x").unwrap().status().as_u16(), 503, "no war selected");
        relay.set_route(Some(Route { token: "tok123".into(), upstream: "http://127.0.0.1:9/w/x/1.0/".into() }));
        let wrong = format!("http://127.0.0.1:{}/other/1.0/", relay.port);
        assert_eq!(agent.post(&wrong).send("action=x").unwrap().status().as_u16(), 404);
        assert_eq!(agent.post(&url).send("action=x").unwrap().status().as_u16(), 502, "a war server that's down");
    }
}

//! One-shot pairing. The TV shows a QR code and listens until Home Assistant
//! posts its address and the bridge code.

use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream, UdpSocket},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub struct Offer {
    pub ip: String,
    pub modules: Vec<bool>,
    pub size: usize,
}

pub struct Paired {
    pub url: String,
    pub token: String,
}

pub struct Server {
    stop: Sender<()>,
    paired: Receiver<Paired>,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.stop.send(());
    }
}

impl Server {
    pub fn try_paired(&self) -> Option<Paired> {
        self.paired.try_recv().ok()
    }
}

pub fn secret() -> String {
    let mut bytes = [0u8; 8];
    if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
        let _ = file.read_exact(&mut bytes);
    } else {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(1);
        bytes = (nanos as u64).to_le_bytes();
    }
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub fn local_ip() -> String {
    for target in ["192.168.3.1:9", "192.168.1.1:9", "10.0.0.1:9", "8.8.8.8:53"] {
        let Ok(socket) = UdpSocket::bind("0.0.0.0:0") else {
            continue;
        };
        if socket.connect(target).is_err() {
            continue;
        }
        if let Ok(addr) = socket.local_addr() {
            let ip = addr.ip();
            if ip.is_ipv4() && !ip.is_loopback() && !ip.is_unspecified() {
                return ip.to_string();
            }
        }
    }
    "0.0.0.0".into()
}

pub fn start(secret: String) -> Result<(Server, String, u16), String> {
    let listener = TcpListener::bind("0.0.0.0:0")
        .map_err(|_| "Telewizor nie mógł otworzyć portu do parowania".to_owned())?;
    listener
        .set_nonblocking(true)
        .map_err(|_| "Telewizor nie mógł otworzyć portu do parowania".to_owned())?;
    let port = listener
        .local_addr()
        .map_err(|_| "Telewizor nie mógł otworzyć portu do parowania".to_owned())?
        .port();
    let ip = local_ip();
    let (stop_tx, stop_rx) = mpsc::channel();
    let (paired_tx, paired_rx) = mpsc::channel();
    thread::spawn(move || serve(listener, secret, paired_tx, stop_rx));
    Ok((
        Server {
            stop: stop_tx,
            paired: paired_rx,
        },
        ip,
        port,
    ))
}

pub fn modules(payload: &str) -> Result<(Vec<bool>, usize), String> {
    let code = qrcode::QrCode::new(payload.as_bytes())
        .map_err(|_| "Nie da się zbudować kodu QR".to_owned())?;
    let size = code.width();
    let modules = code
        .to_colors()
        .into_iter()
        .map(|color| color == qrcode::types::Color::Dark)
        .collect();
    Ok((modules, size))
}

fn serve(listener: TcpListener, secret: String, paired: Sender<Paired>, stop: Receiver<()>) {
    let deadline = Instant::now() + Duration::from_secs(300);
    while Instant::now() < deadline {
        if stop.try_recv().is_ok() {
            return;
        }
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = stream.set_nonblocking(false);
                if let Some(claim) = read_claim(&stream, &secret) {
                    let _ = paired.send(claim);
                    return;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(40));
            }
            Err(_) => return,
        }
    }
}

fn read_claim(mut stream: &TcpStream, secret: &str) -> Option<Paired> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 1024];
    let start = Instant::now();
    while bytes.len() < 8192 && start.elapsed() < Duration::from_secs(3) {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                bytes.extend_from_slice(&chunk[..n]);
                if complete(&bytes) {
                    break;
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut =>
            {
                break;
            }
            Err(_) => break,
        }
    }
    let text = String::from_utf8_lossy(&bytes);
    let (head, body) = text.split_once("\r\n\r\n")?;
    let mut lines = head.lines();
    let request = lines.next().unwrap_or("");
    if !request.starts_with("POST /pair") {
        respond(stream, 404, r#"{"ok":false}"#);
        return None;
    }
    let claim: Claim = serde_json::from_str(body.trim()).ok()?;
    if claim.secret != secret {
        respond(stream, 403, r#"{"ok":false}"#);
        return None;
    }
    let url = crate::model::clean_ha_url(&claim.url);
    let token = crate::model::clean_ha_token(&claim.token);
    if url.is_empty() || token.is_empty() {
        respond(stream, 400, r#"{"ok":false}"#);
        return None;
    }
    respond(stream, 200, r#"{"ok":true}"#);
    Some(Paired { url, token })
}

fn complete(bytes: &[u8]) -> bool {
    let Some(split) = bytes.windows(4).position(|mark| mark == b"\r\n\r\n") else {
        return false;
    };
    let head = String::from_utf8_lossy(&bytes[..split]);
    let length = head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse::<usize>().ok())
            .flatten()
    });
    match length {
        Some(length) => bytes.len() >= split + 4 + length,
        None => true,
    }
}

fn respond(mut stream: &TcpStream, status: u16, body: &str) {
    let reason = if status == 200 { "OK" } else { "Error" };
    let message = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(message.as_bytes());
}

#[derive(serde::Deserialize)]
struct Claim {
    secret: String,
    url: String,
    token: String,
}

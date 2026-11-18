//! Talks to the Luma Bridge integration. The TV only sees the tiles that were
//! chosen in Home Assistant, and a press can only hit one of those tiles.

use serde::Deserialize;
use std::{
    io::Read,
    path::PathBuf,
    sync::{
        Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

const LIMIT: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tile {
    pub entity_id: String,
    pub name: String,
    pub domain: String,
    pub state: String,
    pub action: String,
    /// Lamp color, when the bulb is actually showing one. Absent for a plain white or yellow light.
    pub rgb: Option<[u8; 3]>,
}

impl Tile {
    /// Order of the groups on the Dom tab.
    pub fn group(&self) -> u8 {
        match self.domain.as_str() {
            "light" => 0,
            "camera" => 1,
            "switch" | "input_boolean" => 2,
            "cover" => 3,
            "fan" | "climate" => 4,
            "media_player" => 5,
            "lock" => 6,
            "scene" | "script" | "button" => 7,
            _ => 8,
        }
    }

    pub fn group_key(&self) -> &'static str {
        match self.group() {
            0 => "haLights",
            1 => "haCameras",
            2 => "haSwitches",
            3 => "haCovers",
            4 => "haClimate",
            5 => "haMedia",
            6 => "haLocks",
            7 => "haScenes",
            _ => "haOther",
        }
    }
}

#[derive(Debug)]
pub enum Update {
    Tiles(Vec<Tile>),
    Failed(String),
    Shot {
        entity_id: String,
        path: String,
    },
    Ring {
        id: u64,
        camera: String,
        title: String,
    },
    ShotFailed {
        entity_id: String,
        error: String,
    },
}

enum Job {
    Configure { url: String, token: String },
    Press(String),
    Watch(Option<String>),
    Stop,
}

pub struct Service {
    jobs: Sender<Job>,
    pub receiver: Receiver<Update>,
    pair: Mutex<Option<crate::pair::Server>>,
}

impl Service {
    pub fn new() -> Self {
        let (jobs_tx, jobs_rx) = mpsc::channel();
        let (updates_tx, updates_rx) = mpsc::channel();
        thread::spawn(move || worker(jobs_rx, updates_tx));
        Self {
            jobs: jobs_tx,
            receiver: updates_rx,
            pair: Mutex::new(None),
        }
    }

    pub fn start_pairing(&self) -> Result<crate::pair::Offer, String> {
        let secret = crate::pair::secret();
        let (server, ip, port) = crate::pair::start(secret.clone())?;
        let (modules, size) = match crate::pair::modules(&format!("L1|{ip}|{port}|{secret}")) {
            Ok(modules) => modules,
            Err(error) => {
                drop(server);
                return Err(error);
            }
        };
        *self.pair.lock().unwrap() = Some(server);
        Ok(crate::pair::Offer { ip, modules, size })
    }

    pub fn stop_pairing(&self) {
        self.pair.lock().unwrap().take();
    }

    pub fn poll_paired(&self) -> Option<crate::pair::Paired> {
        let mut guard = self.pair.lock().unwrap();
        let server = guard.as_ref()?;
        match server.try_paired() {
            Some(paired) => {
                guard.take();
                Some(paired)
            }
            None => None,
        }
    }

    pub fn configure(&self, url: &str, token: &str) {
        let _ = self.jobs.send(Job::Configure {
            url: url.into(),
            token: token.into(),
        });
    }

    pub fn press(&self, entity_id: String) {
        let _ = self.jobs.send(Job::Press(entity_id));
    }

    pub fn watch(&self, entity_id: Option<String>) {
        let _ = self.jobs.send(Job::Watch(entity_id));
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.jobs.send(Job::Stop);
        self.pair.lock().unwrap().take();
    }
}

fn worker(jobs: Receiver<Job>, updates: Sender<Update>) {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(8)))
        .max_redirects(0)
        .proxy(None)
        .build()
        .new_agent();
    let mut url = String::new();
    let mut token = String::new();
    let mut cameras: Vec<String> = Vec::new();
    let mut thumb = 0usize;
    let mut watch: Option<String> = None;
    let mut last_ring = 0u64;
    loop {
        let wait = if watch.is_some() {
            Duration::from_secs(1)
        } else {
            Duration::from_secs(2)
        };
        let job = if url.is_empty() || token.is_empty() {
            match jobs.recv() {
                Ok(job) => job,
                Err(_) => return,
            }
        } else {
            match jobs.recv_timeout(wait) {
                Ok(job) => job,
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if let Some(id) = watch.clone() {
                        report_snapshot(&agent, &url, &token, &id, 1920, &updates);
                    }
                    let (update, cams) = fetch(&agent, &url, &token);
                    cameras = cams;
                    let _ = updates.send(update);
                    if let Some(ring) = poll_ring(&agent, &url, &token)
                        && ring.id != last_ring
                    {
                        last_ring = ring.id;
                        let _ = updates.send(Update::Ring {
                            id: ring.id,
                            camera: ring.camera,
                            title: ring.title,
                        });
                    }
                    if !cameras.is_empty() {
                        let id = cameras[thumb % cameras.len()].clone();
                        thumb = thumb.wrapping_add(1);
                        if watch.as_deref() != Some(id.as_str()) {
                            report_snapshot(&agent, &url, &token, &id, 1280, &updates);
                        }
                    }
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
        };
        match job {
            Job::Stop => return,
            Job::Watch(id) => {
                watch = id.clone();
                if let Some(id) = id
                    && !url.is_empty()
                    && !token.is_empty()
                {
                    report_snapshot(&agent, &url, &token, &id, 1920, &updates);
                }
            }
            Job::Configure {
                url: next_url,
                token: next_token,
            } => {
                url = next_url;
                token = next_token;
                if !url.is_empty() && !token.is_empty() {
                    let (update, cams) = fetch(&agent, &url, &token);
                    cameras = cams;
                    let _ = updates.send(update);
                } else {
                    cameras.clear();
                    let _ = updates.send(Update::Tiles(Vec::new()));
                }
            }
            Job::Press(entity_id) => {
                if url.is_empty() || token.is_empty() {
                    continue;
                }
                if let Err(error) = press(&agent, &url, &token, &entity_id) {
                    let _ = updates.send(Update::Failed(error));
                }
                let (update, cams) = fetch(&agent, &url, &token);
                cameras = cams;
                let _ = updates.send(update);
            }
        }
    }
}

fn fetch(agent: &ureq::Agent, url: &str, token: &str) -> (Update, Vec<String>) {
    match tiles(agent, url, token) {
        Ok(list) => {
            let cameras = list
                .iter()
                .filter(|tile| tile.domain == "camera")
                .map(|tile| tile.entity_id.clone())
                .collect();
            (Update::Tiles(list), cameras)
        }
        Err(error) => (Update::Failed(error), Vec::new()),
    }
}

fn tiles(agent: &ureq::Agent, url: &str, token: &str) -> Result<Vec<Tile>, String> {
    let endpoint = format!("{url}/api/luma_bridge/tiles");
    let value = get_json(agent, &endpoint, token)?;
    let parsed: TileList =
        serde_json::from_value(value).map_err(|_| "Home Assistant zwrócił nieczytelną listę")?;
    Ok(parsed
        .tiles
        .into_iter()
        .filter_map(|tile| tile.into_tile())
        .take(60)
        .collect())
}

fn press(agent: &ureq::Agent, url: &str, token: &str, entity_id: &str) -> Result<(), String> {
    if !valid_entity(entity_id) {
        return Err("Nieznany kafelek".into());
    }
    let endpoint = format!("{url}/api/luma_bridge/press");
    let body = format!(r#"{{"entity_id":"{entity_id}"}}"#);
    let mut response = agent
        .post(&endpoint)
        .header("Authorization", &format!("Bearer {token}"))
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("User-Agent", "LumaNative")
        .send(body.as_bytes())
        .map_err(|_| "Home Assistant nie przyjął naciśnięcia".to_owned())?;
    if !response.status().is_success() {
        return Err("Home Assistant odrzucił naciśnięcie".into());
    }
    let _ = std::io::copy(
        &mut response.body_mut().as_reader().take(1024),
        &mut std::io::sink(),
    );
    Ok(())
}

fn get_json(agent: &ureq::Agent, endpoint: &str, token: &str) -> Result<serde_json::Value, String> {
    let mut response = agent
        .get(endpoint)
        .header("Authorization", &format!("Bearer {token}"))
        .header("Accept", "application/json")
        .header("User-Agent", "LumaNative")
        .call()
        .map_err(|_| "Nie ma połączenia z Home Assistant".to_owned())?;
    if response.status().as_u16() == 401 {
        return Err("Zły kod Home Assistant".into());
    }
    if !response.status().is_success() {
        return Err("Home Assistant nie odpowiedział".into());
    }
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Nie dało się odczytać odpowiedzi".to_owned())?;
    if bytes.len() > LIMIT {
        return Err("Odpowiedź Home Assistant jest za duża".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Home Assistant zwrócił nieczytelne dane".into())
}

fn report_snapshot(
    agent: &ureq::Agent,
    url: &str,
    token: &str,
    entity_id: &str,
    max_width: u32,
    updates: &Sender<Update>,
) {
    match save_snapshot(agent, url, token, entity_id, max_width) {
        Ok(path) => {
            let _ = updates.send(Update::Shot {
                entity_id: entity_id.to_owned(),
                path,
            });
        }
        Err(error) => {
            let _ = updates.send(Update::ShotFailed {
                entity_id: entity_id.to_owned(),
                error,
            });
        }
    }
}

fn save_snapshot(
    agent: &ureq::Agent,
    url: &str,
    token: &str,
    entity_id: &str,
    max_width: u32,
) -> Result<String, String> {
    if !valid_entity(entity_id) || !entity_id.starts_with("camera.") {
        return Err("To nie jest kamera".into());
    }
    let endpoint = format!("{url}/api/luma_bridge/snapshot?entity_id={entity_id}");
    let mut response = agent
        .get(&endpoint)
        .header("Authorization", &format!("Bearer {token}"))
        .header("Accept", "image/jpeg")
        .header("User-Agent", "LumaNative")
        .call()
        .map_err(|_| "Nie ma połączenia z kamerą".to_owned())?;
    if !response.status().is_success() {
        return Err("Home Assistant nie dał obrazu z kamery".into());
    }
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(6_000_000)
        .read_to_end(&mut bytes)
        .map_err(|_| "Obraz z kamery się urwał".to_owned())?;
    if bytes.len() < 32 {
        return Err("Kamera odesłała pusty obraz".into());
    }
    let mut picture = image::load_from_memory(&bytes)
        .map_err(|_| "Kamera odesłała nieczytelny obraz".to_owned())?;
    if picture.width() > max_width {
        let height = (u64::from(picture.height()) * u64::from(max_width)
            / u64::from(picture.width()))
        .max(1) as u32;
        picture = picture.resize(max_width, height, image::imageops::FilterType::Lanczos3);
    }
    let path = shot_path(entity_id);
    let directory = path
        .parent()
        .ok_or_else(|| "Nie dało się zapisać zdjęcia".to_owned())?;
    std::fs::create_dir_all(directory).map_err(|_| "Nie dało się zapisać zdjęcia".to_owned())?;
    let temporary = path.with_extension("part.jpg");
    let rgb = picture.to_rgb8();
    let file =
        std::fs::File::create(&temporary).map_err(|_| "Nie dało się zapisać zdjęcia".to_owned())?;
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(file, 92);
    encoder
        .encode(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|_| "Nie dało się zapisać zdjęcia".to_owned())?;
    std::fs::rename(&temporary, &path).map_err(|_| "Nie dało się zapisać zdjęcia".to_owned())?;
    Ok(path.to_string_lossy().into_owned())
}

struct Ring {
    id: u64,
    camera: String,
    title: String,
}

fn poll_ring(agent: &ureq::Agent, url: &str, token: &str) -> Option<Ring> {
    let endpoint = format!("{url}/api/luma_bridge/alert");
    let value = get_json(agent, &endpoint, token).ok()?;
    let id = value.get("id")?.as_u64()?;
    if id == 0 {
        return None;
    }
    let camera = value.get("camera")?.as_str()?.to_owned();
    if !valid_entity(&camera) {
        return None;
    }
    let title = value
        .get("title")
        .and_then(|item| item.as_str())
        .unwrap_or("Ktoś dzwoni")
        .to_owned();
    Some(Ring { id, camera, title })
}

fn shot_path(entity_id: &str) -> PathBuf {
    let safe: String = entity_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    PathBuf::from(format!("/tmp/luma-cam/{safe}.jpg"))
}

fn valid_entity(id: &str) -> bool {
    let Some((domain, object)) = id.split_once('.') else {
        return false;
    };
    !domain.is_empty()
        && !object.is_empty()
        && id.len() <= 128
        && id.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_')
        })
}

#[derive(Deserialize)]
struct TileList {
    #[serde(default)]
    tiles: Vec<RawTile>,
}

#[derive(Deserialize)]
struct RawTile {
    entity_id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    domain: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    action: String,
    #[serde(default)]
    rgb: Option<[u8; 3]>,
}

impl RawTile {
    fn into_tile(self) -> Option<Tile> {
        if !valid_entity(&self.entity_id) {
            return None;
        }
        let domain = if self.domain.is_empty() {
            self.entity_id
                .split_once('.')
                .map(|(d, _)| d)
                .unwrap_or("")
                .into()
        } else {
            self.domain
        };
        let name = if self.name.trim().is_empty() {
            self.entity_id.clone()
        } else {
            self.name.trim().chars().take(40).collect()
        };
        Some(Tile {
            entity_id: self.entity_id,
            name,
            domain,
            state: if self.state.is_empty() {
                "unknown".into()
            } else {
                self.state
            },
            action: if self.action.is_empty() {
                "toggle".into()
            } else {
                self.action
            },
            rgb: self.rgb,
        })
    }
}

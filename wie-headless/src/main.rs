//! Runs one app without a window and reports what happened, for automated checks of many apps.
//!
//!   wie-headless <app.jar> --size 176x208 --out <dir> [--script "<steps>"] [--turbo]
//!
//! The app gets a screen of the given size. The script is a list of steps separated by spaces:
//!   w<ms>              run for this long
//!   k<KEY>[:<ms>]      press a key (UP DOWN LEFT RIGHT OK LSK RSK CLR 0-9 * #), hold it (default 150 ms), release it
//!   s<name>            save the current frame as <dir>/<name>.png
//!   m<name>            start a measuring window (ends at the next m or at the end of the script)
//!
//! The loop runs like the web player: every 16.7 ms a display refresh (redraw), key repeat every 100 ms while a key is
//! held, one emulator tick (at most 8 ms of work), then it waits for the next refresh. `--turbo` does not wait: the app
//! gets all the CPU (compare both to see whether an app is held back by the tick budget).
//!
//! Writes <dir>/result.json (paint counts, frame changes and tick times per window, the app's output and warnings with
//! counts, a fatal error or panic, whether it exited or hung) and <dir>/activity.png (how often each pixel changed).

use std::{
    collections::BTreeMap,
    fs,
    io::{self, Write},
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::{Duration, Instant as StdInstant, SystemTime, UNIX_EPOCH},
};

use hashbrown::HashMap;
use serde_json::{Value, json};
use wie_util::Result as WieResult;

use wie_backend::{
    AudioCommand, AudioSink, Database, DatabaseRepository, Event, Filesystem, Font, Instant, KeyCode, Platform, RecordId, Screen, canvas::Image,
};

const FRAME_MS: f64 = 1000.0 / 60.0;
const KEY_REPEAT_MS: f64 = 100.0;
const MAX_LOG_LINES: usize = 400;
const HANG_MS: u64 = 20_000;

// ---------------------------------------------------------------- state shared with the screen, platform and watchdog
#[derive(Default)]
struct WindowStats {
    name: String,
    start_ms: f64,
    end_ms: f64,
    paints: u64,
    changed_paints: u64,
    ticks: u64,
    tick_ms_sum: f64,
    tick_ms_max: f64,
    full_ticks: u64,
    base: Vec<u32>,
    union: Vec<bool>,
}

impl WindowStats {
    fn json(&self) -> Value {
        let changed_px = self.union.iter().filter(|x| **x).count();
        let secs = ((self.end_ms - self.start_ms) / 1000.0).max(1e-3);
        json!({
            "name": self.name, "start_ms": self.start_ms.round(), "ms": (self.end_ms - self.start_ms).round(),
            "paints": self.paints, "changed_paints": self.changed_paints, "fps": (self.changed_paints as f64 / secs * 10.0).round() / 10.0,
            "changed_px": changed_px, "ticks": self.ticks,
            "tick_ms_avg": if self.ticks > 0 { (self.tick_ms_sum / self.ticks as f64 * 100.0).round() / 100.0 } else { 0.0 },
            "tick_ms_max": (self.tick_ms_max * 100.0).round() / 100.0, "full_ticks": self.full_ticks,
        })
    }
}

struct Shared {
    width: u32,
    height: u32,
    started: StdInstant,
    frame: Vec<u32>,
    paints: u64,
    changed_paints: u64,
    first_paint_ms: Option<f64>,
    first_content_ms: Option<f64>,
    activity: Vec<u16>,
    windows: Vec<WindowStats>,
    log: Vec<String>,
    log_counts: BTreeMap<String, u64>,
    error: Option<String>,
    panic: Option<String>,
}

impl Shared {
    fn ms(&self) -> f64 {
        self.started.elapsed().as_secs_f64() * 1000.0
    }

    fn add_log(&mut self, line: &str) {
        let line = line.trim_end();
        if line.is_empty() {
            return;
        }
        // Group repeated lines without their changing parts (object ids, numbers) for the counts.
        let key: String = line.chars().map(|c| if c.is_ascii_digit() { '#' } else { c }).take(240).collect();
        let count = self.log_counts.entry(key).or_default();
        *count += 1;
        if *count == 1 && self.log.len() < MAX_LOG_LINES {
            self.log.push(line.chars().take(500).collect());
        }
    }
}

type SharedRef = Arc<Mutex<Shared>>;

// ---------------------------------------------------------------- screen
struct HeadlessScreen {
    shared: SharedRef,
}

impl Screen for HeadlessScreen {
    fn resize(&self, _width: u32, _height: u32) -> WieResult<()> {
        Ok(()) // the size is fixed by --size, like the web player's canvas
    }

    fn request_redraw(&self) -> WieResult<()> {
        Ok(())
    }

    fn paint(&self, image: &dyn Image) {
        let mut s = self.shared.lock().unwrap();
        let (w, h) = (s.width as usize, s.height as usize);
        let (iw, ih) = (image.width() as usize, image.height() as usize);
        let colors = image.colors();
        let now = s.ms();
        let mut changed = false;
        let mut content = false;
        let first = s.frame[0];
        let Shared {
            frame, activity, windows, ..
        } = &mut *s;
        let window = windows.last_mut();
        let (mut base, mut union) = match window {
            Some(win) => (Some(&win.base), Some(&mut win.union)),
            None => (None, None),
        };
        for y in 0..h.min(ih) {
            for x in 0..w.min(iw) {
                let c = colors[y * iw + x];
                let v = ((c.r as u32) << 16) | ((c.g as u32) << 8) | c.b as u32;
                let i = y * w + x;
                if v != first {
                    content = true;
                }
                if frame[i] != v {
                    changed = true;
                    frame[i] = v;
                    activity[i] = activity[i].saturating_add(1);
                }
                if let (Some(base), Some(union)) = (base.as_mut(), union.as_mut())
                    && base[i] != v
                {
                    union[i] = true;
                }
            }
        }
        s.paints += 1;
        if s.first_paint_ms.is_none() {
            s.first_paint_ms = Some(now);
        }
        if (content || changed) && s.first_content_ms.is_none() && s.paints > 0 {
            s.first_content_ms = Some(now);
        }
        if changed {
            s.changed_paints += 1;
        }
        if let Some(win) = s.windows.last_mut() {
            win.paints += 1;
            if changed {
                win.changed_paints += 1;
            }
        }
    }

    fn width(&self) -> u32 {
        self.shared.lock().unwrap().width
    }

    fn height(&self) -> u32 {
        self.shared.lock().unwrap().height
    }
}

// ---------------------------------------------------------------- in-memory storage, silent audio
#[derive(Default)]
struct MemoryFilesystem {
    files: spin::Mutex<HashMap<(String, String), Vec<u8>>>,
}

#[async_trait::async_trait]
impl Filesystem for MemoryFilesystem {
    async fn remove(&self, pid: &str, path: &str) -> bool {
        self.files.lock().remove(&(pid.into(), path.into())).is_some()
    }
    async fn usage(&self, pid: &str) -> u64 {
        self.files.lock().iter().filter(|((p, _), _)| p == pid).map(|(_, d)| d.len() as u64).sum()
    }
    async fn exists(&self, pid: &str, path: &str) -> bool {
        self.files.lock().contains_key(&(pid.into(), path.into()))
    }
    async fn size(&self, pid: &str, path: &str) -> Option<usize> {
        self.files.lock().get(&(pid.into(), path.into())).map(|d| d.len())
    }
    async fn read(&self, pid: &str, path: &str, offset: usize, count: usize, buf: &mut [u8]) -> Option<usize> {
        let files = self.files.lock();
        let data = files.get(&(pid.into(), path.into()))?;
        if offset >= data.len() {
            return Some(0);
        }
        let n = count.min(data.len() - offset);
        buf[..n].copy_from_slice(&data[offset..offset + n]);
        Some(n)
    }
    async fn write(&self, pid: &str, path: &str, offset: usize, data: &[u8]) -> usize {
        let mut files = self.files.lock();
        let file = files.entry((pid.into(), path.into())).or_default();
        if file.len() < offset + data.len() {
            file.resize(offset + data.len(), 0);
        }
        file[offset..offset + data.len()].copy_from_slice(data);
        data.len()
    }
    async fn truncate(&self, pid: &str, path: &str, len: usize) {
        self.files.lock().entry((pid.into(), path.into())).or_default().resize(len, 0);
    }
}

type Store = Arc<spin::Mutex<HashMap<(String, String), BTreeMap<RecordId, Vec<u8>>>>>;

#[derive(Default)]
struct MemoryDatabaseRepository {
    store: Store,
}

#[async_trait::async_trait]
impl DatabaseRepository for MemoryDatabaseRepository {
    async fn create(&self, name: &str, app_id: &str) {
        self.store.lock().entry((app_id.into(), name.into())).or_default();
    }
    async fn open(&self, name: &str, app_id: &str) -> Box<dyn Database> {
        Box::new(MemoryDatabase {
            store: self.store.clone(),
            key: (app_id.into(), name.into()),
        })
    }
    async fn exists(&self, name: &str, app_id: &str) -> bool {
        self.store.lock().contains_key(&(app_id.into(), name.into()))
    }
    async fn delete(&self, name: &str, app_id: &str) -> bool {
        self.store.lock().remove(&(app_id.into(), name.into())).is_some()
    }
}

struct MemoryDatabase {
    store: Store,
    key: (String, String),
}

#[async_trait::async_trait]
impl Database for MemoryDatabase {
    async fn next_id(&self) -> RecordId {
        self.store.lock().get(&self.key).and_then(|r| r.keys().last().copied()).unwrap_or(0) + 1
    }
    async fn add(&mut self, data: &[u8]) -> RecordId {
        let id = self.next_id().await;
        self.set(id, data).await;
        id
    }
    async fn get(&self, id: RecordId) -> Option<Vec<u8>> {
        self.store.lock().get(&self.key)?.get(&id).cloned()
    }
    async fn set(&mut self, id: RecordId, data: &[u8]) -> bool {
        self.store.lock().entry(self.key.clone()).or_default().insert(id, data.to_vec());
        true
    }
    async fn delete(&mut self, id: RecordId) -> bool {
        self.store.lock().get_mut(&self.key).is_some_and(|r| r.remove(&id).is_some())
    }
    async fn get_record_ids(&self) -> Vec<RecordId> {
        self.store.lock().get(&self.key).map(|r| r.keys().copied().collect()).unwrap_or_default()
    }
}

struct SilentAudio;

impl AudioSink for SilentAudio {
    fn send(&self, _command: AudioCommand) {}
}

// ---------------------------------------------------------------- platform
struct HeadlessPlatform {
    font: Font,
    screen: HeadlessScreen,
    db: MemoryDatabaseRepository,
    fs: MemoryFilesystem,
    shared: SharedRef,
    exited: Arc<AtomicBool>,
}

impl Platform for HeadlessPlatform {
    fn font(&self) -> &Font {
        &self.font
    }
    fn screen(&self) -> &dyn Screen {
        &self.screen
    }
    fn now(&self) -> Instant {
        Instant::from_epoch_millis(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as _)
    }
    fn database_repository(&self) -> &dyn DatabaseRepository {
        &self.db
    }
    fn filesystem(&self) -> &dyn Filesystem {
        &self.fs
    }
    fn audio_sink(&self) -> Box<dyn AudioSink> {
        Box::new(SilentAudio)
    }
    fn write_stdout(&self, buf: &[u8]) {
        let mut s = self.shared.lock().unwrap();
        for line in String::from_utf8_lossy(buf).lines() {
            s.add_log(&format!("stdout: {line}"));
        }
    }
    fn write_stderr(&self, buf: &[u8]) {
        let mut s = self.shared.lock().unwrap();
        for line in String::from_utf8_lossy(buf).lines() {
            s.add_log(&format!("stderr: {line}"));
        }
    }
    fn exit(&self) {
        self.exited.store(true, Ordering::SeqCst);
    }
    fn vibrate(&self, _duration_ms: u64, _intensity: u8) {}
}

// The engine's own warnings and errors (stubs, unimplemented APIs) go to the same log.
struct LogWriter(SharedRef);

impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Ok(mut s) = self.0.lock() {
            for line in String::from_utf8_lossy(buf).lines() {
                s.add_log(line);
            }
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------- script
enum Step {
    Wait(f64),
    Key(KeyCode, f64),
    Shot(String),
    Mark(String),
}

const KEYS: [&str; 21] = [
    "UP", "DOWN", "LEFT", "RIGHT", "OK", "LSK", "RSK", "CLR", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "*", "#", "",
];

fn parse_script(script: &str) -> anyhow::Result<Vec<Step>> {
    script
        .split_whitespace()
        .map(|word| {
            let (kind, rest) = word.split_at(1);
            Ok(match kind {
                "w" => Step::Wait(rest.parse()?),
                "k" => {
                    let (key, hold) = rest.split_once(':').unwrap_or((rest, "150"));
                    anyhow::ensure!(!key.is_empty() && KEYS.contains(&key), "unknown key {key}");
                    Step::Key(KeyCode::parse(key), hold.parse()?)
                }
                "s" => Step::Shot(rest.to_string()),
                "m" => Step::Mark(rest.to_string()),
                _ => anyhow::bail!("unknown step {word}"),
            })
        })
        .collect()
}

// ---------------------------------------------------------------- the run
struct Runner {
    emulator: Box<dyn wie_backend::Emulator>,
    shared: SharedRef,
    exited: Arc<AtomicBool>,
    progress: Arc<AtomicU64>,
    turbo: bool,
    next_frame: f64,
    held: Vec<(KeyCode, f64)>,
    stopped: bool,
}

impl Runner {
    fn ms(&self) -> f64 {
        self.shared.lock().unwrap().ms()
    }

    fn run_for(&mut self, duration: f64) {
        let end = self.ms() + duration;
        while !self.stopped {
            let now = self.ms();
            if now >= end {
                break;
            }
            if now >= self.next_frame {
                self.emulator.handle_event(Event::Redraw);
                for (key, at) in self.held.iter_mut() {
                    if now - *at > KEY_REPEAT_MS {
                        self.emulator.handle_event(Event::Keyrepeat(*key));
                        *at = now;
                    }
                }
                self.next_frame += FRAME_MS;
                if self.next_frame < now {
                    self.next_frame = now + FRAME_MS; // fell behind: no burst of catch-up frames
                }
            }
            let started = StdInstant::now();
            self.progress.store(now as u64, Ordering::SeqCst);
            let emulator = &mut self.emulator;
            let result = panic::catch_unwind(AssertUnwindSafe(|| emulator.tick()));
            let tick_ms = started.elapsed().as_secs_f64() * 1000.0;
            {
                let mut s = self.shared.lock().unwrap();
                if let Some(win) = s.windows.last_mut() {
                    win.ticks += 1;
                    win.tick_ms_sum += tick_ms;
                    win.tick_ms_max = win.tick_ms_max.max(tick_ms);
                    if tick_ms >= 7.5 {
                        win.full_ticks += 1;
                    }
                }
                match result {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        s.error = Some(format!("{error}"));
                        self.stopped = true;
                    }
                    Err(payload) => {
                        let message = payload
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| payload.downcast_ref::<&str>().map(|x| x.to_string()))
                            .unwrap_or_else(|| "panic".into());
                        if s.panic.is_none() {
                            s.panic = Some(message);
                        }
                        self.stopped = true;
                    }
                }
            }
            if self.exited.load(Ordering::SeqCst) {
                self.stopped = true;
            }
            if !self.turbo {
                let wait = self.next_frame - self.ms();
                if wait > 0.0 {
                    thread::sleep(Duration::from_secs_f64(wait.min(end - self.ms()).max(0.0) / 1000.0));
                }
            }
        }
    }

    fn mark(&mut self, name: &str) {
        let mut s = self.shared.lock().unwrap();
        let now = s.ms();
        if let Some(win) = s.windows.last_mut() {
            win.end_ms = now;
        }
        let base = s.frame.clone();
        let len = base.len();
        s.windows.push(WindowStats {
            name: name.into(),
            start_ms: now,
            end_ms: now,
            base,
            union: vec![false; len],
            ..Default::default()
        });
    }
}

fn save_png(path: &Path, width: u32, height: u32, pixels: &[u32]) -> anyhow::Result<()> {
    let bytes: Vec<u8> = pixels.iter().flat_map(|v| [(v >> 16) as u8, (v >> 8) as u8, *v as u8]).collect();
    image::save_buffer(path, &bytes, width, height, image::ExtendedColorType::Rgb8)?;
    Ok(())
}

fn write_result(out: &Path, shared: &SharedRef, extra: Value) -> anyhow::Result<()> {
    let s = shared.lock().unwrap_or_else(|e| e.into_inner());
    let now = s.ms();
    let mut windows: Vec<Value> = Vec::new();
    for (i, win) in s.windows.iter().enumerate() {
        let mut j = win.json();
        if i + 1 == s.windows.len() {
            let secs = ((now - win.start_ms) / 1000.0).max(1e-3);
            j["ms"] = json!((now - win.start_ms).round());
            j["fps"] = json!((win.changed_paints as f64 / secs * 10.0).round() / 10.0);
        }
        windows.push(j);
    }
    let mut counts: Vec<(&String, &u64)> = s.log_counts.iter().collect();
    counts.sort_by(|a, b| b.1.cmp(a.1));
    let mut result = json!({
        "size": [s.width, s.height], "ms": now.round(), "paints": s.paints, "changed_paints": s.changed_paints,
        "first_paint_ms": s.first_paint_ms.map(f64::round), "first_content_ms": s.first_content_ms.map(f64::round),
        "error": s.error, "panic": s.panic, "windows": windows,
        "log": s.log, "log_counts": counts.iter().take(80).map(|(k, v)| json!([k, v])).collect::<Vec<_>>(),
    });
    if let (Value::Object(r), Value::Object(e)) = (&mut result, extra) {
        r.extend(e);
    }
    let activity: Vec<u32> = s
        .activity
        .iter()
        .map(|c| {
            let v = (*c as u32 * 8).min(255);
            (v << 16) | (v << 8) | v
        })
        .collect();
    save_png(&out.join("activity.png"), s.width, s.height, &activity)?;
    fs::write(out.join("result.json"), serde_json::to_vec_pretty(&result)?)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let (mut jar, mut size, mut out, mut script, mut turbo) = (None, (240u32, 320u32), None, String::from("w5000 send"), false);
    let (mut jad, mut properties) = (None, BTreeMap::new());
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--size" => {
                let v = args.next().unwrap_or_default();
                let (w, h) = v.split_once('x').ok_or_else(|| anyhow::anyhow!("--size WxH"))?;
                size = (w.parse()?, h.parse()?);
            }
            "--out" => out = args.next().map(PathBuf::from),
            "--script" => script = args.next().unwrap_or_default(),
            "--turbo" => turbo = true,
            "--jad" => jad = Some(fs::read(args.next().unwrap_or_default())?),
            "--property" => {
                let v = args.next().unwrap_or_default();
                let (key, value) = v.split_once('=').ok_or_else(|| anyhow::anyhow!("--property KEY=VALUE"))?;
                properties.insert(key.to_string(), value.to_string());
            }
            _ if jar.is_none() => jar = Some(arg),
            _ => anyhow::bail!("unexpected argument {arg}"),
        }
    }
    let jar = jar.ok_or_else(|| {
        anyhow::anyhow!("usage: wie-headless <app.jar> --size WxH --out <dir> [--script ...] [--turbo] [--jad <app.jad>] [--property KEY=VALUE]...")
    })?;
    let out = out.ok_or_else(|| anyhow::anyhow!("--out <dir> is required"))?;
    fs::create_dir_all(&out)?;
    let steps = parse_script(&script)?;

    let (width, height) = size;
    let pixels = (width * height) as usize;
    let shared: SharedRef = Arc::new(Mutex::new(Shared {
        width,
        height,
        started: StdInstant::now(),
        frame: vec![0; pixels],
        paints: 0,
        changed_paints: 0,
        first_paint_ms: None,
        first_content_ms: None,
        activity: vec![0; pixels],
        windows: Vec::new(),
        log: Vec::new(),
        log_counts: BTreeMap::new(),
        error: None,
        panic: None,
    }));

    let log_shared = shared.clone();
    tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_target(false)
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()))
        .with_writer(move || LogWriter(log_shared.clone()))
        .init();
    panic::set_hook(Box::new(|_| {})); // panics are caught and reported in result.json

    let exited = Arc::new(AtomicBool::new(false));
    let platform = Box::new(HeadlessPlatform {
        font: Font::try_from_static(include_bytes!("../../assets/neodgm.ttf"))?,
        screen: HeadlessScreen { shared: shared.clone() },
        db: MemoryDatabaseRepository::default(),
        fs: MemoryFilesystem::default(),
        shared: shared.clone(),
        exited: exited.clone(),
    });

    // Watchdog: a tick that never returns (an app looping without yielding) is reported as a hang.
    let progress = Arc::new(AtomicU64::new(0));
    {
        let (shared, progress, out) = (shared.clone(), progress.clone(), out.clone());
        thread::spawn(move || {
            let mut last = (u64::MAX, StdInstant::now());
            loop {
                thread::sleep(Duration::from_millis(500));
                let p = progress.load(Ordering::SeqCst);
                if p != last.0 {
                    last = (p, StdInstant::now());
                } else if last.1.elapsed().as_millis() as u64 > HANG_MS {
                    let _ = write_result(&out, &shared, json!({"hang": true, "exited": false, "load_error": null}));
                    std::process::exit(3);
                }
            }
        });
    }

    let bytes = fs::read(&jar)?;
    let filename = Path::new(&jar).file_name().unwrap().to_string_lossy().to_string();
    let options = wie_backend::Options {
        enable_gdbserver: false,
        aot: None,
        profile: None,
    };
    let load = || {
        if jad.is_some() || !properties.is_empty() {
            wie::load_j2me_emulator(&filename, bytes, jad.as_deref(), properties, platform)
        } else {
            wie::load_emulator(&filename, bytes, platform, options)
        }
    };
    let emulator = match panic::catch_unwind(AssertUnwindSafe(load)) {
        Ok(Ok(emulator)) => emulator,
        Ok(Err(error)) => {
            write_result(&out, &shared, json!({"hang": false, "exited": false, "load_error": format!("{error:#}")}))?;
            return Ok(());
        }
        Err(_) => {
            write_result(
                &out,
                &shared,
                json!({"hang": false, "exited": false, "load_error": "panic while loading"}),
            )?;
            return Ok(());
        }
    };

    let mut runner = Runner {
        emulator,
        shared: shared.clone(),
        exited: exited.clone(),
        progress,
        turbo,
        next_frame: 0.0,
        held: Vec::new(),
        stopped: false,
    };
    runner.mark("boot");
    for step in steps {
        if runner.stopped {
            break;
        }
        match step {
            Step::Wait(ms) => runner.run_for(ms),
            Step::Key(key, hold) => {
                runner.emulator.handle_event(Event::Keydown(key));
                let at = runner.ms();
                runner.held.push((key, at));
                runner.run_for(hold);
                runner.held.retain(|(k, _)| *k != key);
                runner.emulator.handle_event(Event::Keyup(key));
            }
            Step::Shot(name) => {
                let s = shared.lock().unwrap();
                save_png(&out.join(format!("{name}.png")), s.width, s.height, &s.frame)?;
            }
            Step::Mark(name) => runner.mark(&name),
        }
    }
    write_result(
        &out,
        &shared,
        json!({"hang": false, "exited": exited.load(Ordering::SeqCst), "load_error": null, "turbo": turbo}),
    )?;
    // Leave without dropping the emulator: its threads and tasks are not needed any more.
    std::process::exit(0);
}

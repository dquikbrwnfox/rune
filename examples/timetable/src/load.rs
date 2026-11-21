//! Loading `celcat.json`: parse into the model, and fetch it without blocking the first frame.
//! Web: an async `fetch` of `celcat.json` next to the page. Native: the file named by the
//! `TIMETABLE_DATA` environment variable. The result lands in an inbox that `poll` drains.

use std::cell::RefCell;

use serde::Deserialize;

use crate::data::{Catalogue, Course, Group, Meta, SKind, Session, Stream, mask};
use crate::model::tm;

/// How the catalogue stands.
#[derive(Clone, PartialEq, Eq)]
pub enum Status {
    Loading,
    Ready,
    /// The sample catalogue stands in; the reason is shown to the user.
    Fallback(String),
}

/// Rough timings of the last load, in milliseconds.
#[derive(Clone, Copy, Default)]
pub struct Timing { pub fetch_ms: f64, pub parse_ms: f64 }

pub type Loaded = Result<(Catalogue, Timing), String>;

thread_local! {
    static INBOX: RefCell<Option<Loaded>> = const { RefCell::new(None) };
}

pub fn take_inbox() -> Option<Loaded> { INBOX.with(|i| i.borrow_mut().take()) }
fn put_inbox(r: Loaded) { INBOX.with(|i| *i.borrow_mut() = Some(r)); }

/// Kicks the load off; the waker wakes the window when the result is in the inbox.
pub fn start(waker: rune::Waker) {
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        put_inbox(web::fetch_catalogue().await);
        waker();
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        let r = match std::env::var("TIMETABLE_DATA") {
            Ok(path) => std::fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))
                .and_then(|t| parse_timed(&t, 0.0)),
            Err(_) => Err("TIMETABLE_DATA is not set".to_owned()),
        };
        put_inbox(r);
        waker();
    }
}

pub fn now_ms() -> f64 {
    #[cfg(target_arch = "wasm32")]
    { js_sys::Date::now() }
    #[cfg(not(target_arch = "wasm32"))]
    { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64() * 1000.0) }
}

fn parse_timed(text: &str, fetch_ms: f64) -> Loaded {
    let t0 = now_ms();
    let cat = parse(text)?;
    Ok((cat, Timing { fetch_ms, parse_ms: now_ms() - t0 }))
}

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    pub async fn fetch_catalogue() -> super::Loaded {
        let t0 = super::now_ms();
        let win = web_sys::window().ok_or("no window")?;
        let resp = JsFuture::from(win.fetch_with_str("celcat.json")).await.map_err(|e| format!("fetch failed: {e:?}"))?;
        let resp: web_sys::Response = resp.dyn_into().map_err(|_| "bad response".to_owned())?;
        if !resp.ok() { return Err(format!("celcat.json: HTTP {}", resp.status())); }
        let text = JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?).await.map_err(|e| format!("{e:?}"))?
            .as_string().ok_or("celcat.json is not text")?;
        super::parse_timed(&text, super::now_ms() - t0)
    }
}

// ───────── JSON → model ─────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Doc {
    #[serde(default)] generated: String,
    #[serde(default)] source: String,
    #[serde(default)] published_at: String,
    #[serde(default)] term: String,
    #[serde(default)] term_label: String,
    #[serde(default)] calendar: Cal,
    #[serde(default)] weeks_published: Vec<u32>,
    courses: Vec<DCourse>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Cal { #[serde(default)] week1_monday: String }

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DCourse {
    id: String,
    code: String,
    #[serde(default)] name: String,
    #[serde(default)] dept: String,
    #[serde(default)] faculty: String,
    #[serde(default)] link: String,
    #[serde(default)] groups: Vec<DGroup>,
    #[serde(default)] tutorials: Vec<DStream>,
    #[serde(default)] labs: Vec<DStream>,
    #[serde(default)] field_trips: Vec<DStream>,
    #[serde(default)] other: Vec<DSession>,
    #[serde(default)] exams: Vec<DSession>,
}

#[derive(Deserialize)]
struct DGroup { id: String, #[serde(default)] sessions: Vec<DSession> }

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DStream {
    id: String,
    #[serde(default, rename = "type")] ty: String,
    #[serde(default)] parent_group: Option<String>,
    #[serde(default)] synthetic: bool,
    #[serde(default)] sessions: Vec<DSession>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DSession {
    day: u8,
    start: String,
    end: String,
    #[serde(default, rename = "type")] ty: String,
    #[serde(default)] raw_type: Option<String>,
    #[serde(default)] room: String,
    #[serde(default)] rooms: Vec<String>,
    #[serde(default)] weeks: Vec<u32>,
    #[serde(default)] staff: Vec<String>,
    #[serde(default)] cross_listed: Vec<String>,
}

fn hhmm(s: &str) -> f32 {
    let (h, m) = s.split_once(':').unwrap_or((s, "0"));
    h.trim().parse::<f32>().unwrap_or(0.0) + m.trim().parse::<f32>().unwrap_or(0.0) / 60.0
}

fn kind_of(ty: &str) -> SKind {
    match ty {
        "Lecture" => SKind::Lecture, "Tutorial" => SKind::Tutorial, "Lab" => SKind::Lab,
        "FieldTrip" => SKind::FieldTrip, "Seminar" => SKind::Seminar, _ => SKind::Other,
    }
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for b in bytes { *h ^= u64::from(*b); *h = h.wrapping_mul(0x100_0000_01b3); }
    *h ^= 0xff; *h = h.wrapping_mul(0x100_0000_01b3);
}

fn session(d: DSession, exam: bool) -> Session {
    let mut rooms = d.rooms;
    if rooms.is_empty() && !d.room.trim().is_empty() { rooms = d.room.split(';').map(|r| r.trim().to_owned()).filter(|r| !r.is_empty()).collect(); }
    let (start, end) = (hhmm(&d.start), hhmm(&d.end));
    let mut cross = d.cross_listed;
    cross.sort();
    let twin = if cross.len() > 1 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        fnv(&mut h, &[d.day]);
        fnv(&mut h, &((start * 60.0) as u32).to_le_bytes());
        fnv(&mut h, &((end * 60.0) as u32).to_le_bytes());
        for r in &rooms { fnv(&mut h, r.as_bytes()); }
        for c in &cross { fnv(&mut h, c.as_bytes()); }
        h | 1
    } else { 0 };
    let kind = if exam { SKind::Exam } else { kind_of(&d.ty) };
    Session {
        day: d.day.min(6), start, end: end.max(start + 0.25), kind, raw_type: d.raw_type.unwrap_or_default(),
        rooms, weeks: mask(d.weeks), staff: d.staff, cross, twin,
    }
}

const DAYS3: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// "Tue 09:00" → "Tue 9am"; anything else is left as it is.
fn time_label(id: &str) -> Option<String> {
    let (d, t) = id.split_once(' ')?;
    if !DAYS3.contains(&d) || !t.contains(':') { return None; }
    Some(format!("{d} {}", tm(hhmm(t))))
}

fn streams(list: Vec<DStream>) -> Vec<Stream> {
    let mut out: Vec<Stream> = list.into_iter().map(|d| {
        let kind = kind_of(&d.ty);
        let label = if d.synthetic {
            time_label(&d.id).unwrap_or_else(|| d.id.clone())
        } else {
            match &d.parent_group {
                Some(g) => d.id.strip_prefix(&format!("{g} ")).unwrap_or(&d.id).to_owned(),
                None => d.id.clone(),
            }
        };
        Stream {
            id: d.id, label, kind, parent_group: d.parent_group, synthetic: d.synthetic,
            sessions: d.sessions.into_iter().map(|s| session(s, false)).collect(),
        }
    }).collect();
    // Two time options at the same hour in different rooms need telling apart.
    for i in 0..out.len() {
        if out.iter().filter(|o| o.label == out[i].label && o.parent_group == out[i].parent_group).count() > 1 {
            if let Some(r) = out[i].sessions.first().map(Session::room_text).filter(|r| r != "Room TBA") {
                out[i].label = format!("{} · {r}", out[i].label);
            }
        }
    }
    for i in 0..out.len() {
        let n = out[..i].iter().filter(|o| o.label == out[i].label && o.parent_group == out[i].parent_group).count();
        if n > 0 { out[i].label = format!("{} #{}", out[i].label, n + 1); }
    }
    out
}

/// Odd codes arrive split across the two fields: code `PSYC 6013`, name `/7013, Advanced ...`. The
/// full code is `PSYC 6013/7013` and the name is what follows the comma.
pub fn fix_code_name(code: &str, name: &str) -> (String, String) {
    let n = name.trim_start();
    if !n.starts_with('/') { return (code.to_owned(), name.to_owned()); }
    let Some((suffix, rest)) = n.split_once(',') else { return (code.to_owned(), name.to_owned()) };
    let suffix: String = suffix.chars().filter(|c| !c.is_whitespace()).collect();
    if suffix.len() > 24 { return (code.to_owned(), name.to_owned()); }
    (format!("{code}{suffix}"), rest.trim().to_owned())
}

pub fn parse(text: &str) -> Result<Catalogue, String> {
    let doc: Doc = serde_json::from_str(text).map_err(|e| format!("celcat.json is not valid: {e}"))?;
    if doc.courses.is_empty() { return Err("celcat.json has no courses".into()); }
    let courses = doc.courses.into_iter().map(|c| { let (code, name) = fix_code_name(&c.code, &c.name); Course {
        id: c.id, code, name, dept: c.dept, faculty: if c.faculty.trim().is_empty() || c.faculty.contains("[None]") { "Other".to_owned() } else { c.faculty }, link: c.link,
        groups: c.groups.into_iter().map(|g| Group { id: g.id, sessions: g.sessions.into_iter().map(|s| session(s, false)).collect() }).collect(),
        tutorials: streams(c.tutorials), labs: streams(c.labs), field_trips: streams(c.field_trips),
        other: c.other.into_iter().map(|s| session(s, false)).collect(),
        exams: c.exams.into_iter().map(|s| session(s, true)).collect(),
    } }).collect();
    Ok(Catalogue {
        courses,
        meta: Meta {
            term: doc.term, term_label: doc.term_label, generated: doc.generated, published_at: doc.published_at,
            weeks_published: doc.weeks_published, week1_monday: doc.calendar.week1_monday, source: doc.source, sample: false,
        },
    })
}

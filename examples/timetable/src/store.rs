//! Persistence and sharing. Autosave goes to localStorage under `uwi-timetable:v1`; a share link
//! carries the schedule in the URL hash as `#s=<base64url of compact JSON>`.
//!
//! Share payload: `{"t": term, "c": [[id, code, group, tutorial, lab, field trip], ...]}`, where
//! the group and stream entries are ids ("" for none). Courses match by id, else by code.

use serde::{Deserialize, Serialize};

use crate::State;
use crate::data::Course;
use crate::host;
use crate::load::Status;
use crate::model::{Enr, free_tone};
use crate::solve::Prefs;

pub const KEY: &str = "uwi-timetable:v1";

/// One course in a saved schedule or share link.
#[derive(Serialize, Deserialize, Clone, Default, Debug, PartialEq, Eq)]
pub struct SavedCourse {
    pub id: String,
    pub code: String,
    #[serde(default)] pub g: String,
    #[serde(default)] pub t: String,
    #[serde(default)] pub l: String,
    #[serde(default)] pub f: String,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct Saved {
    pub v: u32,
    #[serde(default)] pub term: String,
    #[serde(default)] pub courses: Vec<SavedCourse>,
    /// free_day, no_early, no_late, compact.
    #[serde(default)] pub prefs: Option<[bool; 4]>,
    /// Theme override: None follows the system.
    #[serde(default)] pub dark: Option<bool>,
    /// Lights override: None follows the device.
    #[serde(default)] pub lights: Option<bool>,
    /// Calm mode if the student chose; absent follows the system's reduced-motion setting.
    #[serde(default)] pub calm_pref: Option<bool>,
    /// Course ids added lately, newest first.
    #[serde(default)] pub recent: Vec<String>,
}

/// A schedule from a share link, waiting for the user to accept it.
#[derive(Clone, Debug)]
pub struct Share { pub term: String, pub courses: Vec<SavedCourse> }

pub fn load_saved() -> Option<Saved> {
    serde_json::from_str::<Saved>(&host::storage_get(KEY)?).ok().filter(|s| s.v == 1)
}

fn saved_course(cat: &[Course], e: &Enr) -> SavedCourse {
    let c = &cat[e.course];
    let sid = |kind: u8| e.pick(kind).and_then(|i| c.streams(kind).get(i)).map_or(String::new(), |s| s.id.clone());
    SavedCourse { id: c.id.clone(), code: c.code.clone(), g: c.group_id(e.group).to_owned(), t: sid(1), l: sid(2), f: sid(3) }
}

pub fn snapshot(s: &State) -> Saved {
    let p = s.prefs;
    Saved {
        v: 1, term: s.meta.term.clone(),
        courses: s.enr.iter().filter(|e| e.course < s.cat.len()).map(|e| saved_course(&s.cat, e)).collect(),
        prefs: Some([p.free_day, p.no_early, p.no_late, p.compact]),
        dark: s.dark, lights: if s.lights_auto { None } else { Some(s.lights) }, calm_pref: s.calm, recent: s.recent.clone(),
    }
}

/// Writes the schedule when it changed. Only with the real catalogue: against the sample, the
/// saved schedule would be overwritten by an empty one.
pub fn autosave(s: &mut State) {
    if s.status != Status::Ready || !s.restored || s.viewing.is_some() { return; }
    let json = serde_json::to_string(&snapshot(s)).unwrap_or_default();
    if json != s.last_saved {
        host::storage_set(KEY, &json);
        s.last_saved = json;
    }
}

/// Theme, lights, calm and preferences apply at once; courses wait for the catalogue.
pub fn apply_settings(s: &mut State, saved: &Saved) {
    if let Some([a, b, c, d]) = saved.prefs { s.prefs = Prefs { free_day: a, no_early: b, no_late: c, compact: d }; }
    s.dark = saved.dark;
    if let Some(l) = saved.lights { s.lights = l; s.lights_auto = false; }
    s.calm = saved.calm_pref;
}

pub fn find_course(cat: &[Course], sc: &SavedCourse) -> Option<usize> {
    cat.iter().position(|c| c.id == sc.id).or_else(|| {
        let norm = |x: &str| x.to_lowercase().replace(' ', "");
        cat.iter().position(|c| norm(&c.code) == norm(&sc.code))
    })
}

fn pos(c: &Course, kind: u8, id: &str) -> Option<usize> { (!id.is_empty()).then(|| c.streams(kind).iter().position(|s| s.id == id)).flatten() }

/// Builds enrolments from saved courses; returns the codes that could not be found.
pub fn enrol(cat: &[Course], list: &[SavedCourse]) -> (Vec<Enr>, Vec<String>) {
    let mut out: Vec<Enr> = vec![];
    let mut missing = vec![];
    for sc in list {
        let Some(ci) = find_course(cat, sc) else { missing.push(sc.code.clone()); continue };
        if out.iter().any(|e| e.course == ci) { continue; }
        let c = &cat[ci];
        let group = c.groups.iter().position(|g| g.id == sc.g).unwrap_or(0);
        out.push(Enr {
            id: c.id.clone(), course: ci, tone: free_tone(&out), group,
            picks: [pos(c, 1, &sc.t), pos(c, 2, &sc.l), pos(c, 3, &sc.f)],
        });
    }
    (out, missing)
}

// ───────── Share ─────────

type Wire = (String, String, String, String, String, String);

#[derive(Serialize, Deserialize)]
struct ShareWire { t: String, c: Vec<Wire> }

pub fn share_hash(s: &State) -> String {
    let snap = snapshot(s);
    let wire = ShareWire {
        t: snap.term,
        c: snap.courses.into_iter().map(|c| (c.id, c.code, c.g, c.t, c.l, c.f)).collect(),
    };
    format!("s={}", b64_encode(serde_json::to_string(&wire).unwrap_or_default().as_bytes()))
}

pub fn share_url(s: &State) -> String { format!("{}#{}", host::page_url(), share_hash(s)) }

/// Parses a hash such as `s=...` (a leading `#` is fine).
pub fn parse_share(hash: &str) -> Option<Share> {
    let b = hash.trim_start_matches('#').strip_prefix("s=")?;
    let wire: ShareWire = serde_json::from_slice(&b64_decode(b)?).ok()?;
    let courses = wire.c.into_iter().map(|(id, code, g, t, l, f)| SavedCourse { id, code, g, t, l, f }).collect();
    Some(Share { term: wire.t, courses })
}

pub fn hash_share() -> Option<Share> { parse_share(&host::hash()) }

/// Replaces the schedule with the shared one; the codes that were not found go to `missing`.
pub fn apply_share(s: &mut State) {
    let Some(sh) = s.pending_share.take() else { return };
    replace_courses(s, &sh.courses);
    host::clear_hash();
}

/// Shows the shared schedule read-only, keeping the student's own aside until "Keep mine".
pub fn view_share(s: &mut State) {
    let Some(sh) = s.pending_share.take() else { return };
    let mine = std::mem::take(&mut s.enr);
    let (enr, missing) = enrol(&s.cat, &sh.courses);
    s.enr = enr;
    s.missing = missing;
    s.hover = None;
    s.focus = s.enr.first().map(|e| e.course);
    s.detail = None;
    crate::fill_defaults(s);
    s.viewing = Some(mine);
    // The hash stays, so a reload offers the same choice again.
}

/// Leaves the read-only view and puts the student's own schedule back.
pub fn keep_mine(s: &mut State) {
    if let Some(mine) = s.viewing.take() { s.enr = mine; }
    s.hover = None;
    s.detail = None;
    s.missing.clear();
    s.focus = s.enr.first().map(|e| e.course);
    host::clear_hash();
}

/// Makes the schedule being viewed the student's own.
pub fn adopt_viewed(s: &mut State) {
    s.viewing = None;
    host::clear_hash();
}

/// Whether a share link was made for a different term than the loaded data.
pub fn term_mismatch(s: &State, sh: &Share) -> bool { !sh.term.is_empty() && !s.meta.term.is_empty() && sh.term != s.meta.term }

pub fn dismiss_share(s: &mut State) {
    s.pending_share = None;
    host::clear_hash();
}

/// Swaps the whole schedule for `list`, filling any course without picks with the solver's.
pub fn replace_courses(s: &mut State, list: &[SavedCourse]) {
    let (enr, missing) = enrol(&s.cat, list);
    s.enr = enr;
    s.missing = missing;
    s.hover = None;
    s.focus = None;
    crate::fill_defaults(s);
}

// ───────── base64url ─────────

const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

pub fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 4 / 3 + 3);
    for ch in data.chunks(3) {
        let n = (u32::from(ch[0]) << 16) | (u32::from(*ch.get(1).unwrap_or(&0)) << 8) | u32::from(*ch.get(2).unwrap_or(&0));
        for i in 0..=ch.len() { out.push(ABC[(n >> (18 - 6 * i) & 63) as usize] as char); }
    }
    out
}

pub fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0);
    for c in s.trim_end_matches('=').bytes() {
        let v = ABC.iter().position(|&a| a == c)? as u32;
        acc = acc << 6 | v;
        bits += 6;
        if bits >= 8 { bits -= 8; out.push((acc >> bits) as u8); acc &= (1 << bits) - 1; }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn b64_round_trip() {
        for s in ["", "a", "ab", "abc", "{\"t\":\"Semester 1\",\"c\":[[\"1\",\"X 1\"]]}", "Tue 09:00 é"] {
            assert_eq!(b64_decode(&b64_encode(s.as_bytes())).as_deref(), Some(s.as_bytes()));
        }
    }

    #[test]
    fn share_round_trip() {
        let w = ShareWire { t: "T".into(), c: vec![("1".into(), "A 1".into(), "G1".into(), "Tue 09:00".into(), String::new(), String::new())] };
        let h = format!("s={}", b64_encode(serde_json::to_string(&w).unwrap().as_bytes()));
        let sh = parse_share(&h).unwrap();
        assert_eq!(sh.courses[0].t, "Tue 09:00");
        assert_eq!(sh.courses[0].g, "G1");
    }
}

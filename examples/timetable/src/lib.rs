//! UWI timetable builder, as a Rune showcase: ghost streams that fly into place when you lock
//! them in, a clash that shudders and sends a shockwave across the week, drag-to-reschedule,
//! a day zoom, a command palette, rolling stats, a brass seal and a theme that springs
//! between paper and night. It loads the published CELCAT timetable, solves for a clash-free
//! week, remembers your schedule, and shares and exports it.
//! Web: `scripts/web-build.sh timetable`. Native: `cargo run -p timetable`.

mod actions;
mod card;
mod cmdk;
mod data;
mod detail;
mod export;
#[cfg(feature = "fx")]
mod gfx;
mod grid;
mod header;
mod host;
mod load;
mod menus;
mod mobile;
mod model;
mod pal;
mod results;
mod search;
mod side;
mod skeleton;
mod solve;
mod store;
#[cfg(test)]
mod tests;
mod today;
mod weeks;

use std::time::Duration;

use data::{Course, Meta};
use load::{Loaded, Status, Timing};
use model::{Enr, active, clash_pairs, signature, week_mask};
use rune::{AppSpec, Cx, El, Key, KeyEvent, Live, TextEdit, Tx, Waker, col};
use solve::Prefs;
use store::{Saved, Share};

pub use actions::*;

/// Effect marks, shared by the view and the compositor.
pub mod mark {
    pub const SHOCK: u32 = 2;
    pub const SPARK: u32 = 3;
    pub const SHEEN: u32 = 4;
    pub const GRAIN: u32 = 5;
    pub const FIRE: u32 = 6;
    pub const ARC: u32 = 7;
    pub const GLOW: u32 = 8;
    pub const PULSE: u32 = 9;
    pub const AURORA: u32 = 10;
    pub const ZAP: u32 = 11;
    /// The lighting switch: data [on, dark].
    pub const LIGHTS: u32 = 12;
    /// Where the hover lamp shines: data [dragging].
    pub const LAMP: u32 = 13;
    /// Course blocks: lit slabs that cast shadows.
    pub const SLAB: u32 = 40;
    /// Sidebar cards and the palette: lower slabs.
    pub const CARD: u32 = 41;
}

/// A tutorial or lab block being dragged: where it started, the day-column width, the travel.
pub struct Drag { pub course: usize, pub kind: u8, pub stream: usize, pub day: u8, pub start: f32, pub cw: f32, pub dx: f32, pub dy: f32 }

pub struct Cmdk { pub open: bool, pub edit: TextEdit, pub sel: usize, /// Frames left to re-assert focus on the field (the modal takes it as it opens).
    pub want_focus: u8, /// The query the selection was last reset for.
    pub last_q: String }

/// A one-shot effect: `seq` tells the compositor it's new; it leaves the view at `until`.
#[derive(Clone)]
pub struct Shot { pub seq: u32, pub key: String, pub until: f64 }

/// A popover (desktop) or sheet (phone) opened from the header.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Menu { Prefs, Share, Term }

/// A short confirmation that fades by itself.
pub struct Toast { pub msg: String, pub until: f64 }

/// The last result worth keeping on screen until dismissed (the solver's answer).
pub struct Notice { pub msg: String, pub bad: bool }

/// The phone tab before the user picks one: Today once there is a schedule, else the week.
pub const AUTO_TAB: u8 = 255;

pub struct State {
    /// The loaded courses (empty while loading) and what is known about them.
    pub cat: Vec<Course>,
    pub meta: Meta,
    pub status: Status,
    pub timing: Timing,
    pub enr: Vec<Enr>,
    pub prefs: Prefs,
    /// Calm mode set by the user; `None` follows the system's reduced-motion setting.
    pub calm: Option<bool>,
    pub shown_calm: std::cell::Cell<bool>,
    /// Saved state read at startup; courses are restored once the catalogue is in.
    pub saved: Option<Saved>,
    /// Autosave is on once the saved schedule has been restored.
    pub restored: bool,
    pub last_saved: String,
    /// A schedule from a share link, waiting for "Replace my schedule" or "View only".
    pub pending_share: Option<Share>,
    /// While viewing a shared schedule read-only: the student's own, set aside.
    pub viewing: Option<Vec<Enr>>,
    /// Codes from a saved schedule or share link that this term's data doesn't have.
    pub missing: Vec<String>,
    pub notice: Option<Notice>,
    pub toast: Option<Toast>,
    /// Course ids added lately, newest first.
    pub recent: Vec<String>,
    /// The week the grid shows: `None` is all of them.
    pub week: Option<u32>,
    /// The open header popover, and the block (its session id) whose details are open.
    pub menu: Option<Menu>,
    pub detail: Option<String>,
    /// Cards folded to their header.
    pub collapsed: Vec<usize>,
    /// The faculty the palette and search are narrowed to.
    pub faculty: Option<String>,
    /// The course whose ghost streams show: the last one added, picked from, or hovered.
    pub focus: Option<usize>,
    pub search: TextEdit,
    /// A ghost stream under the pointer, or previewed from its chip: (course, kind 1|2, stream).
    pub hover: Option<(usize, u8, usize)>,
    /// `None` follows the system; the header switch pins it.
    pub dark: Option<bool>,
    /// rune-light on (shadows, sun, lamp, god rays). `L` or the lamp button flips it.
    pub lights: bool,
    /// The block or card under the pointer (its key), for the hover lamp.
    pub hot: Option<String>,
    /// Phone layout: the open tab (0 today, 1 week, 2 courses, or `AUTO_TAB`), the day being swiped and which way it went.
    pub tab: u8,
    pub swipe: f32,
    pub swipe_dir: i8,
    /// Lighting follows the device (on for desktop, off for phones) until the lamp button picks.
    pub lights_auto: bool,
    pub shown_lights: std::cell::Cell<bool>,
    pub zoom: Option<u8>,
    pub drag: Option<Drag>,
    pub cmdk: Cmdk,
    /// The clash shudder: kicked sideways, it rests back on a bouncy spring.
    pub shake: Live<[f32; 2]>,
    pub now: f64,
    /// Weekday (0 = Mon) and hour, for the "now" line; refreshed every 30 s.
    pub clock: Option<(u8, f32)>,
    /// The teaching week today falls in (from the inferred week-1 Monday), and today's date as text.
    pub cur_week: Option<i64>,
    pub date_text: String,
    next_clock: f64,
    last_clash: usize,
    pub seq: u32,
    pub shock: Option<Shot>,
    pub spark: Option<Shot>,
    pub sheen: Option<Shot>,
    /// The theme the last view drew, so the "T" key flips what you see.
    pub shown_dark: std::cell::Cell<bool>,
}

impl Default for State {
    fn default() -> Self {
        let mut s = State {
            cat: vec![],
            meta: Meta::default(),
            status: Status::Loading,
            timing: Timing::default(),
            enr: vec![],
            prefs: Prefs::default(),
            calm: None,
            shown_calm: std::cell::Cell::new(false),
            saved: store::load_saved(),
            restored: false,
            last_saved: String::new(),
            pending_share: store::hash_share(),
            viewing: None,
            missing: vec![],
            notice: None,
            toast: None,
            recent: vec![],
            week: None,
            menu: None,
            detail: None,
            collapsed: vec![],
            faculty: None,
            focus: None,
            search: TextEdit::new(""),
            hover: None,
            dark: None,
            lights: true,
            hot: None,
            tab: AUTO_TAB,
            swipe: 0.0,
            swipe_dir: 1,
            lights_auto: true,
            shown_lights: std::cell::Cell::new(true),
            zoom: None,
            drag: None,
            cmdk: Cmdk { open: false, edit: TextEdit::new(""), sel: 0, want_focus: 0, last_q: String::new() },
            shake: Live::new([0.0, 0.0]),
            now: 0.0,
            clock: None,
            cur_week: None,
            date_text: String::new(),
            next_clock: 0.0,
            last_clash: 0,
            seq: 0,
            shock: None,
            spark: None,
            sheen: None,
            shown_dark: std::cell::Cell::new(false),
        };
        if let Some(saved) = s.saved.clone() { store::apply_settings(&mut s, &saved); }
        s
    }
}

/// The phone tab in effect.
pub fn cur_tab(s: &State) -> u8 {
    if s.tab != AUTO_TAB { s.tab } else if s.enr.is_empty() { 1 } else { 0 }
}

/// Whether the effects are calmed: the user's choice, else the system's reduced-motion setting.
pub fn is_calm(s: &State, cx: &Cx<State>) -> bool { s.calm.unwrap_or_else(|| cx.system_reduced_motion()) }

/// A shared schedule is being looked at, so nothing may change.
pub fn locked(s: &State) -> bool { s.viewing.is_some() }

/// The course whose ghost streams show: one being dragged, else the last one touched.
pub fn focus_course(s: &State) -> Option<usize> {
    s.drag.as_ref().map(|d| d.course).or(s.focus).filter(|c| s.enr.iter().any(|e| e.course == *c))
}

/// The catalogue arrived (or failed): install it and restore the saved schedule.
fn on_loaded(s: &mut State, r: Loaded) {
    match r {
        Ok((cat, timing)) => {
            s.timing = timing;
            host::log(&format!("timetable: {} courses, fetch {:.0} ms, parse {:.0} ms", cat.courses.len(), timing.fetch_ms, timing.parse_ms));
            s.cat = cat.courses;
            s.meta = cat.meta;
            s.status = Status::Ready;
            if let Some(saved) = s.saved.clone() {
                let (enr, missing) = store::enrol(&s.cat, &saved.courses);
                s.enr = enr;
                s.missing = missing;
                s.recent = saved.recent.clone();
                if !saved.term.is_empty() && saved.term != s.meta.term && !saved.courses.is_empty() {
                    s.notice = Some(Notice { msg: "Your saved schedule was for another term; courses were matched by code.".to_owned(), bad: true });
                }
                fill_defaults(s);
            }
            s.restored = true;
            s.focus = s.enr.first().map(|e| e.course);
            refresh_clock(s);
        }
        Err(reason) => {
            host::log(&format!("timetable: using the sample data ({reason})"));
            let cat = data::sample();
            s.cat = cat.courses;
            s.meta = cat.meta;
            s.status = Status::Fallback(reason);
        }
    }
}

fn setup(_: &mut State, waker: Waker) { load::start(waker); }

pub fn is_dark(s: &State, cx: &Cx<State>) -> bool { s.dark.unwrap_or_else(|| cx.system_dark()) }

// ───────── Clock & triggers ─────────

fn wall_ms() -> i64 {
    #[cfg(target_arch = "wasm32")]
    { js_sys::Date::now() as i64 }
    #[cfg(not(target_arch = "wasm32"))]
    { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64) }
}

/// Local weekday (Mon = 0 … Sun = 6), the hour as a fraction, and the date.
fn read_clock() -> ((u8, f32), (i32, u32, u32)) {
    let t = rune::LocalTime::at(wall_ms());
    // Day of week from the local date (Sakamoto's method): 0 = Sunday.
    let (y, m, d) = (t.year, t.month as i32, t.day as i32);
    const OFF: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let yy = if m < 3 { y - 1 } else { y };
    let dow = (yy + yy / 4 - yy / 100 + yy / 400 + OFF[(m - 1) as usize] + d).rem_euclid(7);
    ((((dow + 6) % 7) as u8, t.hour as f32 + t.minute as f32 / 60.0), (t.year, t.month, t.day))
}

fn refresh_clock(s: &mut State) -> bool {
    let (c, (y, m, d)) = read_clock();
    let today = export::days_from_civil(i64::from(y), i64::from(m), i64::from(d));
    let week = (!s.meta.week1_monday.is_empty()).then(|| (today - export::week1_days(&s.meta.week1_monday)).div_euclid(7) + 1);
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let text = format!("{} {} {}", model::DAYS_LONG[c.0 as usize], d, MONTHS[(m as usize).clamp(1, 12) - 1]);
    let changed = s.clock != Some(c) || s.cur_week != week || s.date_text != text;
    s.clock = Some(c);
    s.cur_week = week;
    s.date_text = text;
    changed
}

/// Watches the schedule between frames: a new clash shudders and sends a shockwave, clearing the
/// last clash sweeps a sheen across the week, and one-shot effects leave on time.
fn poll(s: &mut State, now: f64, tx: &mut Tx) -> bool {
    s.now = now;
    let mut changed = false;
    if let Some(r) = load::take_inbox() { on_loaded(s, r); changed = true; }
    if s.cmdk.want_focus > 0 {
        s.cmdk.want_focus -= 1;
        if s.cmdk.open && !s.cmdk.edit.focused { s.cmdk.edit.focused = true; return true; }
    }
    if s.cmdk.edit.text != s.cmdk.last_q {
        s.cmdk.last_q = s.cmdk.edit.text.clone();
        s.cmdk.sel = 0;
        changed = true;
    }
    let sess = active(&s.cat, &s.enr);
    let pairs = clash_pairs(&sess, week_mask(s.week));
    let n = pairs.len();
    let calm = s.shown_calm.get();
    if n > s.last_clash {
        if !calm {
            let (a, _) = pairs[pairs.len() - 1];
            s.shake.pulse([9.0, 0.0], [0.0, 0.0], Duration::from_millis(50));
            s.seq += 1;
            s.shock = Some(Shot { seq: s.seq, key: sess[a].id.clone(), until: now + 1.3 });
        }
        changed = true;
    } else if n == 0 && s.last_clash > 0 {
        if !calm {
            s.seq += 1;
            s.sheen = Some(Shot { seq: s.seq, key: String::new(), until: now + 3.0 });
        }
        changed = true;
    }
    s.last_clash = n;
    for shot in [&mut s.shock, &mut s.spark, &mut s.sheen] {
        if shot.as_ref().is_some_and(|x| now >= x.until) { *shot = None; changed = true; }
    }
    if s.toast.as_ref().is_some_and(|t| now >= t.until) { s.toast = None; changed = true; }
    if now >= s.next_clock {
        changed |= refresh_clock(s);
        s.next_clock = now + 30.0;
    }
    store::autosave(s);
    let toast = s.toast.as_ref().map(|t| t.until);
    let next = [&s.shock, &s.spark, &s.sheen].iter().filter_map(|x| x.as_ref().map(|x| x.until)).chain(toast).fold(s.next_clock, f64::min);
    tx.wake_after(Duration::from_secs_f64((next - now).max(0.016)));
    changed
}

fn on_key(s: &mut State, ev: &KeyEvent, tx: &mut Tx) -> bool {
    if ev.is_command('k') {
        if s.cmdk.open { s.cmdk.open = false } else { cmdk::open(s) }
        return true;
    }
    // Typing in a field stays in the field; a focused button or block still gets shortcuts.
    if tx.focused().is_some_and(|f| f.ends_with("search") || f.ends_with("cmdk-input")) { return false; }
    match &ev.key {
        Key::Escape if s.detail.is_some() || s.menu.is_some() => { s.detail = None; s.menu = None; true }
        Key::Escape if s.zoom.is_some() => { s.zoom = None; true }
        Key::Text(t) if t == "/" => { cmdk::open(s); true }
        Key::Text(t) if t == "t" || t == "T" => { s.dark = Some(!s.shown_dark.get()); true }
        Key::Text(t) if t == "l" || t == "L" => { toggle_lights(s); true }
        Key::Text(t) if t == "c" || t == "C" => { toggle_calm(s); true }
        Key::Text(t) if matches!(t.as_str(), "1" | "2" | "3" | "4" | "5") => {
            let d = t.parse::<u8>().unwrap_or(1) - 1;
            s.zoom = if s.zoom == Some(d) { None } else { Some(d) };
            true
        }
        Key::Text(t) if (t == "6" || t == "7") && model::ndays() >= t.parse::<u8>().unwrap_or(9) => {
            let d = t.parse::<u8>().unwrap_or(6) - 1;
            s.zoom = if s.zoom == Some(d) { None } else { Some(d) };
            true
        }
        _ => false,
    }
}

// ───────── View ─────────

fn view(s: &State, cx: &mut Cx<State>) -> El {
    let p = pal::pal(is_dark(s, cx));
    s.shown_dark.set(p.dark);
    s.shown_calm.set(is_calm(s, cx));
    let small = mobile::is_compact(cx);
    grid::set_compact(small);
    // Lighting is a desktop default; phones start flat (fill-rate and battery) until you turn it on.
    let lit = if s.lights_auto { !small } else { s.lights };
    s.shown_lights.set(lit);
    let mut root = col().key("root").w_full().h_full().fill(p.bg)
        // paper grain under everything (drawn only with the fx feature)
        .child(rune::effect(mark::GRAIN).key("grain").effect_data([if p.dark { 0.05 } else { 0.07 }, 0.0, 0.0, 0.0])
            .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).inert())
        // the lighting switch the compositor reads
        .child(rune::effect(mark::LIGHTS).key("lights").effect_data([if lit { 1.0 } else { 0.0 }, if p.dark { 1.0 } else { 0.0 }, if small { 1.0 } else { 0.0 }, 0.0])
            .pin(Some(0.0), Some(0.0), None, None).size(1.0, 1.0).inert());
    if s.status == Status::Loading {
        return root.child(skeleton::body(s, cx, &p, small));
    }
    let sess = active(&s.cat, &s.enr);
    let wm = week_mask(s.week);
    let pairs = clash_pairs(&sess, wm);
    // The grid covers every enrolled session; the focused course's ghosts widen it only if they must.
    let gh = model::ghosts(&s.cat, &s.enr, &model::clash_set(&s.cat, &s.enr, wm), focus_course(s));
    model::set_range(&sess, &gh, signature(&s.enr));
    if small {
        root = mobile::body(root, s, cx, &p, &sess, &pairs);
    } else {
        let h = cx.layout_size("root").map_or(800.0, |x| x[1]);
        let extra = if pairs.is_empty() { 0.0 } else { 52.0 } + header::notice_count(s) as f32 * 40.0;
        grid::set_row_h(((h - 224.0 - extra) / (model::end() - model::start())).clamp(24.0, 46.0));
        root = root.child(header::header(s, &p, &sess, pairs.len()))
            .child(header::notices(s, &p, small))
            .child(rune::row().w_full().grow(1.0).min_h(0.0)
                .child(side::sidebar(s, &p))
                .child(grid::stage(s, cx, &p, &sess, &pairs)));
    }
    if let Some(o) = menus::overlay(s, cx, &p, &sess, small) { root = root.child(o); }
    if let Some(t) = menus::toast(s, cx, &p, small) { root = root.child(t); }
    if s.cmdk.open { root = root.child(cmdk::palette(s, &p, cx.layout_size("root").map_or(800.0, |x| x[1]))); }
    root
}

/// The lamp button and `L`: flip what's showing, and stop following the device default.
pub fn toggle_lights(s: &mut State) {
    s.lights = !s.shown_lights.get();
    s.lights_auto = false;
}

/// The calm button and `C`: flip what's showing, and stop following the system.
pub fn toggle_calm(s: &mut State) {
    s.calm = Some(!s.shown_calm.get());
    s.shock = None;
    s.sheen = None;
}

/// The day a phone shows: the one picked, else today, else Monday.
pub fn shown_day(s: &State) -> u8 { s.zoom.unwrap_or_else(|| s.clock.map_or(0, |c| if c.0 < 5 { c.0 } else { 0 })) }

/// A full-size absolutely-placed layer.
pub fn layer() -> El { el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)) }

use rune::el;

/// The hover lamp's slot: put inside the element it should shine on.
pub fn lamp(dragging: bool) -> El {
    rune::effect(mark::LAMP).key("lamp-slot").effect_data([if dragging { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0])
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).inert()
}

/// Tracks which block or card is under the pointer, for the lamp.
pub fn track_hot(key: String) -> impl Fn(&mut State, bool, &mut Tx) + 'static {
    move |s: &mut State, h: bool, _: &mut Tx| {
        if h {
            s.hot = Some(key.clone());
            if s.drag.is_none() { if let Some(c) = model::course_of_key(&key) { s.focus = Some(c); } }
        } else if s.hot.as_deref() == Some(key.as_str()) { s.hot = None }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(start))]
pub fn start() {
    let spec = AppSpec { setup: Some(setup), poll: Some(poll), on_key: Some(on_key), ..AppSpec::new("UWI Timetable", (1240.0, 800.0), State::default, view) };
    let app = rune::App::new(spec);
    #[cfg(feature = "fx")]
    let app = app.compositor(gfx::Gfx::default());
    app.run();
}

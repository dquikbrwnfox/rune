//! UWI timetable builder, as a Rune showcase: ghost streams that fly into place when you lock
//! them in, a clash that shudders and sends a shockwave across the week, drag-to-reschedule,
//! a day zoom, a command palette, rolling stats, a liquid-brass seal and a theme that springs
//! between paper and night.
//! Web: `scripts/web-build.sh timetable`. Native: `cargo run -p timetable`.

mod cmdk;
mod data;
#[cfg(feature = "fx")]
mod gfx;
mod grid;
mod header;
mod mobile;
mod model;
mod pal;
mod side;

use std::time::Duration;

use data::{Course, catalogue};
use model::{Enr, active, clash_pairs, free_tone};
use rune::{AppSpec, Cx, El, Key, KeyEvent, Live, TextEdit, Tx, col, el};


/// Effect marks, shared by the view and the compositor.
pub mod mark {
    pub const SEAL: u32 = 1;
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
    pub want_focus: u8 }

/// A one-shot effect: `seq` tells the compositor it's new; it leaves the view at `until`.
#[derive(Clone)]
pub struct Shot { pub seq: u32, pub key: String, pub until: f64 }

pub struct State {
    pub cat: Vec<Course>,
    pub enr: Vec<Enr>,
    pub search: TextEdit,
    /// A ghost stream under the pointer, or previewed from its chip: (course, kind 1|2, stream).
    pub hover: Option<(usize, u8, usize)>,
    /// `None` follows the system; the header switch pins it.
    pub dark: Option<bool>,
    /// rune-light on (shadows, sun, lamp, god rays). `L` or the lamp button flips it.
    pub lights: bool,
    /// The block or card under the pointer (its key), for the hover lamp.
    pub hot: Option<String>,
    /// Phone layout: the open tab (0 week, 1 courses), the day being swiped and which way it went.
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
    next_clock: f64,
    last_clash: usize,
    pub seq: u32,
    pub shock: Option<Shot>,
    pub spark: Option<Shot>,
    pub sheen: Option<Shot>,
    pub seal_gen: u32,
    /// The theme the last view drew, so the "T" key flips what you see.
    pub shown_dark: std::cell::Cell<bool>,
}

impl Default for State {
    fn default() -> Self {
        State {
            cat: catalogue(),
            enr: vec![],
            search: TextEdit::new(""),
            hover: None,
            dark: None,
            lights: true,
            hot: None,
            tab: 0,
            swipe: 0.0,
            swipe_dir: 1,
            lights_auto: true,
            shown_lights: std::cell::Cell::new(true),
            zoom: None,
            drag: None,
            cmdk: Cmdk { open: false, edit: TextEdit::new(""), sel: 0, want_focus: 0 },
            shake: Live::new([0.0, 0.0]),
            now: 0.0,
            clock: None,
            next_clock: 0.0,
            last_clash: 0,
            seq: 0,
            shock: None,
            spark: None,
            sheen: None,
            seal_gen: 0,
            shown_dark: std::cell::Cell::new(false),
        }
    }
}

// ───────── Actions ─────────

pub fn add(s: &mut State, course: usize) {
    if s.enr.iter().any(|e| e.course == course) { return; }
    let c = &s.cat[course];
    let (tut, lab) = ((!c.tutorials.is_empty()).then_some(0), (!c.labs.is_empty()).then_some(0));
    let tone = free_tone(&s.enr);
    s.enr.push(Enr { course, tone, tut, lab });
}

/// Lock in a tutorial or lab stream; sparkles burst on its first block.
pub fn pick(s: &mut State, course: usize, kind: u8, stream: usize) {
    let Some(e) = s.enr.iter_mut().find(|e| e.course == course) else { return };
    let cur = if kind == 1 { &mut e.tut } else { &mut e.lab };
    if *cur == Some(stream) { return; }
    *cur = Some(stream);
    s.hover = None;
    s.seq += 1;
    s.spark = Some(Shot { seq: s.seq, key: model::sid(course, kind, stream, 0), until: s.now + 1.4 });
}

pub fn remove(s: &mut State, course: usize) {
    s.enr.retain(|e| e.course != course);
    s.hover = None;
}

pub fn is_dark(s: &State, cx: &Cx<State>) -> bool { s.dark.unwrap_or_else(|| cx.system_dark()) }

// ───────── Clock & triggers ─────────

fn wall_ms() -> i64 {
    #[cfg(target_arch = "wasm32")]
    { js_sys::Date::now() as i64 }
    #[cfg(not(target_arch = "wasm32"))]
    { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64) }
}

fn read_clock() -> Option<(u8, f32)> {
    let ms = wall_ms();
    let t = rune::LocalTime::at(ms);
    // Day of week from the local date (Sakamoto's method): 0 = Sunday.
    let (y, m, d) = (t.year, t.month as i32, t.day as i32);
    const OFF: [i32; 12] = [0, 3, 2, 5, 0, 3, 5, 1, 4, 6, 2, 4];
    let yy = if m < 3 { y - 1 } else { y };
    let dow = (yy + yy / 4 - yy / 100 + yy / 400 + OFF[(m - 1) as usize] + d).rem_euclid(7);
    let weekday = (dow + 6) % 7; // Mon = 0
    (weekday < 5).then_some((weekday as u8, t.hour as f32 + t.minute as f32 / 60.0))
}

/// Watches the schedule between frames: a new clash shudders and sends a shockwave, clearing the
/// last clash sweeps a sheen across the week, and one-shot effects leave on time.
fn poll(s: &mut State, now: f64, tx: &mut Tx) -> bool {
    s.now = now;
    if s.cmdk.want_focus > 0 {
        s.cmdk.want_focus -= 1;
        if s.cmdk.open && !s.cmdk.edit.focused { s.cmdk.edit.focused = true; return true; }
    }
    let mut changed = false;
    let sess = active(&s.cat, &s.enr);
    let pairs = clash_pairs(&sess);
    let n = pairs.len();
    if n > s.last_clash {
        let (a, _) = pairs[pairs.len() - 1];
        s.shake.pulse([9.0, 0.0], [0.0, 0.0], Duration::from_millis(50));
        s.seq += 1;
        s.shock = Some(Shot { seq: s.seq, key: sess[a].id.clone(), until: now + 1.3 });
        changed = true;
    } else if n == 0 && s.last_clash > 0 {
        s.seq += 1;
        s.sheen = Some(Shot { seq: s.seq, key: String::new(), until: now + 3.0 });
        changed = true;
    }
    s.last_clash = n;
    for shot in [&mut s.shock, &mut s.spark, &mut s.sheen] {
        if shot.as_ref().is_some_and(|x| now >= x.until) { *shot = None; changed = true; }
    }
    if now >= s.next_clock {
        let c = read_clock();
        if c != s.clock { s.clock = c; changed = true; }
        s.next_clock = now + 30.0;
    }
    let next = [&s.shock, &s.spark, &s.sheen].iter().filter_map(|x| x.as_ref().map(|x| x.until)).fold(s.next_clock, f64::min);
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
        Key::Escape if s.zoom.is_some() => { s.zoom = None; true }
        Key::Text(t) if t == "/" => { cmdk::open(s); true }
        Key::Text(t) if t == "t" || t == "T" => { s.dark = Some(!s.shown_dark.get()); true }
        Key::Text(t) if t == "l" || t == "L" => { toggle_lights(s); true }
        Key::Text(t) if matches!(t.as_str(), "1" | "2" | "3" | "4" | "5") => {
            let d = t.parse::<u8>().unwrap_or(1) - 1;
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
    let sess = active(&s.cat, &s.enr);
    let pairs = clash_pairs(&sess);
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
    if small {
        root = mobile::body(root, s, cx, &p, &sess, &pairs);
    } else {
        grid::set_row_h(46.0);
        root = root.child(header::header(s, &p, &sess, pairs.len()))
            .child(rune::row().w_full().grow(1.0).min_h(0.0)
                .child(side::sidebar(s, &p))
                .child(grid::stage(s, cx, &p, &sess, &pairs)));
    }
    if s.cmdk.open { root = root.child(cmdk::palette(s, &p)); }
    root
}

/// The lamp button and `L`: flip what's showing, and stop following the device default.
pub fn toggle_lights(s: &mut State) {
    s.lights = !s.shown_lights.get();
    s.lights_auto = false;
}

/// The day a phone shows: the one picked, else today, else Monday.
pub fn shown_day(s: &State) -> u8 { s.zoom.unwrap_or_else(|| s.clock.map_or(0, |c| c.0)) }

/// A full-size absolutely-placed layer.
pub fn layer() -> El { el().pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)) }

/// The hover lamp's slot: put inside the element it should shine on.
pub fn lamp(dragging: bool) -> El {
    rune::effect(mark::LAMP).key("lamp-slot").effect_data([if dragging { 1.0 } else { 0.0 }, 0.0, 0.0, 0.0])
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).inert()
}

/// Tracks which block or card is under the pointer, for the lamp.
pub fn track_hot(key: String) -> impl Fn(&mut State, bool, &mut Tx) + 'static {
    move |s: &mut State, h: bool, _: &mut Tx| {
        if h { s.hot = Some(key.clone()) } else if s.hot.as_deref() == Some(key.as_str()) { s.hot = None }
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen(start))]
pub fn start() {
    let spec = AppSpec { poll: Some(poll), on_key: Some(on_key), ..AppSpec::new("UWI Timetable", (1240.0, 800.0), State::default, view) };
    let app = rune::App::new(spec);
    #[cfg(feature = "fx")]
    let app = app.compositor(gfx::Gfx::default());
    app.run();
}

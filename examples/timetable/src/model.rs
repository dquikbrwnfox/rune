//! Schedule logic: active sessions, ghosts, clashes, lane layout. No view code.
//!
//! Kinds used throughout: 0 lecture group, 1 tutorial, 2 lab, 3 field trip, 4 other (non-class).
//! `Enr` holds the chosen lecture group and one stream per kind. Only kinds 0..=3 take part in
//! clashes and the solver; `other` entries (help desks, review centres) are shown but never clash.

use std::cell::Cell;
use std::collections::HashSet;

use crate::data::{Course, SKind, Session};

pub const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
pub const DAYS_LONG: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

thread_local! {
    /// The hour range and day count the grid draws, set once per view from the data, and the
    /// schedule signature it was worked out for.
    static RANGE: Cell<(f32, f32, u8)> = const { Cell::new((8.0, 18.0, 5)) };
    static SIG: Cell<u64> = const { Cell::new(0) };
}
pub fn start() -> f32 { RANGE.with(|r| r.get().0) }
pub fn end() -> f32 { RANGE.with(|r| r.get().1) }
/// Visible weekdays: 5, 6 when Saturday is used, 7 when Sunday is.
pub fn ndays() -> u8 { RANGE.with(|r| r.get().2) }

/// Fits the grid to the sessions shown: earliest start to latest end, clamped to 7:00–22:00 and at
/// least eight hours tall; Saturday and Sunday only when something sits there. It covers every
/// enrolled session; the focused course's ghosts widen it only when they need to, and the range
/// never shrinks while the schedule (`sig`) is unchanged, so hovering from card to card can't make
/// the grid jump.
pub fn set_range(sess: &[Sess], ghosts: &[Ghost], sig: u64) {
    let all = || sess.iter().chain(ghosts.iter().map(|g| &g.sess));
    let (mut lo, mut hi, mut days) = (f32::MAX, 0.0f32, 5u8);
    for x in all() {
        lo = lo.min(x.start);
        hi = hi.max(x.end());
        days = days.max(x.day + 1);
    }
    if SIG.with(|s| s.replace(sig)) == sig {
        let (plo, phi, pd) = RANGE.with(Cell::get);
        if lo <= hi { lo = lo.min(plo); hi = hi.max(phi); days = days.max(pd); } else { (lo, hi, days) = (plo, phi, pd); }
    }
    if lo > hi { (lo, hi) = (8.0, 18.0); }
    let lo = lo.floor().clamp(7.0, 21.0);
    let mut hi = hi.ceil().clamp(lo + 1.0, 22.0);
    if hi - lo < 8.0 { hi = (lo + 8.0).min(22.0); }
    let lo = if hi - lo < 8.0 { (hi - 8.0).max(7.0) } else { lo };
    RANGE.with(|r| r.set((lo, hi, days)));
}

/// What a student has picked for one course. `course` indexes the loaded catalogue; `id` is the
/// stable CELCAT id that persistence and share links use.
#[derive(Clone)]
pub struct Enr {
    pub id: String,
    pub course: usize,
    pub tone: usize,
    /// Index into `Course::groups`.
    pub group: usize,
    /// Chosen stream per kind: [tutorial, lab, field trip].
    pub picks: [Option<usize>; 3],
}

impl Enr {
    pub fn pick(&self, kind: u8) -> Option<usize> { self.picks.get(usize::from(kind).wrapping_sub(1)).copied().flatten() }
    pub fn set_pick(&mut self, kind: u8, v: Option<usize>) { if let Some(p) = self.picks.get_mut(usize::from(kind).wrapping_sub(1)) { *p = v; } }
}

#[derive(Clone)]
pub struct Sess {
    pub course: usize,
    /// 0 lecture group, 1 tutorial, 2 lab, 3 field trip, 4 other.
    pub kind: u8,
    pub stream: usize,
    /// Position among the course's sessions, for the entrance cascade.
    pub ord: usize,
    pub day: u8,
    pub start: f32,
    pub hours: f32,
    pub room: String,
    pub id: String,
    pub weeks: u64,
    pub ty: SKind,
    pub raw: String,
    pub twin: u64,
}

impl Sess {
    pub fn end(&self) -> f32 { self.start + self.hours }
    /// The block's type tag: "LEC", "TUT", or the raw label for a non-class entry.
    pub fn tag(&self) -> String { if self.kind == 4 && !self.raw.is_empty() { self.raw.clone() } else { self.ty.tag().to_owned() } }
}

pub fn mk(course: usize, kind: u8, stream: usize, se: &Session, n: usize, ord: usize) -> Sess {
    Sess {
        course, kind, stream, ord, day: se.day, start: se.start, hours: se.hours(), room: se.room_text(),
        id: sid(course, kind, stream, n), weeks: se.weeks, ty: se.kind, raw: se.raw_type.clone(), twin: se.twin,
    }
}

/// Session id, shared by a ghost and the block it becomes (so `matched` flies one into the other).
pub fn sid(course: usize, kind: u8, stream: usize, n: usize) -> String { format!("s{course}-{kind}-{stream}-{n}") }

/// The course index inside a session or block key (`s{course}-...`) or card key (`card-{course}`).
pub fn course_of_key(key: &str) -> Option<usize> {
    let rest = key.strip_prefix("card-").or_else(|| key.strip_prefix('s'))?;
    rest.split('-').next()?.parse().ok()
}

pub fn active(cat: &[Course], enr: &[Enr]) -> Vec<Sess> {
    let mut out = vec![];
    for e in enr {
        let Some(c) = cat.get(e.course) else { continue };
        let mut ord = 0;
        if let Some(g) = c.groups.get(e.group) {
            for (n, se) in g.sessions.iter().enumerate() { out.push(mk(e.course, 0, e.group, se, n, ord)); ord += 1; }
        }
        for kind in 1..=3u8 {
            let Some(i) = e.pick(kind) else { continue };
            let Some(st) = c.streams(kind).get(i) else { continue };
            for (n, se) in st.sessions.iter().enumerate() { out.push(mk(e.course, kind, i, se, n, ord)); ord += 1; }
        }
        for (n, se) in c.other.iter().enumerate() { out.push(mk(e.course, 4, 0, se, n, ord)); ord += 1; }
    }
    out
}

/// Two sessions clash on the same day, at overlapping times, in at least one shared week. The two
/// copies of one cross-listed session (same time, room and listing) are one session, not a clash.
pub fn clashes(a: &Sess, b: &Sess) -> bool {
    a.kind != 4 && b.kind != 4 && a.day == b.day && a.start < b.end() && b.start < a.end()
        && a.weeks & b.weeks != 0 && !(a.twin != 0 && a.twin == b.twin && a.course != b.course)
}

/// Every clashing pair, by index into `sess`, counting only the weeks in `wmask` (all ones: every week).
pub fn clash_pairs(sess: &[Sess], wmask: u64) -> Vec<(usize, usize)> {
    let mut out = vec![];
    for i in 0..sess.len() {
        for j in i + 1..sess.len() {
            if clashes(&sess[i], &sess[j]) && sess[i].weeks & sess[j].weeks & wmask != 0 { out.push((i, j)); }
        }
    }
    out
}

/// The weeks two clashing sessions share, as text ("W10–12").
pub fn shared_weeks(a: &Sess, b: &Sess) -> String { crate::data::weeks_label(a.weeks & b.weeks) }

pub type ClashSet = HashSet<(usize, u8, usize)>;

/// Every (course, kind, stream) whose sessions would clash with everything else scheduled:
/// worked out once per view against one copy of the active schedule.
pub fn clash_set(cat: &[Course], enr: &[Enr], wmask: u64) -> ClashSet {
    let act = active(cat, enr);
    let mut out = HashSet::new();
    for e in enr {
        let Some(c) = cat.get(e.course) else { continue };
        for kind in 1..=3u8 {
            for (i, st) in c.streams(kind).iter().enumerate() {
                let bad = st.sessions.iter().enumerate().any(|(n, se)| {
                    let g = mk(e.course, kind, 0, se, n, 0);
                    act.iter().any(|o| !(o.course == e.course && o.kind == kind) && clashes(o, &g) && o.weeks & g.weeks & wmask != 0)
                });
                if bad { out.insert((e.course, kind, i)); }
            }
        }
    }
    out
}

pub struct Ghost { pub sess: Sess, pub clash: bool }

/// The unpicked streams of one course (those that fit its lecture group), projected onto the grid.
/// With real data a course can offer dozens of time options, so only the focused course shows them.
pub fn ghosts(cat: &[Course], enr: &[Enr], clashes: &ClashSet, focus: Option<usize>) -> Vec<Ghost> {
    let mut out = vec![];
    for e in enr.iter().filter(|e| Some(e.course) == focus) {
        let Some(c) = cat.get(e.course) else { continue };
        for kind in 1..=3u8 {
            for (i, st) in c.streams(kind).iter().enumerate() {
                if e.pick(kind) == Some(i) || !c.stream_ok(st, e.group) { continue; }
                let bad = clashes.contains(&(e.course, kind, i));
                for (n, se) in st.sessions.iter().enumerate() { out.push(Ghost { sess: mk(e.course, kind, i, se, n, 0), clash: bad }); }
            }
        }
    }
    out
}

/// The week filter as a mask: one week, or every week.
pub fn week_mask(week: Option<u32>) -> u64 { week.map_or(u64::MAX, |w| 1u64 << w.min(63)) }

/// Whether a session runs in the picked week (always, with none picked).
pub fn runs_in(x: &Sess, week: Option<u32>) -> bool { x.weeks & week_mask(week) != 0 }

/// Weekdays Mon–Fri with no class at all, as "Fri free" / "Mon, Fri free"; empty with nothing enrolled.
pub fn free_days_text(sess: &[Sess]) -> String {
    if sess.iter().all(|x| x.kind == 4) { return String::new(); }
    let free: Vec<&str> = (0..5u8).filter(|d| !sess.iter().any(|x| x.kind != 4 && x.day == *d)).map(|d| DAYS[d as usize]).collect();
    if free.is_empty() { "No free day".to_owned() } else { format!("{} free", free.join(", ")) }
}

/// A signature of the picks, to tell when the schedule (not the hover) changed.
pub fn signature(enr: &[Enr]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for e in enr {
        for v in [e.course, e.group, e.picks[0].map_or(0, |x| x + 1), e.picks[1].map_or(0, |x| x + 1), e.picks[2].map_or(0, |x| x + 1)] {
            h = (h ^ v as u64).wrapping_mul(0x100_0000_01b3);
        }
    }
    h
}

pub fn free_tone(enr: &[Enr]) -> usize { (0..8).find(|t| !enr.iter().any(|e| e.tone == *t)).unwrap_or(enr.len() % 8) }

pub fn tm(h: f32) -> String {
    let total = (h * 60.0).round() as i32;
    let (hh, mm) = (total.div_euclid(60), total.rem_euclid(60));
    let (h12, suffix) = (if hh % 12 == 0 { 12 } else { hh % 12 }, if hh >= 12 && hh < 24 { "pm" } else { "am" });
    if mm == 0 { format!("{h12}{suffix}") } else { format!("{h12}:{mm:02}{suffix}") }
}

pub struct Cluster { pub start: f32, pub end: f32, pub lanes: usize, pub items: Vec<(usize, usize)> } // (item, lane)

/// Groups overlapping spans and gives each a lane, so clashing blocks sit side by side.
pub fn clusters(items: &[(f32, f32)]) -> Vec<Cluster> {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&a, &b| items[a].0.total_cmp(&items[b].0).then(items[b].1.total_cmp(&items[a].1)));
    let mut out: Vec<Cluster> = vec![];
    let mut lane_ends: Vec<f32> = vec![];
    for i in order {
        let (st, en) = items[i];
        if out.last().is_none_or(|c| st >= c.end) {
            out.push(Cluster { start: st, end: en, lanes: 1, items: vec![] });
            lane_ends.clear();
        }
        let lane = lane_ends.iter().position(|&e| e <= st).unwrap_or(lane_ends.len());
        if lane == lane_ends.len() { lane_ends.push(en) } else { lane_ends[lane] = en }
        let Some(c) = out.last_mut() else { continue };
        c.end = c.end.max(en);
        c.lanes = c.lanes.max(lane + 1);
        c.items.push((i, lane));
    }
    out
}

/// "lecture", "tutorial", "lab", "field trip" or "entry" for a session kind (0..=4).
pub fn kind_name(k: u8) -> &'static str { ["lecture", "tutorial", "lab", "field trip", "entry"][usize::from(k).min(4)] }

/// "COMP 1601 lecture vs MATH 1115 tutorial (Mon 9am–10am, W10–12)": which two sessions meet where.
pub fn describe_pair(cat: &[Course], a: &Sess, b: &Sess) -> String {
    format!("{} {} vs {} {} ({} {}–{}, {})", cat[a.course].code, kind_name(a.kind), cat[b.course].code, kind_name(b.kind),
        DAYS[a.day as usize], tm(a.start.max(b.start)), tm(a.end().min(b.end())), shared_weeks(a, b))
}

/// The catalogue session behind a scheduled one (its position is the last part of its id).
pub fn session_of<'a>(cat: &'a [Course], x: &Sess) -> Option<&'a Session> {
    let n: usize = x.id.rsplit('-').next()?.parse().ok()?;
    let c = cat.get(x.course)?;
    match x.kind {
        0 => c.groups.get(x.stream)?.sessions.get(n),
        1..=3 => c.streams(x.kind).get(x.stream)?.sessions.get(n),
        _ => c.other.get(n),
    }
}

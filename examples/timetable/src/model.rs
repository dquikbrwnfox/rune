//! Schedule logic: active sessions, ghosts, clashes, lane layout. No view code.

use crate::data::{Course, Slot};

pub const START: f32 = 8.0;
pub const END: f32 = 21.0;
pub const DAYS: [&str; 5] = ["Mon", "Tue", "Wed", "Thu", "Fri"];
pub const DAYS_LONG: [&str; 5] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday"];

pub struct Enr {
    pub course: usize,
    pub tone: usize,
    pub tut: Option<usize>,
    pub lab: Option<usize>,
}

#[derive(Clone)]
pub struct Sess {
    pub course: usize,
    /// 0 lecture, 1 tutorial, 2 lab.
    pub kind: u8,
    pub stream: usize,
    /// Position among the course's sessions, for the entrance cascade.
    pub ord: usize,
    pub day: u8,
    pub start: f32,
    pub hours: f32,
    pub room: &'static str,
    pub id: String,
}

impl Sess {
    pub fn end(&self) -> f32 { self.start + self.hours }
}

pub fn mk(course: usize, kind: u8, stream: usize, sl: &Slot, n: usize, ord: usize) -> Sess {
    Sess { course, kind, stream, ord, day: sl.day, start: sl.start, hours: sl.hours, room: sl.room, id: sid(course, kind, stream, n) }
}

/// Session id, shared by a ghost and the block it becomes (so `matched` flies one into the other).
pub fn sid(course: usize, kind: u8, stream: usize, n: usize) -> String { format!("s{course}-{kind}-{stream}-{n}") }

pub fn streams(c: &Course, kind: u8) -> &[crate::data::Stream] { if kind == 1 { &c.tutorials } else { &c.labs } }

pub fn active(cat: &[Course], enr: &[Enr]) -> Vec<Sess> {
    let mut out = vec![];
    for e in enr {
        let c = &cat[e.course];
        let mut ord = 0;
        let mut push = |out: &mut Vec<Sess>, kind, stream, sl: &Slot, n| {
            out.push(mk(e.course, kind, stream, sl, n, ord));
            ord += 1;
        };
        for (n, sl) in c.lectures.iter().enumerate() { push(&mut out, 0, 0, sl, n); }
        if let Some(i) = e.tut { for (n, sl) in c.tutorials[i].slots.iter().enumerate() { push(&mut out, 1, i, sl, n); } }
        if let Some(i) = e.lab { for (n, sl) in c.labs[i].slots.iter().enumerate() { push(&mut out, 2, i, sl, n); } }
    }
    out
}

pub fn overlaps(a: &Sess, b: &Sess) -> bool { a.day == b.day && a.start < b.end() && b.start < a.end() }

/// Every overlapping pair, by index into `sess`.
pub fn clash_pairs(sess: &[Sess]) -> Vec<(usize, usize)> {
    let mut out = vec![];
    for i in 0..sess.len() {
        for j in i + 1..sess.len() {
            if overlaps(&sess[i], &sess[j]) { out.push((i, j)); }
        }
    }
    out
}

/// Every (course, kind, stream) whose sessions would clash with everything else scheduled:
/// worked out once per view against one copy of the active schedule.
pub fn clash_set(cat: &[Course], enr: &[Enr]) -> std::collections::HashSet<(usize, u8, usize)> {
    let act = active(cat, enr);
    let mut out = std::collections::HashSet::new();
    for e in enr {
        let c = &cat[e.course];
        for kind in [1u8, 2] {
            for (i, st) in streams(c, kind).iter().enumerate() {
                let bad = st.slots.iter().enumerate().any(|(n, sl)| {
                    let g = mk(e.course, kind, 0, sl, n, 0);
                    act.iter().any(|o| !(o.course == e.course && o.kind == kind) && overlaps(o, &g))
                });
                if bad { out.insert((e.course, kind, i)); }
            }
        }
    }
    out
}

pub struct Ghost { pub sess: Sess, pub clash: bool }

/// Every stream you haven't picked, projected onto the grid.
pub fn ghosts(cat: &[Course], enr: &[Enr], clashes: &std::collections::HashSet<(usize, u8, usize)>) -> Vec<Ghost> {
    let mut out = vec![];
    for e in enr {
        let c = &cat[e.course];
        for (kind, sel) in [(1u8, e.tut), (2u8, e.lab)] {
            for (i, st) in streams(c, kind).iter().enumerate() {
                if sel == Some(i) { continue; }
                let bad = clashes.contains(&(e.course, kind, i));
                for (n, sl) in st.slots.iter().enumerate() { out.push(Ghost { sess: mk(e.course, kind, i, sl, n, 0), clash: bad }); }
            }
        }
    }
    out
}

pub fn free_tone(enr: &[Enr]) -> usize { (0..8).find(|t| !enr.iter().any(|e| e.tone == *t)).unwrap_or(enr.len() % 8) }

pub fn tm(h: f32) -> String {
    let (hh, mm) = (h.floor() as i32, ((h - h.floor()) * 60.0).round() as i32);
    let (h12, suffix) = (if hh % 12 == 0 { 12 } else { hh % 12 }, if hh >= 12 { "pm" } else { "am" });
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

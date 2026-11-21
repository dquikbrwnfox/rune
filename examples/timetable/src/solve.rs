//! The solver: pick a lecture group and one stream per kind for every course so the week has the
//! fewest clashes, breaking ties by what the student prefers. Depth-first with branch and bound;
//! capped by node count and wall time, in which case the best found so far is returned.

use crate::data::Course;
use crate::load::now_ms;
use crate::model::{Enr, Sess, active, clashes, mk};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Prefs {
    /// Use as few weekdays as possible.
    pub free_day: bool,
    /// Avoid sessions starting before 9:00.
    pub no_early: bool,
    /// Avoid sessions ending after 18:00.
    pub no_late: bool,
    /// Fewer gaps between sessions in a day.
    pub compact: bool,
}

impl Default for Prefs {
    fn default() -> Self { Prefs { free_day: true, no_early: true, no_late: false, compact: false } }
}

/// One course's chosen group and streams.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice { pub course: usize, pub group: usize, pub picks: [Option<usize>; 3] }

pub struct Solution {
    pub choices: Vec<Choice>,
    pub clashes: usize,
    /// Hit the node or time cap: the answer is the best found, not proven best.
    pub capped: bool,
    pub nodes: u64,
    pub ms: f64,
}

const CLASH: i64 = 10_000;
const DAY: i64 = 300;
const EDGE: i64 = 100;
const GAP_HOUR: i64 = 40;
const CHANGE: i64 = 1;
const MAX_NODES: u64 = 400_000;
const MAX_MS: f64 = 40.0;

#[derive(Clone, Copy)]
struct P { day: u8, s: f32, e: f32, w: u64, c: usize, tw: u64 }

fn clash_p(a: &P, b: &P) -> bool {
    a.day == b.day && a.s < b.e && b.s < a.e && a.w & b.w != 0 && !(a.tw != 0 && a.tw == b.tw && a.c != b.c)
}

fn lite(x: &Sess) -> P { P { day: x.day, s: x.start, e: x.end(), w: x.weeks, c: x.course, tw: x.twin } }

struct Var { course: usize, kind: u8 }

/// One choice for a variable: a group (kind 0) or a stream, with its sessions precomputed.
struct Opt { i: usize, /** Lecture group this stream is tied to (`usize::MAX`: a group that does not exist). */ pg: Option<usize>, ps: Vec<P> }

fn build_options(cat: &[Course], course: usize) -> [Vec<Opt>; 4] {
    let c = &cat[course];
    let groups: Vec<Opt> = (0..c.groups.len().max(1)).map(|i| Opt {
        i, pg: None, ps: c.groups.get(i).map_or(vec![], |g| g.sessions.iter().map(|s| lite(&mk(course, 0, i, s, 0, 0))).collect()),
    }).collect();
    let kind = |k: u8| -> Vec<Opt> {
        c.streams(k).iter().enumerate().map(|(i, st)| Opt {
            i, pg: st.parent_group.as_ref().map(|p| c.groups.iter().position(|g| &g.id == p).unwrap_or(usize::MAX)),
            ps: st.sessions.iter().map(|s| lite(&mk(course, k, i, s, 0, 0))).collect(),
        }).collect()
    };
    [groups, kind(1), kind(2), kind(3)]
}

struct Search {
    prefs: Prefs,
    vars: Vec<Var>,
    /// Per free course (index = position in `free`): course index, current choice.
    free: Vec<Choice>,
    cache: std::rc::Rc<Vec<[Vec<Opt>; 4]>>,
    cur: Vec<Choice>,
    placed: Vec<P>,
    day_n: [u32; 7],
    mono: i64,
    best: i64,
    best_choice: Vec<Choice>,
    best_clashes: usize,
    clashes: usize,
    nodes: u64,
    capped: bool,
    t0: f64,
}

impl Search {
    /// Cost of adding `ps`: (clash count, monotone cost delta), without adding them.
    fn delta(&self, ps: &[P]) -> (usize, i64) {
        let mut n = 0;
        for p in ps { for q in &self.placed { if clash_p(p, q) { n += 1; } } }
        let mut cost = n as i64 * CLASH;
        let mut seen = [false; 7];
        for p in ps {
            if self.prefs.no_early && p.s < 9.0 { cost += EDGE; }
            if self.prefs.no_late && p.e > 18.0 { cost += EDGE; }
            let d = p.day as usize;
            if self.prefs.free_day && self.day_n[d] == 0 && !seen[d] { cost += DAY; }
            seen[d] = true;
        }
        (n, cost)
    }

    fn push(&mut self, ps: &[P]) {
        for p in ps { self.day_n[p.day as usize] += 1; }
        self.placed.extend_from_slice(ps);
    }
    fn pop(&mut self, ps: &[P]) {
        for p in ps { self.day_n[p.day as usize] -= 1; }
        self.placed.truncate(self.placed.len() - ps.len());
    }

    fn gaps(&self) -> i64 {
        if !self.prefs.compact { return 0; }
        let mut total = 0.0f32;
        for d in 0..7u8 {
            let mut iv: Vec<(f32, f32)> = self.placed.iter().filter(|p| p.day == d).map(|p| (p.s, p.e)).collect();
            iv.sort_by(|a, b| a.0.total_cmp(&b.0));
            let mut end = f32::MIN;
            for (s, e) in iv {
                if end > f32::MIN && s > end { total += s - end; }
                end = end.max(e);
            }
        }
        (total * GAP_HOUR as f32) as i64
    }

    fn dfs(&mut self, depth: usize) {
        self.nodes += 1;
        if self.nodes & 255 == 0 && (self.nodes > MAX_NODES || now_ms() - self.t0 > MAX_MS) { self.capped = true; }
        if self.capped { return; }
        if depth == self.vars.len() {
            let total = self.mono + self.gaps();
            if total < self.best { self.best = total; self.best_choice = self.cur.clone(); self.best_clashes = self.clashes; }
            return;
        }
        let (ci, kind) = (self.vars[depth].course, self.vars[depth].kind);
        let cache = std::rc::Rc::clone(&self.cache);
        let g = self.cur[ci].group;
        let cur = if kind == 0 { Some(self.free[ci].group) } else { self.free[ci].picks[kind as usize - 1] };
        let mut opts: Vec<(usize, usize, i64)> = cache[ci][kind as usize].iter().enumerate()
            .filter(|(_, o)| kind == 0 || o.pg.is_none_or(|p| p == g))
            .map(|(k, o)| {
                let (n, mut cost) = self.delta(&o.ps);
                if cur != Some(o.i) { cost += CHANGE; }
                (k, n, cost)
            }).collect();
        opts.sort_by_key(|o| o.2);
        if opts.is_empty() {
            if kind > 0 { self.cur[ci].picks[kind as usize - 1] = None; }
            self.dfs(depth + 1);
            return;
        }
        for (k, n, cost) in opts {
            let o = &cache[ci][kind as usize][k];
            let (i, ps) = (o.i, &o.ps);
            if self.mono + cost >= self.best { continue; }
            if kind == 0 { self.cur[ci].group = i } else { self.cur[ci].picks[kind as usize - 1] = Some(i) }
            self.mono += cost;
            self.clashes += n;
            self.push(ps);
            self.dfs(depth + 1);
            self.pop(ps);
            self.clashes -= n;
            self.mono -= cost;
            if self.capped { return; }
        }
    }
}

/// Best group and streams for the enrolled courses. With `only`, every other course keeps its
/// picks and the chosen one is fitted around them; otherwise all courses are re-picked.
pub fn solve(cat: &[Course], enr: &[Enr], prefs: Prefs, only: Option<&str>) -> Solution {
    let t0 = now_ms();
    let is_free = |e: &Enr| only.is_none_or(|id| e.id == id);
    let fixed: Vec<Enr> = enr.iter().filter(|e| !is_free(e)).cloned().collect();
    let free: Vec<Choice> = enr.iter().filter(|e| is_free(e) && e.course < cat.len())
        .map(|e| Choice { course: e.course, group: e.group, picks: e.picks }).collect();
    let mut s = Search {
        prefs, vars: vec![], cur: free.clone(), cache: std::rc::Rc::new(free.iter().map(|ch| build_options(cat, ch.course)).collect()), free, placed: vec![], day_n: [0; 7], mono: 0,
        best: i64::MAX, best_choice: vec![], best_clashes: 0, clashes: 0, nodes: 0, capped: false, t0,
    };
    // Courses with the fewest options go first; within a course, the group, then the smallest kinds.
    let mut order: Vec<usize> = (0..s.free.len()).collect();
    let size = |ci: usize| -> usize {
        let c = &cat[s.free[ci].course];
        (1..=3u8).map(|k| c.streams(k).len().max(1)).product::<usize>() * c.groups.len().max(1)
    };
    order.sort_by_key(|&ci| size(ci));
    for ci in order {
        let c = &cat[s.free[ci].course];
        if c.groups.len() > 1 { s.vars.push(Var { course: ci, kind: 0 }); }
        let mut kinds: Vec<u8> = (1..=3u8).filter(|&k| !c.streams(k).is_empty()).collect();
        kinds.sort_by_key(|&k| c.streams(k).len());
        for k in kinds { s.vars.push(Var { course: ci, kind: k }); }
    }
    // Fixed courses, and the single lecture group of free ones, are placed up front.
    let base: Vec<P> = active(cat, &fixed).iter().filter(|x| x.kind != 4).map(lite).collect();
    for p in &base { s.day_n[p.day as usize] += 1; }
    for i in 0..base.len() {
        for j in i + 1..base.len() { if clash_p(&base[i], &base[j]) { s.clashes += 1; s.mono += CLASH; } }
    }
    s.placed = base;
    for ci in 0..s.free.len() {
        let c = &cat[s.free[ci].course];
        if c.groups.len() <= 1 {
            s.cur[ci].group = 0;
            let cache = std::rc::Rc::clone(&s.cache);
            let ps = &cache[ci][0][0].ps;
            let (n, cost) = s.delta(ps);
            s.mono += cost; s.clashes += n;
            s.push(ps);
        }
    }
    s.dfs(0);
    if s.best == i64::MAX {
        // Capped before any leaf (cannot happen with the greedy first dive, but stay safe).
        s.best_choice = s.free.clone();
        s.best_clashes = 0;
    }
    // The count that matters is the real one, over the whole schedule.
    let mut all: Vec<Enr> = fixed;
    for ch in &s.best_choice {
        if let Some(e) = enr.iter().find(|e| e.course == ch.course) {
            let mut e = e.clone();
            e.group = ch.group;
            e.picks = ch.picks;
            all.push(e);
        }
    }
    let act = active(cat, &all);
    let mut n = 0;
    for i in 0..act.len() { for j in i + 1..act.len() { if clashes(&act[i], &act[j]) { n += 1; } } }
    Solution { choices: s.best_choice, clashes: n, capped: s.capped, nodes: s.nodes, ms: now_ms() - t0 }
}

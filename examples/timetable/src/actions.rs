//! Everything a press can do to the schedule. Each edit goes through here so that a shared
//! schedule being viewed read-only stays untouched.

use rune::Tx;

use crate::model::{self, Enr, active, clash_pairs, free_tone};
use crate::solve::solve;
use crate::{Notice, Shot, State, Toast, export, host, store};

/// Shows a short confirmation for a couple of seconds.
pub fn toast(s: &mut State, msg: impl Into<String>) { s.toast = Some(Toast { msg: msg.into(), until: s.now + 2.6 }); }

/// Adds a course with the solver's best streams around what is already picked.
pub fn add(s: &mut State, course: usize) {
    if crate::locked(s) || course >= s.cat.len() || s.enr.iter().any(|e| e.course == course) { return; }
    let c = &s.cat[course];
    let tone = free_tone(&s.enr);
    s.enr.push(Enr { id: c.id.clone(), course, tone, group: 0, picks: [None; 3] });
    let id = c.id.clone();
    s.recent.retain(|r| *r != id);
    s.recent.insert(0, id.clone());
    s.recent.truncate(8);
    let sol = solve(&s.cat, &s.enr, s.prefs, Some(&id));
    for ch in sol.choices {
        if let Some(e) = s.enr.iter_mut().find(|e| e.course == ch.course) { e.group = ch.group; e.picks = ch.picks; }
    }
    s.focus = Some(course);
}

/// Lock in a tutorial, lab or field-trip stream (kind 1, 2, 3); sparkles burst on its first block.
pub fn pick(s: &mut State, course: usize, kind: u8, stream: usize) {
    if crate::locked(s) { return; }
    let Some(e) = s.enr.iter_mut().find(|e| e.course == course) else { return };
    if e.pick(kind) == Some(stream) { return; }
    e.set_pick(kind, Some(stream));
    s.hover = None;
    s.focus = Some(course);
    if !s.shown_calm.get() {
        s.seq += 1;
        s.spark = Some(Shot { seq: s.seq, key: model::sid(course, kind, stream, 0), until: s.now + 1.4 });
    }
}

/// Switches lecture group; streams tied to another group move to the first one that fits.
pub fn set_group(s: &mut State, course: usize, group: usize) {
    if crate::locked(s) { return; }
    let Some(c) = s.cat.get(course) else { return };
    let Some(e) = s.enr.iter_mut().find(|e| e.course == course) else { return };
    if group >= c.groups.len() || e.group == group { return; }
    e.group = group;
    for kind in 1..=3u8 {
        let ok = e.pick(kind).and_then(|i| c.streams(kind).get(i)).is_some_and(|st| c.stream_ok(st, group));
        if !ok { e.set_pick(kind, c.streams(kind).iter().position(|st| c.stream_ok(st, group))); }
    }
    s.focus = Some(course);
}

pub fn remove(s: &mut State, course: usize) {
    if crate::locked(s) { return; }
    s.enr.retain(|e| e.course != course);
    s.collapsed.retain(|c| *c != course);
    s.hover = None;
    s.detail = None;
    if s.focus == Some(course) { s.focus = None; }
}

/// Gives every course a stream of each kind it has but none picked.
pub fn fill_defaults(s: &mut State) {
    let ids: Vec<String> = s.enr.iter().filter(|e| {
        let c = &s.cat[e.course];
        (1..=3u8).any(|k| !c.streams(k).is_empty() && e.pick(k).is_none())
    }).map(|e| e.id.clone()).collect();
    for id in ids {
        let sol = solve(&s.cat, &s.enr, s.prefs, Some(&id));
        for ch in sol.choices {
            if let Some(e) = s.enr.iter_mut().find(|e| e.course == ch.course) { e.group = ch.group; e.picks = ch.picks; }
        }
    }
}

/// Re-picks groups and streams for every course (or just `only`) to minimise clashes, applying the
/// changes through `set_group` and `pick` so the lock-in animations play.
pub fn solve_with(s: &mut State, only: Option<usize>) {
    if crate::locked(s) || s.enr.is_empty() { return; }
    let id = only.and_then(|c| s.enr.iter().find(|e| e.course == c)).map(|e| e.id.clone());
    if only.is_some() && id.is_none() { return; }
    let sol = solve(&s.cat, &s.enr, s.prefs, id.as_deref());
    let mut changed = 0;
    for ch in &sol.choices {
        let Some(e) = s.enr.iter().find(|e| e.course == ch.course) else { continue };
        if e.group != ch.group { set_group(s, ch.course, ch.group); changed += 1; }
        for kind in 1..=3u8 {
            let want = ch.picks[usize::from(kind) - 1];
            let have = s.enr.iter().find(|e| e.course == ch.course).and_then(|e| e.pick(kind));
            if let (Some(w), true) = (want, want != have) { pick(s, ch.course, kind, w); changed += 1; }
        }
    }
    host::log(&format!("timetable: solve {} nodes, {:.1} ms{}", sol.nodes, sol.ms, if sol.capped { ", capped" } else { "" }));
    s.menu = None;
    s.notice = Some(solve_notice(s, sol.clashes, changed));
}

/// "No clashes · 2 changes", or "Best found: 1 clash left — A vs B".
fn solve_notice(s: &State, clashes: usize, changed: usize) -> Notice {
    let s_ = |n: usize| if n == 1 { "" } else { "s" };
    if clashes == 0 {
        let msg = if changed == 0 { "No clashes · nothing to change".to_owned() } else { format!("No clashes · {changed} change{}", s_(changed)) };
        return Notice { msg, bad: false };
    }
    let sess = active(&s.cat, &s.enr);
    let pairs = clash_pairs(&sess, u64::MAX);
    let what = pairs.first().map_or(String::new(), |&(a, b)| format!(" — {}", model::describe_pair(&s.cat, &sess[a], &sess[b])));
    Notice { msg: format!("Best found: {clashes} clash{} left{what}", if clashes == 1 { "" } else { "es" }), bad: true }
}

pub fn solve_all(s: &mut State) { solve_with(s, None) }

pub fn fix_course(s: &mut State, course: usize) { solve_with(s, Some(course)) }

/// Copies the share link to the clipboard.
pub fn share(s: &mut State, tx: &mut Tx) {
    tx.copy(store::share_url(s));
    s.menu = None;
    toast(s, "Share link copied");
}

pub fn copy_summary(s: &mut State, tx: &mut Tx) {
    tx.copy(export::summary_text(s));
    s.menu = None;
    toast(s, "Schedule copied as text");
}

pub fn export_ics(s: &mut State) {
    export::download_ics(s);
    s.menu = None;
    toast(s, "Saved uwi-timetable.ics");
}

/// Picks the week the grid shows (`None`: all of them).
pub fn set_week(s: &mut State, week: Option<u32>) { s.week = week; }

/// A few well-known first-year courses (or the first three of whatever is loaded).
pub fn try_sample(s: &mut State) {
    let want = ["COMP 1601", "MATH 1141", "FOUN 1101", "INFO 1600"];
    let mut ids: Vec<usize> = want.iter().filter_map(|w| s.cat.iter().position(|c| c.code == *w)).take(3).collect();
    if ids.is_empty() { ids = (0..s.cat.len().min(3)).collect(); }
    for i in ids { add(s, i); }
    s.tab = 1;
}

/// Opens the details of a block (by its session id).
pub fn open_detail(s: &mut State, id: String) {
    s.menu = None;
    s.detail = Some(id);
}

pub fn toggle_collapsed(s: &mut State, course: usize) {
    if let Some(i) = s.collapsed.iter().position(|c| *c == course) { s.collapsed.remove(i); } else { s.collapsed.push(course); }
}

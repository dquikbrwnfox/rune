//! A course card: lecture-group control, tutorial / lab / field-trip option chips with human
//! labels, a clash summary with a one-tap Fix, the exam line, collapse and remove.

use rune::{El, Spring, Transition, col, el, row, text};

use crate::data::{Course, weeks_label};
use crate::model::{ClashSet, DAYS, Enr, Sess, shared_weeks, tm};
use crate::pal::{BODY, CODE, LABEL, SMALL, TINY, TITLE, Pal, tone};
use crate::{State, fix_course, pick, remove, set_group, toggle_collapsed};

/// "Exam: Wed 4pm–7pm · W11" for each exam of a course.
pub fn exam_text(c: &Course) -> Option<String> {
    if c.exams.is_empty() { return None; }
    let parts: Vec<String> = c.exams.iter()
        .map(|x| format!("{} {}–{} · {}", DAYS[x.day as usize], tm(x.start), tm(x.end), weeks_label(x.weeks)))
        .collect();
    Some(format!("Exam: {}", parts.join("; ")))
}

/// What this course's current picks clash with: one line naming the sessions, day, time and weeks.
fn clash_summary(s: &State, course: usize, sess: &[Sess], pairs: &[(usize, usize)]) -> Option<String> {
    let mine: Vec<&(usize, usize)> = pairs.iter().filter(|&&(a, b)| sess[a].course == course || sess[b].course == course).collect();
    let &&(a, b) = mine.first()?;
    let (me, other) = if sess[a].course == course { (&sess[a], &sess[b]) } else { (&sess[b], &sess[a]) };
    let more = if mine.len() > 1 { format!(" (+{} more)", mine.len() - 1) } else { String::new() };
    Some(format!("{} clashes with {} {}: {} {}–{}, {}{more}", me.tag(), s.cat[other.course].code, other.tag(),
        DAYS[me.day as usize], tm(me.start.max(other.start)), tm(me.end().min(other.end())), shared_weeks(me, other)))
}

/// "T1, LB2" for a collapsed card: the picks that matter.
fn picks_text(c: &Course, e: &Enr) -> String {
    let mut out: Vec<String> = vec![];
    if c.groups.len() > 1 { out.push(c.group_id(e.group).to_owned()); }
    for kind in 1..=3u8 {
        if let Some(st) = e.pick(kind).and_then(|i| c.streams(kind).get(i)) { out.push(st.label.clone()); }
    }
    if out.is_empty() { "Lectures only".to_owned() } else { out.join(" · ") }
}

/// Lecture groups as a segmented control (the caller checks there are two or more).
pub fn group_control(s: &State, p: &Pal, e: &Enr, small: bool) -> El {
    let c = &s.cat[e.course];
    let (course, tc) = (e.course, tone(e.tone, p.dark));
    let locked = crate::locked(s);
    let mut r = row().w_full().h(if small { 40.0 } else { 28.0 }).radius(9.0).border(1.0, tc.alpha(0.6)).clip_rounded(9.0);
    for (g, gr) in c.groups.iter().enumerate() {
        let on = e.group == g;
        let mut seg = el().key(format!("grp-{course}-{g}")).grow(1.0).h_full().center().motion(Spring::SNAPPY)
            .fill(if on { tc } else { tc.alpha(0.0) }).hover(if on { tc } else { p.hover })
            .child(text(gr.id.clone()).font(CODE).color(if on { p.bg } else { tc }));
        if !locked { seg = seg.press_scale(0.96).on_press(move |s: &mut State| set_group(s, course, g)); }
        r = r.child(seg);
    }
    col().gap(5.0).child(text("LECTURE GROUP").font(TINY).color(p.sub)).child(r)
}

/// The chips for one kind of option (1 tutorial, 2 lab, 3 field trip), plus a line saying what
/// the hovered (else the selected) one is: day, time, room, weeks.
pub fn option_chips(s: &State, p: &Pal, e: &Enr, kind: u8, label: &str, clashes: &ClashSet, small: bool) -> Option<El> {
    let c = &s.cat[e.course];
    let streams = c.streams(kind);
    if streams.is_empty() { return None; }
    let (course, tc) = (e.course, tone(e.tone, p.dark));
    let sel = e.pick(kind);
    let published = s.meta.published_mask();
    let locked = crate::locked(s);
    let multi = c.groups.len() > 1;
    let mut r = row().gap(6.0);
    for (i, st) in streams.iter().enumerate() {
        let on = sel == Some(i);
        let fits = c.stream_ok(st, e.group);
        let bad = fits && !on && clashes.contains(&(course, kind, i));
        let edge = if bad { p.danger } else { tc };
        let hot = s.hover == Some((course, kind, i));
        let mut chip = el().key(format!("chip-{course}-{kind}-{i}")).h(if small { 40.0 } else { 28.0 }).px(10.0).gap(5.0).center_y().radius(8.0)
            .press_scale(0.92).motion(Spring::SNAPPY).scale(if hot && !on { 1.06 } else { 1.0 });
        let reason = if !fits { format!("Only with lecture group {}", st.parent_group.clone().unwrap_or_default()) }
            else if bad { format!("Would clash. {}", st.detail_text(published)) } else { format!("{}: {}", st.chip_text(), st.detail_text(published)) };
        chip = chip.help(reason);
        if fits && !locked {
            chip = chip.on_hover_change(move |s: &mut State, h, _| {
                let cur = (course, kind, i);
                if h && !on { s.hover = Some(cur); s.focus = Some(course); } else if s.hover == Some(cur) { s.hover = None }
            }).on_press(move |s: &mut State| pick(s, course, kind, i));
        } else if !fits {
            chip = chip.opacity(0.35);
        }
        let ink = if on { p.bg } else { edge };
        chip = chip.child(text(st.chip_text()).font(CODE).color(ink));
        if st.partial(published) {
            let wk = st.sessions.iter().fold(0, |m, x| m | x.weeks);
            chip = chip.child(el().h(14.0).px(4.0).center().radius(4.0).fill(ink.alpha(0.2)).child(text(weeks_label(wk)).font(TINY).color(ink)));
        }
        if multi && st.parent_group.is_some() {
            chip = chip.child(text(st.parent_group.clone().unwrap_or_default()).font(TINY).color(ink.alpha(0.8)));
        }
        r = r.child(if on { chip.fill(edge) } else { chip.fill(edge.alpha(if hot { 0.16 } else { 0.0 })).border(1.0, edge).border_dash(4.0, 3.0) });
    }
    // Many options: a compact scrolling row on a phone, wrapped rows on the desktop.
    let strip = if small && streams.len() > 5 {
        r.scroll_x().h(44.0).w_full().scroll_edge_fade(10.0)
    } else { r.flex_wrap().w_full() };
    let (txt, note) = match s.hover.filter(|h| h.0 == course && h.1 == kind) {
        Some((_, _, i)) => (streams.get(i), "Preview"),
        None => (sel.and_then(|i| streams.get(i)), "Selected"),
    };
    let line = match txt {
        Some(st) => format!("{note}: {} · {}", st.chip_text(), st.detail_text(published)),
        None => "None picked yet".to_owned(),
    };
    Some(col().gap(5.0).child(text(label.to_owned()).font(TINY).color(p.sub)).child(strip)
        .child(text(line).font(SMALL).color(p.sub).w_full().wrap()))
}

pub fn course_card(s: &State, e: &Enr, p: &Pal, clashes: &ClashSet, sess: &[Sess], pairs: &[(usize, usize)]) -> El {
    let c = &s.cat[e.course];
    let course = e.course;
    let tc = tone(e.tone, p.dark);
    let small = crate::grid::compact();
    let locked = crate::locked(s);
    let folded = s.collapsed.contains(&course);
    let hours: f32 = sess.iter().filter(|x| x.course == course && x.kind != 4).map(|x| x.hours).sum::<f32>() + 0.0;
    let btn = if small { 40.0 } else { 28.0 };
    let summary = clash_summary(s, course, sess, pairs);

    let mut head = row().w_full().gap(8.0).center_y()
        .child(el().size(8.0, 8.0).radius(4.0).fill(tc))
        .child(col().grow(1.0).min_w(0.0)
            .child(row().gap(8.0).center_y().child(text(c.code.clone()).font(CODE).color(tc))
                .child(text(format!("{hours:.0}h / wk")).font(SMALL).color(p.sub)))
            .child(text(c.name.clone()).font(BODY).color(p.text).truncate()))
        .child(el().key(format!("fold-{course}")).size(btn, btn).center().radius(8.0).hover(p.hover).press_scale(0.88)
            .label(if folded { "Expand course" } else { "Collapse course" })
            .child(text(if folded { "+" } else { "–" }).font(TITLE).color(p.sub))
            .on_press(move |s: &mut State| toggle_collapsed(s, course)));
    if !locked {
        head = head.child(el().key(format!("rm-{course}")).size(btn, btn).center().radius(8.0).hover(p.hover).press_scale(0.88)
            .label("Remove course")
            .child(text("×").font(TITLE).color(p.sub))
            .on_press(move |s: &mut State| remove(s, course)));
    }
    let mut card = col().key(format!("card-{course}")).matched(format!("course-{course}")).mark(crate::mark::CARD).w_full().padding(12.0).gap(10.0).radius(12.0)
        .fill(p.card).border(1.0, if summary.is_some() { p.danger.alpha(0.7) } else { tc.alpha(0.45) }).shadow(1.0, 4.0, p.shadow.alpha(0.08))
        .transition(Transition::scale(0.94).combined(Transition::opacity()))
        .child(head);
    if folded {
        card = card.child(text(picks_text(c, e)).font(SMALL).color(p.sub).w_full().wrap());
        if let Some(msg) = &summary { card = card.child(text(msg.clone()).font(SMALL).color(p.danger).w_full().wrap()); }
    } else {
        if c.groups.len() > 1 { card = card.child(group_control(s, p, e, small)); }
        for (label, kind) in [("TUTORIAL", 1u8), ("LAB", 2), ("FIELD TRIP", 3)] {
            if let Some(r) = option_chips(s, p, e, kind, label, clashes, small) { card = card.child(r); }
        }
        if let Some(msg) = summary {
            let fix = el().key(format!("fix-{course}")).h(if small { 40.0 } else { 28.0 }).px(12.0).center().radius(8.0).fill(p.danger).press_scale(0.94)
                .child(text("Fix").font(LABEL).color(p.bg));
            card = card.child(row().key(format!("clash-{course}")).w_full().gap(8.0).center_y()
                .child(text(msg).font(SMALL).color(p.danger).grow(1.0).min_w(0.0).wrap())
                .child(if locked { el() } else { fix.on_press(move |s: &mut State| fix_course(s, course)) }));
        }
        if let Some(x) = exam_text(c) { card = card.child(text(x).font(SMALL).color(p.sub).w_full().wrap()); }
    }
    let key = format!("card-{course}");
    if s.drag.is_none() && s.hot.as_deref() == Some(key.as_str()) { card = card.child(crate::lamp(false)); }
    card.on_hover_change(crate::track_hot(key))
}

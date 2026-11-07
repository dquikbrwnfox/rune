//! The course list: search, cards with stream chips. A picked result flies into its card.

use rune::{El, ScrollSpec, Spring, Transition, col, el, row, text, text_input};

use crate::data::Stream;
use crate::model::{Enr, active, clash_set};
use crate::pal::{BODY, CODE, LABEL, SMALL, TINY, TITLE, Pal, tone};
use crate::{State, add, pick, remove};

pub fn sidebar(s: &State, p: &Pal) -> El {
    let q = s.search.text.to_lowercase().replace(' ', "");
    let search = text_input(&s.search, BODY).key("search")
        .placeholder("Search courses…", p.sub).color(p.text)
        .w_full().h(40.0).padding(12.0).radius(10.0).fill(p.raised).border(1.0, p.sep)
        .on_edit(|st: &mut State| &mut st.search);

    let mut list = col().key("side-list").w_full().gap(10.0).pb(16.0);
    if !q.is_empty() {
        let mut shown = 0;
        for (i, c) in s.cat.iter().enumerate() {
            let hay = format!("{}{}", c.code, c.name).to_lowercase().replace(' ', "");
            if !hay.contains(&q) || s.enr.iter().any(|e| e.course == i) { continue; }
            list = list.child(result_row(p, i, c.code, c.name).enter_delay(shown as f32 * 0.03));
            shown += 1;
        }
        if shown == 0 {
            list = list.child(text(format!("No course matches “{}”.", s.search.text)).key("nomatch").font(BODY).color(p.sub));
        }
    }
    if s.enr.is_empty() && q.is_empty() {
        list = list.child(col().key("empty").w_full().padding(16.0).gap(8.0).radius(12.0).border(1.0, p.sep).border_dash(5.0, 4.0)
            .transition(Transition::opacity())
            .child(text("Start by adding a course.").font(TITLE).color(p.text).w_full())
            .child(text("Lectures land straight away. Tutorials and labs show as dashed ghosts you lock in, by click or by dragging a block onto one.")
                .font(BODY).color(p.sub).w_full().wrap())
            .child(el().key("sample").h(32.0).px(12.0).center().radius(8.0).fill(p.brass).press_scale(0.96)
                .child(text("Try a sample first year").font(LABEL).color(p.bg))
                .on_press(|st: &mut State| for c in [0, 2, 4] { add(st, c) })));
    }
    let clashes = clash_set(&s.cat, &s.enr);
    for e in &s.enr { list = list.child(course_card(s, e, p, &clashes)); }

    col().key("side").w(320.0).h_full().padding(16.0).gap(12.0).fill(p.bg).border(1.0, p.sep)
        .child(search)
        .child(list.scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(12.0).grow(1.0).min_h(0.0))
}

/// One search result; `matched` lets it fly into the course card it becomes.
pub fn result_row(p: &Pal, i: usize, code: &'static str, name: &'static str) -> El {
    row().key(format!("res-{i}")).matched(format!("course-{i}")).w_full().h(46.0).px(12.0).gap(10.0).center_y()
        .radius(10.0).fill(p.raised).border(1.0, p.sep).hover(p.hover).press_scale(0.98)
        .transition(Transition::offset(0.0, 6.0).combined(Transition::opacity()))
        .child(el().size(22.0, 22.0).radius(6.0).center().fill(p.brass.alpha(0.18)).child(text("+").font(LABEL).color(p.brass)))
        .child(col().grow(1.0).min_w(0.0).child(text(code).font(CODE).color(p.text))
            .child(text(name).font(SMALL).color(p.sub).truncate()))
        .on_press(move |st: &mut State| { add(st, i); st.search.clear(); })
}

fn course_card(s: &State, e: &Enr, p: &Pal, clashes: &std::collections::HashSet<(usize, u8, usize)>) -> El {
    let c = &s.cat[e.course];
    let course = e.course;
    let tc = tone(e.tone, p.dark);
    let hours: f32 = active(&s.cat, std::slice::from_ref(e)).iter().map(|x| x.hours).sum();
    let chips = |label: &'static str, kind: u8, streams: &[Stream], sel: Option<usize>| -> Option<El> {
        if streams.is_empty() { return None; }
        let mut r = row().gap(6.0);
        for (i, st) in streams.iter().enumerate() {
            let on = sel == Some(i);
            let bad = clashes.contains(&(course, kind, i));
            let edge = if bad { p.danger } else { tc };
            let hot = s.hover == Some((course, kind, i));
            let chip = el().key(format!("chip-{course}-{kind}-{i}")).h(26.0).px(10.0).center().radius(7.0)
                .press_scale(0.92).motion(Spring::SNAPPY).scale(if hot && !on { 1.08 } else { 1.0 })
                .on_hover_change(move |s: &mut State, h, _| {
                    let cur = (course, kind, i);
                    if h && !on { s.hover = Some(cur) } else if s.hover == Some(cur) { s.hover = None }
                })
                .on_press(move |s: &mut State| pick(s, course, kind, i));
            let chip = if on { chip.fill(edge).child(text(st.id).font(CODE).color(p.bg)) }
                else { chip.fill(edge.alpha(if hot { 0.16 } else { 0.0 })).border(1.0, edge).border_dash(4.0, 3.0).child(text(st.id).font(CODE).color(edge)) };
            r = r.child(chip);
        }
        Some(col().gap(5.0).child(text(label).font(TINY).color(p.sub)).child(r))
    };
    let mut card = col().key(format!("card-{course}")).matched(format!("course-{course}")).mark(crate::mark::CARD).w_full().padding(12.0).gap(10.0).radius(12.0)
        .fill(p.card).border(1.0, tc.alpha(0.45)).shadow(1.0, 4.0, p.shadow.alpha(0.08))
        .transition(Transition::scale(0.94).combined(Transition::opacity()))
        .child(row().w_full().gap(10.0).center_y()
            .child(el().size(8.0, 8.0).radius(4.0).fill(tc))
            .child(col().grow(1.0).min_w(0.0)
                .child(row().gap(8.0).center_y().child(text(c.code).font(CODE).color(tc))
                    .child(text(format!("{hours:.0}h")).font(SMALL).color(p.sub)))
                .child(text(c.name).font(BODY).color(p.text).truncate()))
            .child(el().key(format!("rm-{course}")).size(26.0, 26.0).center().radius(7.0).hover(p.hover).press_scale(0.88)
                .child(text("×").font(LABEL).color(p.sub))
                .on_press(move |s: &mut State| remove(s, course))));
    if let Some(r) = chips("TUTORIAL", 1, &c.tutorials, e.tut) { card = card.child(r); }
    if let Some(r) = chips("LAB", 2, &c.labs, e.lab) { card = card.child(r); }
    let key = format!("card-{course}");
    if s.drag.is_none() && s.hot.as_deref() == Some(key.as_str()) { card = card.child(crate::lamp(false)); }
    card.on_hover_change(crate::track_hot(key))
}

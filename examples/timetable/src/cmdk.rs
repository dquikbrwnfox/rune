//! Ctrl/Cmd-K: a frosted command palette. Arrow keys move, Return adds, and the course flies
//! out of the palette into the sidebar as its card.

use rune::tok::TextStyle;
use rune::{El, Key, KeyEvent, Spring, Transition, Tx, col, el, row, text, text_input};

use crate::data::Course;
use crate::pal::{BODY, CODE, LABEL, SMALL, TINY, Pal};
use crate::{State, add, layer};

pub fn open(s: &mut State) {
    s.cmdk.open = true;
    s.cmdk.sel = 0;
    s.cmdk.edit.clear();
    s.cmdk.edit.focused = true;
    s.cmdk.want_focus = 3;
}

fn close(s: &mut State) {
    s.cmdk.open = false;
    s.cmdk.edit.focused = false;
}

fn matches(s: &State) -> Vec<usize> {
    let q = s.cmdk.edit.text.to_lowercase().replace(' ', "");
    s.cat.iter().enumerate()
        .filter(|(_, c)| q.is_empty() || format!("{}{}{}", c.code, c.name, dept(c)).to_lowercase().replace(' ', "").contains(&q))
        .map(|(i, _)| i).take(8).collect()
}

fn dept(c: &Course) -> &'static str {
    match &c.code[..4] { "COMP" | "INFO" => "Computing", "MATH" => "Mathematics", "FOUN" => "Foundation", "PHYS" => "Physics", "ECON" => "Economics", _ => "" }
}

fn choose(s: &mut State, i: usize) {
    s.tab = 0;
    add(s, i);
    close(s);
}

pub fn palette(s: &State, p: &Pal) -> El {
    let hits = matches(s);
    let sel = s.cmdk.sel.min(hits.len().saturating_sub(1));
    let input = text_input(&s.cmdk.edit, TextStyle::new(15.0, 450.0)).key("cmdk-input")
        .placeholder("Search by code, name or department…", p.sub).color(p.text)
        .w_full().h(52.0).px(16.0).py(16.0)
        .on_edit(|st: &mut State| &mut st.cmdk.edit)
        .on_input_key(|st: &mut State, ev: &KeyEvent, _: &mut Tx| {
            let n = matches(st).len();
            match ev.key {
                Key::Down if n > 0 => { st.cmdk.sel = (st.cmdk.sel + 1).min(n - 1); true }
                Key::Up => { st.cmdk.sel = st.cmdk.sel.saturating_sub(1); true }
                _ => false,
            }
        })
        .on_submit(|st: &mut State, _: &mut Tx| {
            let hits = matches(st);
            if let Some(&i) = hits.get(st.cmdk.sel.min(hits.len().saturating_sub(1))) { choose(st, i) }
        });

    let mut list = col().key("cmdk-list").w_full().padding(6.0).gap(2.0);
    for (k, &i) in hits.iter().enumerate() {
        let c = &s.cat[i];
        let added = s.enr.iter().any(|e| e.course == i);
        let on = k == sel;
        let mut week = row().gap(3.0);
        for d in 0..5u8 {
            let busy = c.lectures.iter().any(|l| l.day == d);
            week = week.child(el().size(6.0, 6.0).radius(3.0).fill(if busy { p.brass } else { p.sep }));
        }
        let hours: f32 = c.lectures.iter().map(|l| l.hours).sum();
        let mut r = row().key(format!("cmdk-{i}")).w_full().h(48.0).px(12.0).gap(12.0).center_y().radius(10.0)
            .fill(if on { p.brass.alpha(0.14) } else { p.raised.alpha(0.0) }).motion(Spring::SNAPPY)
            .on_hover_change(move |st: &mut State, h, _| if h { st.cmdk.sel = k })
            .transition(Transition::offset(0.0, 6.0).combined(Transition::opacity())).enter_delay(k as f32 * 0.025)
            .child(col().grow(1.0).min_w(0.0)
                .child(row().gap(8.0).center_y().child(text(c.code).font(CODE).color(if on { p.brass } else { p.text }))
                    .child(text(dept(c)).font(TINY).color(p.sub)))
                .child(text(c.name).font(SMALL).color(p.sub).truncate()))
            .child(week)
            .child(text(format!("{hours:.0}h lec")).font(SMALL).color(p.sub))
            .child(if added { text("Added").font(LABEL).color(p.sub) } else { el().h(22.0).px(8.0).center().radius(6.0).fill(if on { p.brass } else { p.hover })
                .child(text("Add ↵").font(LABEL).color(if on { p.bg } else { p.sub })) });
        if !added {
            r = r.matched(format!("course-{i}")).press_scale(0.98).on_press(move |st: &mut State| choose(st, i));
        }
        list = list.child(r);
    }
    if hits.is_empty() {
        list = list.child(el().key("cmdk-none").w_full().h(60.0).center().child(text("No courses match.").font(BODY).color(p.sub)));
    }

    let small = crate::grid::compact();
    let panel = col().key("cmdk-panel").mark(crate::mark::CARD);
    let panel = if small { panel.w_full() } else { panel.w(560.0) };
    let panel = panel.radius(16.0).fill(p.card.alpha(0.92)).border(1.0, p.sep)
        .shadow(24.0, 60.0, p.shadow).clip_rounded(16.0)
        .transition(Transition::scale(0.94).combined(Transition::opacity()).combined(Transition::offset(0.0, -12.0)))
        .child(input)
        .child(el().w_full().h(1.0).fill(p.sep))
        .child(list)
        .child(row().w_full().h(34.0).px(14.0).gap(14.0).center_y().fill(p.bg.alpha(0.5))
            .child(text("↑↓ move").font(SMALL).color(p.sub))
            .child(text("↵ add").font(SMALL).color(p.sub))
            .child(text("Esc close").font(SMALL).color(p.sub)));

    layer().key("cmdk").fill(p.scrim).backdrop_blur(14.0)
        .transition(Transition::opacity())
        .modal(true, close)
        .child(col().w_full().pt(if small { 64.0 } else { 110.0 }).px(if small { 10.0 } else { 0.0 }).items_center().child(panel))
}

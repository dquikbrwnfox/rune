//! Ctrl/Cmd-K: a frosted command palette. Arrow keys move (and scroll the list to follow),
//! Return adds, and the course flies out of the palette into the sidebar as its card. Empty, it
//! offers "Browse by faculty" and the recently added courses.

use rune::tok::TextStyle;
use rune::{El, Key, KeyEvent, Spring, Transition, Tx, col, el, row, text, text_input};

use crate::pal::{BODY, SMALL, Pal};
use crate::results::{browse_chips, browsing, heading, hits, result_row};
use crate::{State, add, layer};

pub fn open(s: &mut State) {
    s.cmdk.open = true;
    s.cmdk.sel = 0;
    s.cmdk.edit.clear();
    s.cmdk.last_q.clear();
    s.cmdk.edit.focused = true;
    s.cmdk.want_focus = 3;
}

fn close(s: &mut State) {
    s.cmdk.open = false;
    s.cmdk.edit.focused = false;
}

fn matches(s: &State) -> Vec<usize> { hits(s, &s.cmdk.edit.text) }

fn choose(s: &mut State, i: usize) {
    s.tab = 1;
    add(s, i);
    close(s);
}

pub fn palette(s: &State, p: &Pal, h: f32) -> El {
    let small = crate::grid::compact();
    let all = matches(s);
    let sel = s.cmdk.sel.min(all.len().saturating_sub(1));
    let q = s.cmdk.edit.text.clone();
    let input = text_input(&s.cmdk.edit, TextStyle::new(15.0, 450.0)).key("cmdk-input")
        .placeholder("Search by code, name or department…", p.sub).color(p.text)
        .w_full().h(52.0).px(16.0).py(16.0)
        .on_edit(|st: &mut State| &mut st.cmdk.edit)
        .on_input_key(|st: &mut State, ev: &KeyEvent, tx: &mut Tx| {
            let list = matches(st);
            let n = list.len();
            let go = |st: &mut State, tx: &mut Tx, to: usize| {
                st.cmdk.sel = to;
                if let Some(i) = list.get(to) { tx.scroll_into_view(format!("cmdk-{i}")); }
            };
            match ev.key {
                Key::Down if n > 0 => { go(st, tx, (st.cmdk.sel + 1).min(n - 1)); true }
                Key::Up => { go(st, tx, st.cmdk.sel.saturating_sub(1)); true }
                _ => false,
            }
        })
        .on_submit(|st: &mut State, _: &mut Tx| {
            let list = matches(st);
            if let Some(&i) = list.get(st.cmdk.sel.min(list.len().saturating_sub(1))) { choose(st, i) }
        });

    let mut list = col().key("cmdk-list").w_full().padding(6.0).gap(2.0).max_h(if small { (h - 52.0 - 34.0 - 72.0).max(200.0) } else { 440.0 })
        .scroll(rune::ScrollSpec { stick_bottom: false });
    if browsing(&q) {
        list = list.child(el().w_full().px(6.0).pb(6.0).child(browse_chips(s, p, small)));
        if s.faculty.is_none() && !all.is_empty() { list = list.child(el().px(6.0).child(heading(p, "RECENTLY ADDED"))); }
    }
    for (k, &i) in all.iter().enumerate() {
        let c = &s.cat[i];
        let added = s.enr.iter().any(|e| e.course == i);
        let on = k == sel;
        let mut r = result_row(p, format!("cmdk-{i}"), i, c, added, on, !small, small)
            .motion(Spring::SNAPPY)
            .on_hover_change(move |st: &mut State, h, _| if h { st.cmdk.sel = k })
            .enter_delay(k.min(8) as f32 * 0.025);
        if !added { r = r.on_press(move |st: &mut State| choose(st, i)); }
        list = list.child(r);
    }
    if all.is_empty() && !browsing(&q) {
        list = list.child(el().key("cmdk-none").w_full().h(60.0).center().child(text("No courses match.").font(BODY).color(p.sub)));
    } else if all.is_empty() && s.faculty.is_none() {
        list = list.child(el().key("cmdk-none").w_full().h(44.0).center().child(text("Type a course code or a name.").font(SMALL).color(p.sub)));
    }

    let panel = col().key("cmdk-panel").mark(crate::mark::CARD);
    let panel = if small { panel.w_full() } else { panel.w(580.0) };
    let panel = panel.radius(16.0).fill(p.card.alpha(0.94)).border(1.0, p.sep)
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
        .child(col().w_full().pt(if small { 56.0 } else { 90.0 }).px(if small { 10.0 } else { 0.0 }).items_center().child(panel))
}

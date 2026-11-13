//! The phone layout: a compact masthead, and three tabs: Today (what's next), Week (day pills
//! over one day at a time, with the week scrubber) and Courses (search, cards, solver, share).
//! Everything sits inside the safe area. The grid's hour height shrinks to fit the screen, so
//! the day never scrolls.

use rune::{Cx, El, Spring, Transition, col, el, row, text};

use crate::model::{DAYS, Sess, end, ndays, start};
use crate::pal::{BODY, H1, LABEL, SMALL, TINY, TITLE, Pal};
use crate::{State, cur_tab, grid, header, shown_day, side, today};

/// Phone-sized: under 720 pt wide. Before the first layout on the web, the browser's width decides.
pub fn is_compact(cx: &Cx<State>) -> bool {
    cx.layout_size("root").map_or_else(first_guess, |s| s[0] < 720.0)
}

fn first_guess() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Reflect::get(&js_sys::global(), &"innerWidth".into()).ok().and_then(|v| v.as_f64()).is_some_and(|w| w < 720.0)
    }
    #[cfg(not(target_arch = "wasm32"))]
    { false }
}

pub const HEADER: f32 = 62.0;
const PILLS: f32 = 44.0;
const TABS: f32 = 60.0;

pub fn body(root: El, s: &State, cx: &mut Cx<State>, p: &Pal, sess: &[Sess], pairs: &[(usize, usize)]) -> El {
    let [top, right, bottom, left] = cx.safe_area();
    let h = cx.layout_size("root").map_or(800.0, |x| x[1]);
    let tab = cur_tab(s);
    // Fit the day: everything but the hours is fixed, the hours share the rest.
    let banner = if pairs.is_empty() { 0.0 } else { 50.0 } + if s.enr.is_empty() && tab == 1 { 150.0 } else { 0.0 } + header::notice_count(s) as f32 * 76.0;
    let avail = h - top - bottom - HEADER - PILLS - TABS - 38.0 - 24.0 - 84.0 - banner;
    grid::set_row_h((avail / (end() - start())).clamp(26.0, 46.0));

    let content = match tab {
        2 => col().key("m-courses").w_full().grow(1.0).min_h(0.0).transition(Transition::opacity())
            .child(col().w_full().px(12.0).pt(8.0).gap(8.0)
                .child(header::actions(s, p, true))
                .child(row().gap(8.0).center_y().child(header::calm_button(s, p, true)).child(header::lamp_button(s, p))))
            .child(side::sidebar(s, p)),
        0 => col().key("m-today").w_full().grow(1.0).min_h(0.0).transition(Transition::opacity())
            .child(today::view(s, p, sess).grow(1.0).min_h(0.0)),
        _ => {
            let mut week = col().key("m-week").w_full().grow(1.0).min_h(0.0).transition(Transition::opacity())
                .child(pills(s, p, sess, pairs));
            if s.enr.is_empty() { week = week.child(empty(p)); }
            week.child(grid::stage(s, cx, p, sess, pairs))
        }
    };
    root.child(col().key("m-frame").w_full().h_full().pt(top).pr(right).pl(left)
        .child(masthead(s, p, sess, pairs.len()))
        .child(header::notices(s, p, true))
        .child(content)
        .child(tabbar(s, p, pairs.len(), bottom)))
}

fn masthead(s: &State, p: &Pal, sess: &[Sess], n_clash: usize) -> El {
    let hours: f32 = sess.iter().filter(|x| x.kind != 4).map(|x| x.hours).sum::<f32>() + 0.0;
    // The stats make way for the clash badge when there is one, so nothing runs under the buttons.
    let mut title = row().gap(8.0).center_y().child(text("Timetable").font(H1).color(p.text));
    title = if n_clash > 0 {
        title.child(el().key("m-clash").h(16.0).px(5.0).center().radius(4.0).fill(p.danger).transition(Transition::scale(0.6).combined(Transition::opacity()))
            .child(text(format!("{n_clash} CLASH{}", if n_clash > 1 { "ES" } else { "" })).font(TINY).color(p.bg)))
    } else {
        title.child(text(format!("{} course{} · {hours:.0} h", s.enr.len(), if s.enr.len() == 1 { "" } else { "s" })).font(SMALL).color(p.sub))
    };
    row().key("m-head").w_full().h(HEADER).px(14.0).gap(10.0).center_y().fill(p.card).border(1.0, p.sep)
        .child(header::seal(s, p))
        .child(col().grow(1.0).min_w(0.0).gap(2.0).child(title).child(row().child(header::term_chip(s, p, true))))
        .child(header::search_button(p))
        .child(header::theme_switch(s, p))
}

/// Mon–Fri: the picked one is ink, today is brass, a red dot marks a day with a clash.
fn pills(s: &State, p: &Pal, sess: &[Sess], pairs: &[(usize, usize)]) -> El {
    let day = shown_day(s);
    let mut r = row().key("m-pills").w_full().h(PILLS).px(10.0).gap(6.0).center_y();
    for d in 0..ndays() {
        let on = d == day;
        let today = s.clock.is_some_and(|c| c.0 == d);
        let clash = pairs.iter().any(|&(a, _)| sess[a].day == d);
        let ink = if on { p.bg } else if today { p.brass } else { p.text };
        let mut pill = el().key(format!("pill-{d}")).grow(1.0).h(38.0).center().radius(19.0)
            .fill(if on { p.text } else { p.raised }).border(1.0, if on { p.text } else { p.sep })
            .motion(Spring::SNAPPY).press_scale(0.92)
            .on_press(move |s: &mut State| {
                let cur = shown_day(s);
                s.swipe_dir = if d >= cur { 1 } else { -1 };
                s.zoom = Some(d);
            })
            .child(text(DAYS[d as usize]).font(LABEL).color(ink));
        if clash {
            pill = pill.child(el().key("dot").size(6.0, 6.0).radius(3.0).fill(p.danger).absolute(6.0, 5.0).transition(Transition::scale(0.2)));
        }
        r = r.child(pill);
    }
    r
}

fn empty(p: &Pal) -> El {
    col().key("m-empty").w_full().px(12.0).pt(4.0).child(
        col().w_full().padding(14.0).gap(8.0).radius(12.0).border(1.0, p.sep).border_dash(5.0, 4.0)
            .child(text("No courses yet.").font(TITLE).color(p.text))
            .child(text("Tap a sample first year to see it, or browse every course.").font(BODY).color(p.sub).w_full().wrap())
            .child(row().gap(8.0)
                .child(el().key("m-sample").h(44.0).px(14.0).center().radius(10.0).fill(p.brass).press_scale(0.95)
                    .child(text("Try a sample").font(LABEL).color(p.bg))
                    .on_press(|s: &mut State| crate::try_sample(s)))
                .child(el().key("m-browse").h(44.0).px(14.0).center().radius(10.0).fill(p.raised).border(1.0, p.sep).press_scale(0.95)
                    .child(text("Browse courses").font(LABEL).color(p.text))
                    .on_press(|s: &mut State| s.tab = 2))))
}

/// Today, Week and Courses, with a brass bar that springs between them.
fn tabbar(s: &State, p: &Pal, n_clash: usize, bottom: f32) -> El {
    let cur = cur_tab(s);
    let tab = |i: u8, label: &'static str, badge: Option<usize>| {
        let on = cur == i;
        let mut t = col().key(format!("tab-{i}")).grow(1.0).h(TABS - 10.0).center().gap(4.0).radius(12.0)
            .fill(if on { p.brass.alpha(0.14) } else { p.bg.alpha(0.0) }).motion(Spring::SNAPPY).press_scale(0.94)
            .on_press(move |s: &mut State| s.tab = i)
            .child(row().gap(6.0).center_y()
                .child(text(label).font(TITLE).color(if on { p.brass } else { p.sub }))
                .child(match badge {
                    Some(n) if n > 0 => el().key("badge").h(16.0).px(5.0).center().radius(8.0).fill(p.danger).transition(Transition::scale(0.5))
                        .child(text(n.to_string()).font(TINY).color(p.bg)),
                    _ => el(),
                }));
        t = t.child(el().key("bar").w(if on { 28.0 } else { 0.0 }).h(3.0).radius(1.5).fill(p.brass).motion(Spring::BOUNCY));
        t
    };
    row().key("m-tabs").w_full().h(TABS + bottom).pb(bottom).px(12.0).gap(8.0).center_y().fill(p.card).border(1.0, p.sep)
        .child(tab(0, "Today", None))
        .child(tab(1, "Week", Some(n_clash)))
        .child(tab(2, "Courses", None))
}

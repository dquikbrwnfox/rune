//! The phone layout: a compact masthead, day pills over one day at a time (swipe to page), and a
//! tab bar between the week and the course list. Everything sits inside the safe area. The grid's
//! hour height shrinks to fit the screen, so the day never scrolls.

use rune::{Cx, El, Spring, Transition, col, el, row, text};

use crate::model::{DAYS, END, START, Sess};
use crate::pal::{BODY, H1, LABEL, SMALL, TINY, TITLE, Pal};
use crate::{State, add, grid, header, side, shown_day};

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

const HEADER: f32 = 56.0;
const PILLS: f32 = 44.0;
const TABS: f32 = 58.0;

pub fn body(root: El, s: &State, cx: &mut Cx<State>, p: &Pal, sess: &[Sess], pairs: &[(usize, usize)]) -> El {
    let [top, right, bottom, left] = cx.safe_area();
    let h = cx.layout_size("root").map_or(800.0, |x| x[1]);
    // Fit the day: everything but the hours is fixed, the hours share the rest.
    let banner = if pairs.is_empty() { 0.0 } else { 62.0 } + if s.enr.is_empty() && s.tab == 0 { 150.0 } else { 0.0 };
    let avail = h - top - bottom - HEADER - PILLS - TABS - 38.0 - 24.0 - banner;
    grid::set_row_h((avail / (END - START)).clamp(26.0, 46.0));

    let content = if s.tab == 1 {
        col().key("m-courses").w_full().grow(1.0).min_h(0.0).transition(Transition::opacity()).child(side::sidebar(s, p))
    } else {
        let mut week = col().key("m-week").w_full().grow(1.0).min_h(0.0).transition(Transition::opacity())
            .child(pills(s, p, sess, pairs));
        if s.enr.is_empty() { week = week.child(empty(p)); }
        week.child(grid::stage(s, cx, p, sess, pairs))
    };
    root.child(col().key("m-frame").w_full().h_full().pt(top).pr(right).pl(left)
        .child(masthead(s, p, sess, pairs.len()))
        .child(content)
        .child(tabbar(s, p, pairs.len(), bottom)))
}

fn masthead(s: &State, p: &Pal, sess: &[Sess], n_clash: usize) -> El {
    let hours: f32 = sess.iter().map(|x| x.hours).sum::<f32>() + 0.0;
    let mut stats = row().gap(6.0).center_y()
        .child(text(format!("{} courses · {hours:.0} h", s.enr.len())).font(SMALL).color(p.sub));
    if n_clash > 0 {
        stats = stats.child(el().key("m-clash").h(16.0).px(5.0).center().radius(4.0).fill(p.danger).transition(Transition::scale(0.6).combined(Transition::opacity()))
            .child(text(format!("{n_clash} CLASH{}", if n_clash > 1 { "ES" } else { "" })).font(TINY).color(p.bg)));
    }
    row().key("m-head").w_full().h(HEADER).px(14.0).gap(12.0).center_y().fill(p.card).border(1.0, p.sep)
        .child(header::seal(s, p))
        .child(col().grow(1.0).min_w(0.0).gap(1.0).child(text("Timetable").font(H1).color(p.text)).child(stats))
        .child(search_icon(p))
        .child(header::lamp_button(s, p))
        .child(header::theme_switch(s, p))
}

/// A magnifier drawn from two shapes, so it needs no icon font.
fn search_icon(p: &Pal) -> El {
    el().key("m-search").size(34.0, 34.0).radius(17.0).fill(p.raised).border(1.0, p.sep).press_scale(0.9).hover(p.hover)
        .on_press(|s: &mut State| crate::cmdk::open(s))
        .label("Find a course")
        .child(el().size(12.0, 12.0).radius(6.0).border(2.0, p.sub).absolute(9.0, 9.0))
        .child(el().size(2.0, 7.0).radius(1.0).fill(p.sub).absolute(21.0, 19.0).rotate(-0.8))
}

/// Mon–Fri: the picked one is ink, today is brass, a red dot marks a day with a clash.
fn pills(s: &State, p: &Pal, sess: &[Sess], pairs: &[(usize, usize)]) -> El {
    let day = shown_day(s);
    let mut r = row().key("m-pills").w_full().h(PILLS).px(10.0).gap(6.0).center_y();
    for d in 0..5u8 {
        let on = d == day;
        let today = s.clock.is_some_and(|c| c.0 == d);
        let clash = pairs.iter().any(|&(a, _)| sess[a].day == d);
        let ink = if on { p.bg } else if today { p.brass } else { p.text };
        let mut pill = el().key(format!("pill-{d}")).grow(1.0).h(32.0).center().radius(16.0)
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
                .child(el().key("m-sample").h(34.0).px(12.0).center().radius(8.0).fill(p.brass).press_scale(0.95)
                    .child(text("Try a sample").font(LABEL).color(p.bg))
                    .on_press(|s: &mut State| for c in [0, 2, 4] { add(s, c) }))
                .child(el().key("m-browse").h(34.0).px(12.0).center().radius(8.0).fill(p.raised).border(1.0, p.sep).press_scale(0.95)
                    .child(text("Browse courses").font(LABEL).color(p.text))
                    .on_press(|s: &mut State| s.tab = 1))))
}

/// Week and Courses, with a brass bar that springs between them.
fn tabbar(s: &State, p: &Pal, n_clash: usize, bottom: f32) -> El {
    let tab = |i: u8, label: &'static str, badge: Option<usize>| {
        let on = s.tab == i;
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
    row().key("m-tabs").w_full().h(TABS + bottom).pb(bottom).px(12.0).gap(10.0).center_y().fill(p.card).border(1.0, p.sep)
        .child(tab(0, "Week", Some(n_clash)))
        .child(tab(1, "Courses", None))
}

//! The loading state: the masthead and an empty grid in the app's own paper and brass, with the
//! placeholders breathing, until `celcat.json` is in. Then the real thing fades in.

use rune::{Cx, El, LoopSpec, col, el, row, text};

use crate::header;
use crate::mobile::HEADER;
use crate::model::DAYS;
use crate::pal::{BODY, H1, SMALL, Pal};
use crate::State;

fn bar(p: &Pal, w: f32, h: f32) -> El {
    el().w(w).h(h).radius(h / 2.0).fill(p.sep.alpha(0.5)).looping(LoopSpec::opacity(1.6, 0.0, 0.45, 1.0))
}

pub fn body(s: &State, cx: &mut Cx<State>, p: &Pal, small: bool) -> El {
    let [top, right, bottom, left] = if small { cx.safe_area() } else { [0.0; 4] };
    let head = if small {
        row().w_full().h(HEADER).px(14.0).gap(10.0).center_y().fill(p.card).border(1.0, p.sep)
            .child(header::seal(s, p))
            .child(col().grow(1.0).gap(5.0).child(text("Timetable").font(H1).color(p.text)).child(bar(p, 150.0, 8.0)))
    } else {
        row().w_full().h(60.0).px(20.0).gap(14.0).center_y().fill(p.card).border(1.0, p.sep)
            .child(header::seal(s, p))
            .child(col().gap(5.0).child(text("Timetable").font(H1).color(p.text)).child(bar(p, 220.0, 8.0)))
            .child(el().grow(1.0))
            .child(bar(p, 70.0, 12.0)).child(bar(p, 70.0, 12.0)).child(bar(p, 70.0, 12.0))
    };
    // An empty week: day heads and hour lines, with a few blocks breathing.
    let days = if small { 1 } else { 5 };
    let mut g = row().w_full().grow(1.0).min_h(0.0).radius(14.0).fill(p.card).border(1.0, p.sep).clip_rounded(14.0);
    for d in 0..days {
        let mut c = col().grow(1.0).min_w(0.0).border(1.0, p.sep.alpha(0.0))
            .child(el().w_full().h(38.0).center().child(text(DAYS[if small { 0 } else { d }]).font(SMALL).color(p.sub)));
        for k in 0..8 {
            let block = (d + k) % 3 == 0 && k % 2 == 0;
            c = c.child(el().w_full().h(1.0).fill(p.sep));
            c = c.child(el().w_full().h(46.0).px(6.0).py(4.0).child(if block { bar(p, 10.0, 36.0).w_full().radius(8.0) } else { el() }));
        }
        g = g.child(c);
    }
    let main = col().grow(1.0).min_w(0.0).h_full().padding(if small { 10.0 } else { 20.0 }).gap(10.0)
        .child(text("Loading the timetable…").font(BODY).color(p.sub))
        .child(g);
    let side = if small { el() } else {
        col().w(340.0).h_full().padding(16.0).gap(12.0).border(1.0, p.sep)
            .child(el().w_full().h(40.0).radius(10.0).fill(p.raised).border(1.0, p.sep))
            .child(bar(p, 200.0, 10.0)).child(bar(p, 260.0, 10.0)).child(bar(p, 120.0, 10.0))
    };
    col().key("skeleton").w_full().h_full().pt(top).pr(right).pb(bottom).pl(left)
        .child(head)
        .child(row().w_full().grow(1.0).min_h(0.0).child(side).child(main))
}

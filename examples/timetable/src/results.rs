//! Search results shared by the sidebar and the palette: ranked rows with faculty, contact hours
//! and lecture-day dots, plus the empty-query view ("Browse by faculty" and recently added).

use rune::{El, Transition, col, el, row, text};

use crate::data::Course;
use crate::pal::{CODE, LABEL, SMALL, TINY, Pal};
use crate::search::{faculties, recent, search_in};
use crate::State;

/// What the list shows: matches for the query (within the chosen faculty), or, with nothing typed
/// and no faculty chosen, the recently added courses.
pub fn hits(s: &State, query: &str) -> Vec<usize> {
    if query.trim().is_empty() && s.faculty.is_none() {
        return recent(&s.cat, &s.recent, |i| s.enr.iter().any(|e| e.course == i));
    }
    search_in(&s.cat, query, s.faculty.as_deref())
}

/// Whether the query is empty and so the browse chips belong above the list.
pub fn browsing(query: &str) -> bool { query.trim().is_empty() }

/// Five weekday dots: brass where the course has a lecture.
pub fn day_dots(p: &Pal, c: &Course) -> El {
    let days = c.lecture_days();
    let mut r = row().gap(3.0).center_y();
    for d in 0..5 {
        r = r.child(el().size(6.0, 6.0).radius(3.0).fill(if days[d] { p.brass } else { p.sep }));
    }
    r
}

/// One result. `wide` lays it on one line (the palette); otherwise it takes two (the sidebar and phones).
#[allow(clippy::too_many_arguments)]
pub fn result_row(p: &Pal, key: String, i: usize, c: &Course, added: bool, on: bool, wide: bool, tall: bool) -> El {
    let hours = c.contact_hours();
    let add = if added {
        text("Added").font(LABEL).color(p.sub)
    } else {
        el().h(24.0).px(8.0).center().radius(6.0).fill(if on { p.brass } else { p.hover })
            .child(text(if wide { "Add ↵" } else { "Add" }).font(LABEL).color(if on { p.bg } else { p.sub }))
    };
    let faculty = if c.faculty.is_empty() { c.dept.clone() } else { c.faculty.clone() };
    let mut r = row().key(key).w_full().px(12.0).gap(10.0).center_y().radius(10.0)
        .fill(if on { p.brass.alpha(0.14) } else { p.raised.alpha(if wide { 0.0 } else { 1.0 }) })
        .border(1.0, if wide { p.sep.alpha(0.0) } else { p.sep })
        .transition(Transition::offset(0.0, 6.0).combined(Transition::opacity()));
    if wide {
        r = r.h(52.0).child(col().grow(1.0).min_w(0.0)
            .child(row().gap(8.0).center_y().child(text(c.code.clone()).font(CODE).color(if on { p.brass } else { p.text }))
                .child(text(faculty).font(TINY).color(p.sub).truncate()))
            .child(text(c.name.clone()).font(SMALL).color(p.sub).truncate()))
            .child(day_dots(p, c))
            .child(text(format!("{hours:.0}h")).font(SMALL).color(p.sub))
            .child(add);
    } else {
        r = r.h(if tall { 64.0 } else { 58.0 }).child(col().grow(1.0).min_w(0.0).gap(1.0)
            .child(row().gap(8.0).center_y().child(text(c.code.clone()).font(CODE).color(p.text)).child(el().grow(1.0))
                .child(day_dots(p, c)).child(text(format!("{hours:.0}h")).font(SMALL).color(p.sub)))
            .child(text(c.name.clone()).font(SMALL).color(p.text).truncate())
            .child(text(faculty).font(TINY).color(p.sub).truncate()))
            .child(add);
    }
    if !added { r = r.matched(format!("course-{i}")).press_scale(0.98).hover(p.hover); }
    r
}

/// "Browse by faculty" chips, with the chosen one filled. `on_pick` is the same for every chip.
pub fn browse_chips(s: &State, p: &Pal, small: bool) -> El {
    let mut chips = row().gap(6.0).flex_wrap().w_full();
    for (f, n) in faculties(&s.cat).into_iter().take(10) {
        let on = s.faculty.as_deref() == Some(f.as_str());
        let name = f.clone();
        let chip = el().key(format!("fac-{f}")).h(if small { 36.0 } else { 26.0 }).px(10.0).center().radius(13.0).press_scale(0.94)
            .on_press(move |st: &mut State| st.faculty = if st.faculty.as_deref() == Some(name.as_str()) { None } else { Some(name.clone()) });
        chips = chips.child(if on {
            chip.fill(p.text).child(text(format!("{f} · {n}")).font(LABEL).color(p.bg))
        } else {
            chip.fill(p.raised).border(1.0, p.sep).child(text(format!("{f} · {n}")).font(LABEL).color(p.text))
        });
    }
    col().gap(6.0).w_full().child(text("BROWSE BY FACULTY").font(TINY).color(p.sub)).child(chips)
}

/// A small heading over a list.
pub fn heading(p: &Pal, label: &str) -> El { text(label.to_owned()).font(TINY).color(p.sub) }

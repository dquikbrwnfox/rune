//! The course list: search, browse by faculty, recently added, and the cards. A picked result
//! flies into its card.

use rune::{El, ScrollSpec, Transition, col, el, text, text_input};

use crate::card::course_card;
use crate::model::{active, clash_pairs, clash_set, week_mask};
use crate::pal::{BODY, LABEL, TITLE, Pal};
use crate::results::{browse_chips, browsing, heading, hits, result_row};
use crate::{State, add, try_sample};

pub fn sidebar(s: &State, p: &Pal) -> El {
    let small = crate::grid::compact();
    let search_box = text_input(&s.search, BODY).key("search")
        .placeholder("Search courses…", p.sub).color(p.text)
        .w_full().h(if small { 44.0 } else { 40.0 }).padding(12.0).radius(10.0).fill(p.raised).border(1.0, p.sep)
        .on_edit(|st: &mut State| &mut st.search);

    let mut list = col().key("side-list").w_full().gap(10.0).pb(16.0);
    let q = s.search.text.trim();
    let locked = crate::locked(s);
    if !locked {
        if browsing(q) && (s.enr.is_empty() || s.search.focused || s.faculty.is_some()) { list = list.child(browse_chips(s, p, small)); }
        let found = hits(s, q);
        let mut shown = 0;
        if !q.is_empty() || s.faculty.is_some() || !found.is_empty() {
            if q.is_empty() && s.faculty.is_none() { list = list.child(heading(p, "RECENTLY ADDED")); }
            for i in found {
                if s.enr.iter().any(|e| e.course == i) { continue; }
                let c = &s.cat[i];
                list = list.child(result_row(p, format!("res-{i}"), i, c, false, false, false, small)
                    .enter_delay(shown.min(8) as f32 * 0.03)
                    .on_press(move |st: &mut State| { add(st, i); st.search.clear(); }));
                shown += 1;
            }
            if shown == 0 && (!q.is_empty() || s.faculty.is_some()) {
                list = list.child(text(if q.is_empty() { "Nothing left in this faculty.".to_owned() } else { format!("No course matches “{}”.", s.search.text) })
                    .key("nomatch").font(BODY).color(p.sub));
            }
        }
        if s.enr.is_empty() && q.is_empty() && s.faculty.is_none() {
            list = list.child(col().key("empty").w_full().padding(16.0).gap(8.0).radius(12.0).border(1.0, p.sep).border_dash(5.0, 4.0)
                .transition(Transition::opacity())
                .child(text("Start by adding a course.").font(TITLE).color(p.text).w_full())
                .child(text("Search by code, name or department. Lectures land straight away; tutorials and labs are picked for you, and show as dashed ghosts you can switch to.")
                    .font(BODY).color(p.sub).w_full().wrap())
                .child(el().key("sample").h(if small { 44.0 } else { 32.0 }).px(12.0).center().radius(8.0).fill(p.brass).press_scale(0.96)
                    .child(text("Try a sample first year").font(LABEL).color(p.bg))
                    .on_press(|st: &mut State| try_sample(st))));
        }
    }
    let wm = week_mask(s.week);
    let clashes = clash_set(&s.cat, &s.enr, wm);
    let sess = active(&s.cat, &s.enr);
    let pairs = clash_pairs(&sess, wm);
    for e in &s.enr { list = list.child(course_card(s, e, p, &clashes, &sess, &pairs)); }

    let side = col().key("side");
    let side = if small { side.w_full().grow(1.0).min_h(0.0) } else { side.w(340.0).h_full().border(1.0, p.sep) };
    side.padding(if small { 12.0 } else { 16.0 }).gap(12.0).fill(p.bg)
        .child(if locked { el() } else { search_box })
        .child(list.scroll(ScrollSpec { stick_bottom: false }).scroll_edge_fade(12.0).grow(1.0).min_h(0.0))
}

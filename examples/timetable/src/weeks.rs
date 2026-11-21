//! The week scrubber: "All weeks" and one pill per week CELCAT has published. Picking a week
//! fades the sessions that don't run in it and limits clashes to it. It also says plainly that
//! only some weeks are published.

use rune::{El, Spring, col, el, row, text};

use crate::export::{civil_from_days, days_from_civil_iso};
use crate::pal::{LABEL, SMALL, TINY, Pal};
use crate::{State, set_week};

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// "Mon 2 Nov" for the Monday of teaching week `w`, from the inferred week-1 Monday.
pub fn week_start_text(s: &State, w: u32) -> Option<String> {
    let base = days_from_civil_iso(&s.meta.week1_monday)?;
    let (_, m, d) = civil_from_days(base + i64::from(w.max(1) - 1) * 7);
    Some(format!("Mon {d} {}", MONTHS[(m as usize).clamp(1, 12) - 1]))
}

/// The line under the pills.
pub fn caption(s: &State) -> String {
    let published = s.meta.weeks_text();
    match (s.week, s.meta.sample) {
        (_, true) => "Sample data covers a full 13-week term.".to_owned(),
        (Some(w), _) => {
            let when = week_start_text(s, w).map(|t| format!(" ({t}, inferred)")).unwrap_or_default();
            format!("Showing week {w}{when}. CELCAT has published {published} so far.")
        }
        (None, _) if published.is_empty() => String::new(),
        (None, _) => format!("CELCAT has published {published} so far."),
    }
}

pub fn scrubber(s: &State, p: &Pal, small: bool) -> El {
    let h = if small { 40.0 } else { 28.0 };
    let pill = |key: String, label: String, on: bool, now: bool, week: Option<u32>| {
        let ink = if on { p.bg } else { p.text };
        let mut e = row().key(key).h(h).px(if small { 14.0 } else { 11.0 }).gap(5.0).center().radius(h / 2.0).motion(Spring::SNAPPY)
            .fill(if on { p.text } else { p.raised }).border(1.0, if on { p.text } else { p.sep }).hover(if on { p.text } else { p.hover })
            .press_scale(0.94)
            .child(text(label).font(LABEL).color(ink))
            .on_press(move |s: &mut State| set_week(s, week));
        if now { e = e.child(el().size(5.0, 5.0).radius(2.5).fill(if on { p.bg } else { p.brass })); }
        e
    };
    let mut pills = row().gap(6.0).center_y();
    pills = pills.child(pill("wk-all".into(), "All weeks".into(), s.week.is_none(), false, None));
    for &w in &s.meta.weeks_published {
        let now = s.cur_week == Some(i64::from(w));
        pills = pills.child(pill(format!("wk-{w}"), format!("Week {w}"), s.week == Some(w), now, Some(w)));
    }
    let strip = if small || s.meta.weeks_published.len() > 8 { pills.scroll_x().h(h + 4.0).w_full().scroll_edge_fade(10.0) } else { pills.flex_wrap() };
    let cap = caption(s);
    let mut c = col().key("scrubber").w_full().gap(4.0).child(strip);
    if !cap.is_empty() { c = c.child(text(cap).font(if small { TINY } else { SMALL }).color(p.sub).w_full().wrap()); }
    c
}

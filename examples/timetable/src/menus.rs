//! Popovers on the desktop and bottom sheets on a phone: the solver's preferences, share and
//! export, the data source, and a block's details. Plus the toast that confirms a copy.

use rune::{Cx, El, Spring, Transition, Tx, col, el, row, text};

use crate::detail;
use crate::model::Sess;
use crate::pal::{BODY, LABEL, SMALL, TINY, TITLE, Pal};
use crate::{Menu, State, layer};

/// The open menu or block detail, wrapped as a modal layer; `None` when nothing is open.
pub fn overlay(s: &State, cx: &Cx<State>, p: &Pal, sess: &[Sess], small: bool) -> Option<El> {
    let win = cx.layout_size("root").unwrap_or([1240.0, 800.0]);
    if let Some(id) = &s.detail {
        let x = detail::find(sess, id)?;
        let anchor = cx.layout(id);
        let panel = detail::content(s, p, x, small);
        return Some(shell(p, win, small, "detail-layer", anchor, 340.0, panel, cx.safe_area()[2], |s| s.detail = None));
    }
    let (key, anchor_key, w, body) = match s.menu? {
        Menu::Prefs => ("prefs", "prefs-btn", 320.0, prefs_panel(s, p, small)),
        Menu::Share => ("share", "share-btn", 320.0, share_panel(s, p, small)),
        Menu::Term => ("term", "term-chip", 340.0, term_panel(s, p, small)),
    };
    Some(shell(p, win, small, key, cx.layout(anchor_key), w, body, cx.safe_area()[2], |s| s.menu = None))
}

/// The modal layer: a card under its anchor (desktop) or a sheet from the bottom (phone).
#[allow(clippy::too_many_arguments)]
fn shell(p: &Pal, win: [f32; 2], small: bool, key: &str, anchor: Option<[f32; 4]>, w: f32, body: El, bottom: f32, close: fn(&mut State)) -> El {
    let layer = layer().key(key.to_owned()).transition(Transition::opacity()).modal(true, close);
    if small {
        let sheet = col().key(format!("{key}-sheet")).w_full().padding(16.0).pb(16.0 + bottom).gap(12.0)
            .radius(18.0).fill(p.card).border(1.0, p.sep).shadow(0.0, 40.0, p.shadow)
            .transition(Transition::offset(0.0, 80.0).combined(Transition::opacity()))
            .child(el().w(36.0).h(4.0).radius(2.0).fill(p.sep).self_center())
            .child(col().w_full().max_h(win[1] * 0.78 - 60.0 - bottom).scroll(rune::ScrollSpec { stick_bottom: false }).child(body));
        return layer.fill(p.scrim).child(col().w_full().h_full().justify_end().child(sheet));
    }
    let (x, y) = match anchor {
        Some([ax, ay, aw, ah]) => {
            // Under a header button, right-aligned to it; beside a block, on whichever side has room.
            if key == "detail-layer" {
                let right = ax + aw + 10.0;
                let x = if right + w + 12.0 < win[0] { right } else { (ax - w - 10.0).max(12.0) };
                (x, ay.clamp(70.0, (win[1] - 420.0).max(70.0)))
            } else {
                (((ax + aw) - w).clamp(12.0, (win[0] - w - 12.0).max(12.0)), ay + ah + 8.0)
            }
        }
        None => (win[0] - w - 16.0, 68.0),
    };
    layer.child(col().key(format!("{key}-card")).w(w).max_h(win[1] - y - 12.0).padding(14.0).gap(10.0).radius(14.0)
        .fill(p.card).border(1.0, p.sep).shadow(10.0, 36.0, p.shadow)
        .transition(Transition::scale(0.95).combined(Transition::opacity()).combined(Transition::offset(0.0, -8.0)))
        .absolute(x, y)
        .scroll(rune::ScrollSpec { stick_bottom: false })
        .child(body))
}

fn title(p: &Pal, t: &str, sub: &str) -> El {
    col().w_full().gap(2.0).child(text(t.to_owned()).font(TITLE).color(p.text)).child(text(sub.to_owned()).font(SMALL).color(p.sub).w_full().wrap())
}

fn switch(p: &Pal, on: bool) -> El {
    el().size(40.0, 24.0).radius(12.0).fill(if on { p.brass } else { p.sep }).motion(Spring::SNAPPY)
        .child(el().size(18.0, 18.0).radius(9.0).fill(if on { p.bg } else { p.card }).absolute(if on { 19.0 } else { 3.0 }, 3.0).motion(Spring::BOUNCY))
}

fn toggle(p: &Pal, key: &str, label: &str, sub: &str, on: bool, small: bool, f: fn(&mut State)) -> El {
    row().key(key.to_owned()).w_full().h(if small { 56.0 } else { 46.0 }).gap(10.0).center_y().radius(8.0).hover(p.hover).press_scale(0.98)
        .child(col().grow(1.0).min_w(0.0).child(text(label.to_owned()).font(BODY).color(p.text)).child(text(sub.to_owned()).font(SMALL).color(p.sub)))
        .child(switch(p, on))
        .on_press(f)
}

fn prefs_panel(s: &State, p: &Pal, small: bool) -> El {
    let pr = s.prefs;
    col().w_full().gap(8.0)
        .child(title(p, "Make it work", "When several weeks are clash-free, the solver picks the one that does best on these."))
        .child(toggle(p, "pref-free", "Keep a day free", "Use as few weekdays as possible", pr.free_day, small, |s| s.prefs.free_day = !s.prefs.free_day))
        .child(toggle(p, "pref-early", "No early starts", "Avoid classes before 9am", pr.no_early, small, |s| s.prefs.no_early = !s.prefs.no_early))
        .child(toggle(p, "pref-late", "No late finishes", "Avoid classes ending after 6pm", pr.no_late, small, |s| s.prefs.no_late = !s.prefs.no_late))
        .child(toggle(p, "pref-compact", "Compact days", "Fewer gaps between classes", pr.compact, small, |s| s.prefs.compact = !s.prefs.compact))
        .child(action(p, "pref-go", "Make it work now", true, small).on_press(|s: &mut State| crate::solve_all(s)))
}

fn action(p: &Pal, key: &str, label: &str, primary: bool, small: bool) -> El {
    let b = el().key(key.to_owned()).w_full().h(if small { 48.0 } else { 36.0 }).center().radius(9.0).press_scale(0.97);
    if primary { b.fill(p.brass).child(text(label.to_owned()).font(LABEL).color(p.bg)) }
    else { b.fill(p.raised).border(1.0, p.sep).hover(p.hover).child(text(label.to_owned()).font(LABEL).color(p.text)) }
}

fn menu_row(p: &Pal, key: &str, label: &str, sub: &str, small: bool) -> El {
    col().key(key.to_owned()).w_full().min_h(if small { 56.0 } else { 46.0 }).px(10.0).py(6.0).justify_center().radius(9.0).hover(p.hover).press_scale(0.98)
        .child(text(label.to_owned()).font(BODY).color(p.text))
        .child(text(sub.to_owned()).font(SMALL).color(p.sub).w_full().wrap())
}

fn share_panel(s: &State, p: &Pal, small: bool) -> El {
    let n = s.enr.len();
    let mut c = col().w_full().gap(4.0).child(title(p, "Share and export", &format!("{n} course{} in this schedule.", if n == 1 { "" } else { "s" })));
    c = c.child(menu_row(p, "share-link", "Copy share link", "Anyone who opens it can view or copy this schedule", small)
        .on_press_tx(|s: &mut State, tx: &mut Tx| crate::share(s, tx)))
        .child(menu_row(p, "share-ics", "Download calendar (.ics)", "Every class as events for Google, Apple or Outlook calendar", small)
            .on_press(|s: &mut State| crate::export_ics(s)))
        .child(menu_row(p, "share-txt", "Copy as text", "A plain list for a chat or your notes", small)
            .on_press_tx(|s: &mut State, tx: &mut Tx| crate::copy_summary(s, tx)));
    c
}

fn term_panel(s: &State, p: &Pal, _small: bool) -> El {
    let m = &s.meta;
    let mut c = col().w_full().gap(8.0)
        .child(title(p, if m.sample { "Sample data" } else { &m.term_label }, "Unofficial. Data from UWI CELCAT. Check the official timetable before relying on it."));
    let mut fact = |label: &str, v: String| {
        if v.is_empty() { return; }
        c = std::mem::replace(&mut c, col()).child(col().w_full().gap(1.0).child(text(label.to_owned()).font(TINY).color(p.sub)).child(text(v).font(BODY).color(p.text).w_full().wrap()));
    };
    if m.sample {
        fact("WHY", "celcat.json could not be loaded, so this built-in sample stands in. It is not the real timetable.".to_owned());
    } else {
        fact("DATA FETCHED", m.generated_text());
        fact("WEEKS PUBLISHED", format!("CELCAT has published {} so far.", m.weeks_text()));
        fact("DATES", crate::weeks::week_start_text(s, 1).map(|t| format!("Week 1 starts {t} (inferred from the academic calendar).")).unwrap_or_default());
        fact("COURSES", format!("{} in the catalogue", s.cat.len()));
    }
    let src = m.source.clone();
    if !src.is_empty() {
        c = c.child(action(p, "term-open", "Open the official CELCAT timetable", false, false)
            .on_press_tx(move |_: &mut State, tx: &mut Tx| { let _ = tx.open_url(&src); }));
    }
    c
}

/// The confirmation pill: bottom centre, above the tab bar on a phone.
pub fn toast(s: &State, cx: &Cx<State>, p: &Pal, small: bool) -> Option<El> {
    let t = s.toast.as_ref()?;
    let lift = if small { 74.0 + cx.safe_area()[2] } else { 24.0 };
    Some(layer().key("toast-layer").inert().child(
        col().w_full().h_full().justify_end().items_center().pb(lift).child(
            row().key("toast").h(40.0).px(16.0).gap(8.0).center_y().radius(20.0).fill(p.text).shadow(4.0, 18.0, p.shadow)
                .transition(Transition::offset(0.0, 16.0).combined(Transition::opacity()))
                .child(el().size(8.0, 8.0).radius(4.0).fill(p.brass))
                .child(text(t.msg.clone()).font(LABEL).color(p.bg)))))
}

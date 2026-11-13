//! The masthead: a struck-brass seal, the term chip, odometer stats, the actions, and the sun
//! that turns into a moon. Plus the notice bars under it.

use std::f32::consts::PI;

use rune::{El, Spring, Transition, col, el, row, text};
use rune_kit::{Kit, RollingText};

use crate::load::Status;
use crate::model::{Sess, free_days_text};
use crate::pal::{H1, LABEL, SMALL, STAT, Pal};
use crate::{Menu, State, cmdk, store};

const ROLL: Spring = Spring::new(0.38, 0.12);

/// "Semester 1, 2026/2027 · weeks 10–12 · Unofficial", or the sample's own line.
pub fn term_line(s: &State) -> String {
    match &s.status {
        Status::Loading => "Loading the timetable…".to_owned(),
        Status::Fallback(_) => "Sample data, offline · unofficial".to_owned(),
        Status::Ready => format!("{} · {} · unofficial", s.meta.term_label, s.meta.weeks_text()),
    }
}

/// A small down-pointing chevron, drawn (the font has no glyph for one).
pub fn chevron(c: rune::Color) -> El {
    rune::canvas(rune::Shape::new().view_box(9.0, 5.0).stroke(vec![[0.5, 0.5], [4.5, 4.3], [8.5, 0.5]], 1.4, c)).size(9.0, 5.0).inert()
}

/// The chip under the title that opens the data source: term, weeks and the unofficial label.
pub fn term_chip(s: &State, p: &Pal, short: bool) -> El {
    let fallback = matches!(s.status, Status::Fallback(_));
    let label = if short {
        if fallback { "Sample data".to_owned() } else { format!("{} · wks {}", s.meta.short_term(), s.meta.weeks_published.first().zip(s.meta.weeks_published.last()).map_or(String::new(), |(a, b)| if a == b { a.to_string() } else { format!("{a}–{b}") })) }
    } else { term_line(s) };
    let tint = if fallback { p.danger } else { p.sub };
    let on = s.menu == Some(Menu::Term);
    row().key("term-chip").h(if short { 22.0 } else { 20.0 }).px(7.0).gap(5.0).center_y().radius(6.0).hover(p.hover).press_scale(0.97)
        .fill(if on { p.hover } else { tint.alpha(0.0) })
        .child(el().size(6.0, 6.0).radius(3.0).fill(if fallback { p.danger } else { p.brass }))
        .child(text(label).font(SMALL).color(tint).truncate())
        .child(chevron(p.sub))
        .label("Data source and term")
        .on_press(|s: &mut State| s.menu = if s.menu == Some(Menu::Term) { None } else { Some(Menu::Term) })
}

pub fn header(s: &State, p: &Pal, sess: &[Sess], n_clash: usize) -> El {
    let kit = Kit::neutral(p.dark);
    let hours: f32 = sess.iter().filter(|x| x.kind != 4).map(|x| x.hours).sum::<f32>() + 0.0; // + 0.0 turns -0 into 0
    let roll = |key: &str, v: f64, txt: String, c| RollingText::new(key, txt).value(v).font(STAT).color(c).spring(ROLL).build(&kit);
    let stat = |key: &str, v: f64, num: String, unit: &str, c| {
        row().key(format!("{key}-stat")).gap(5.0).center_y().width_leads().motion(ROLL)
            .child(roll(key, v, num, c))
            .child(text(unit.to_owned()).font(SMALL).color(p.sub))
    };
    let clash_c = if n_clash > 0 { p.danger } else { p.sub };
    let free = free_days_text(sess);
    let mut r = row().key("header").w_full().h(60.0).px(20.0).gap(14.0).center_y().fill(p.card)
        .border(1.0, p.sep)
        .child(seal(s, p))
        .child(col().gap(1.0).min_w(0.0)
            .child(text("Timetable").font(H1).color(p.text))
            .child(term_chip(s, p, false)))
        .child(el().grow(1.0))
        .child(stat("courses", s.enr.len() as f64, s.enr.len().to_string(), if s.enr.len() == 1 { "course" } else { "courses" }, p.text))
        .child(dot(p))
        .child(stat("hours", f64::from(hours), format!("{hours:.0}"), "hrs / wk", p.text))
        .child(dot(p))
        .child(stat("clashes", n_clash as f64, n_clash.to_string(), if n_clash == 1 { "clash" } else { "clashes" }, clash_c));
    if !free.is_empty() {
        r = r.child(dot(p)).child(text(free.clone()).key("free-days").font(SMALL).color(if free.starts_with("No") { p.sub } else { p.brass }));
    }
    r.child(el().w(6.0))
        .child(search_button(p))
        .child(actions(s, p, false))
        .child(calm_button(s, p, false))
        .child(lamp_button(s, p))
        .child(theme_switch(s, p))
}

/// Make it work (with its preferences) and Share. `wide` makes them fill a row, for a phone.
pub fn actions(s: &State, p: &Pal, wide: bool) -> El {
    let off = crate::locked(s) || s.enr.is_empty();
    let h = if wide { 44.0 } else { 32.0 };
    let solve = button(p, "solve-btn", "Make it work", true, h).opacity(if off { 0.4 } else { 1.0 })
        .on_press(move |s: &mut State| crate::solve_all(s));
    let prefs = button(p, "prefs-btn", "Prefs", false, h)
        .on_press(|s: &mut State| s.menu = if s.menu == Some(Menu::Prefs) { None } else { Some(Menu::Prefs) });
    let share = button(p, "share-btn", "Share", false, h).opacity(if s.enr.is_empty() { 0.4 } else { 1.0 })
        .on_press(|s: &mut State| if !s.enr.is_empty() { s.menu = if s.menu == Some(Menu::Share) { None } else { Some(Menu::Share) } });
    row().key("actions").gap(6.0).center_y().child(solve).child(prefs).child(share)
}

fn button(p: &Pal, key: &str, label: &str, primary: bool, h: f32) -> El {
    let b = el().key(key.to_owned()).h(h).px(12.0).center().radius(8.0).press_scale(0.95).hover(p.hover);
    if primary { b.fill(p.brass).child(text(label.to_owned()).font(LABEL).color(p.bg)) }
    else { b.fill(p.raised).border(1.0, p.sep).child(text(label.to_owned()).font(LABEL).color(p.text)) }
}

/// The calm-mode switch: static red instead of fire and lightning, no aurora or sparkles.
pub fn calm_button(s: &State, p: &Pal, tall: bool) -> El {
    let on = s.shown_calm.get();
    row().key("calm").h(if tall { 44.0 } else { 34.0 }).px(11.0).gap(6.0).center_y().radius(17.0).fill(p.raised)
        .border(1.0, if on { p.brass } else { p.sep }).hover(p.hover).press_scale(0.94)
        .child(el().size(8.0, 8.0).radius(4.0).fill(if on { p.brass } else { p.sub.alpha(0.4) }))
        .child(text("Calm").font(LABEL).color(if on { p.text } else { p.sub }))
        .label(if on { "Calm mode on: effects reduced" } else { "Calm mode off" })
        .help("Calm mode (C): replaces fire, lightning, aurora and sparkles with a static treatment")
        .on_press(|s: &mut State| crate::toggle_calm(s))
}

/// How many notice rows `notices` shows.
pub fn notice_count(s: &State) -> usize {
    usize::from(matches!(s.status, Status::Fallback(_))) + usize::from(s.pending_share.is_some()) + usize::from(s.viewing.is_some())
        + usize::from(!s.missing.is_empty()) + usize::from(s.notice.is_some())
}

/// Notices under the masthead: data fallback, a share link waiting, a shared schedule being
/// viewed, courses that went missing, and the solver's last answer.
pub fn notices(s: &State, p: &Pal, small: bool) -> El {
    let bar = |key: &str, msg: String, tint: rune::Color, buttons: Vec<El>| {
        let msg = text(msg).font(SMALL).color(p.text).grow(1.0).min_w(0.0).wrap();
        let b = rune::row().gap(6.0).center_y().flex_wrap();
        let b = buttons.into_iter().fold(b, |b, x| b.child(x));
        let base = if small { col().gap(6.0) } else { row().gap(10.0).center_y() };
        base.key(key.to_owned()).w_full().px(16.0).py(8.0)
            .fill(tint.alpha(0.12)).border(1.0, tint.alpha(0.5))
            .transition(Transition::offset(0.0, -8.0).combined(Transition::opacity()))
            .child(msg).child(b)
    };
    let pill = |key: &str, label: &str, primary: bool| {
        let b = el().key(key.to_owned()).h(if small { 40.0 } else { 26.0 }).px(10.0).center().radius(7.0).press_scale(0.94);
        if primary { b.fill(p.brass).child(text(label.to_owned()).font(LABEL).color(p.bg)) }
        else { b.border(1.0, p.sep).child(text(label.to_owned()).font(LABEL).color(p.text)) }
    };
    let mut col_ = col().key("notices").w_full();
    if let Status::Fallback(why) = &s.status {
        col_ = col_.child(bar("n-fallback", format!("Could not load celcat.json ({why}). This is sample data, not the real timetable."), p.danger, vec![]));
    }
    if let Some(sh) = &s.pending_share {
        let n = sh.courses.len();
        let mut msg = format!("A shared schedule with {n} course{} was opened.", if n == 1 { "" } else { "s" });
        if store::term_mismatch(s, sh) {
            msg.push_str(&format!(" It was made for a different term ({}); this data is {}. Courses are matched by code and may differ.", sh.term.replace('_', " "), s.meta.term_label));
        }
        col_ = col_.child(bar("n-share", msg, p.brass, vec![
            pill("n-share-yes", "Replace my schedule", true).on_press(|s: &mut State| store::apply_share(s)),
            pill("n-share-view", "View only", false).on_press(|s: &mut State| store::view_share(s)),
            pill("n-share-no", "Dismiss", false).on_press(|s: &mut State| store::dismiss_share(s)),
        ]));
    }
    if s.viewing.is_some() {
        col_ = col_.child(bar("n-viewing", "Viewing a shared schedule, read-only. Your own schedule is kept aside.".to_owned(), p.brass, vec![
            pill("n-keep", "Keep mine", true).on_press(|s: &mut State| store::keep_mine(s)),
            pill("n-adopt", "Make this mine", false).on_press(|s: &mut State| store::adopt_viewed(s)),
        ]));
    }
    if !s.missing.is_empty() {
        col_ = col_.child(bar("n-missing", format!("Not in this term's data: {}.", s.missing.join(", ")), p.danger,
            vec![pill("n-missing-ok", "OK", false).on_press(|s: &mut State| s.missing.clear())]));
    }
    if let Some(n) = &s.notice {
        col_ = col_.child(bar("n-note", n.msg.clone(), if n.bad { p.danger } else { p.brass },
            vec![pill("n-note-ok", "OK", false).on_press(|s: &mut State| s.notice = None)]));
    }
    col_
}

fn dot(p: &Pal) -> El { el().size(3.0, 3.0).radius(1.5).fill(p.sep) }

/// The "U" seal: struck brass, drawn with plain paint (a gradient, a bevelled rim and a soft top
/// gloss), so it costs nothing once it's on screen: no shader layer, no frames of its own.
pub fn seal(_s: &State, p: &Pal) -> El {
    let hi = p.brass.mix(p.raised, 0.55);
    el().key("seal").size(34.0, 34.0).radius(10.0).center()
        .linear_gradient(2.4, [(0.0, p.brass.mix(p.raised, 0.35)), (0.55, p.brass), (1.0, p.brass.mix(p.bg, 0.45))])
        .border_gradient(1.0, hi.alpha(0.9), p.brass.mix(p.bg, 0.6).alpha(0.7))
        .shadow(2.0, 8.0, p.shadow).press_scale(0.92)
        .child(el().key("seal-gloss").pin(Some(1.0), Some(1.0), Some(1.0), Some(17.0)).radius(9.0).inert()
            .gradient(hi.alpha(0.5), hi.alpha(0.0)))
        .child(text("U").font(H1).color(rune::oklch(0.27, 0.05, 65.0)))
}

/// Lights on or off: a bulb that glows when the scene is lit.
pub fn lamp_button(s: &State, p: &Pal) -> El {
    let on = s.shown_lights.get();
    el().key("lamp").size(34.0, 34.0).radius(17.0).center().fill(p.raised).border(1.0, if on { p.brass } else { p.sep })
        .hover(p.hover).press_scale(0.9).motion(Spring::BOUNCY)
        .shadow(0.0, if on { 16.0 } else { 0.0 }, p.brass.alpha(if on { 0.55 } else { 0.0 }))
        .on_press(|s: &mut State| crate::toggle_lights(s))
        .label(if on { "Turn lights off" } else { "Turn lights on" })
        .child(col().items_center().gap(1.0)
            .child(el().key("bulb").size(12.0, 12.0).radius(6.0).fill(if on { p.brass } else { p.sub.alpha(0.4) }).scale(if on { 1.0 } else { 0.8 }).motion(Spring::BOUNCY))
            .child(el().size(6.0, 3.0).radius(1.0).fill(p.sub)))
}

/// A magnifier drawn from two shapes, so it needs no icon font.
pub fn search_button(p: &Pal) -> El {
    el().key("cmdk-btn").size(34.0, 34.0).radius(17.0).fill(p.raised).border(1.0, p.sep).press_scale(0.9).hover(p.hover)
        .on_press(|s: &mut State| cmdk::open(s))
        .label("Find a course")
        .help("Find a course (/)")
        .child(el().size(12.0, 12.0).radius(6.0).border(2.0, p.sub).absolute(9.0, 9.0))
        .child(el().size(2.0, 7.0).radius(1.0).fill(p.sub).absolute(21.0, 19.0).rotate(-0.8))
}

/// A sun whose rays retract while the disc turns half a revolution and a shadow slides across
/// it into a crescent.
pub fn theme_switch(s: &State, p: &Pal) -> El {
    let dark = p.dark;
    let spin = Spring::new(0.7, 0.28);
    let mut sun = el().key("sun").size(30.0, 30.0).rotate(if dark { PI } else { 0.0 }).rotate_motion(spin);
    for i in 0..8 {
        let a = i as f32 * PI / 4.0;
        let (cx, cy) = (15.0 + a.cos() * 11.0, 15.0 + a.sin() * 11.0);
        sun = sun.child(el().key(format!("ray-{i}")).size(2.0, 5.0).radius(1.0).fill(p.brass)
            .absolute(cx - 1.0, cy - 2.5).rotate(a + PI / 2.0)
            .scale(if dark { 0.0 } else { 1.0 }).opacity(if dark { 0.0 } else { 1.0 }).motion(spin));
    }
    let disc = el().key("disc").size(14.0, 14.0).radius(7.0).fill(p.brass).absolute(8.0, 8.0)
        .scale(if dark { 1.25 } else { 1.0 }).motion(spin);
    let bite = el().key("bite").size(13.0, 13.0).radius(6.5).fill(p.raised)
        .absolute(if dark { 13.0 } else { 26.0 }, if dark { 5.0 } else { -6.0 })
        .opacity(if dark { 1.0 } else { 0.0 }).motion(spin);
    el().key("theme").size(34.0, 34.0).radius(17.0).center().fill(p.raised).border(1.0, p.sep)
        .hover(p.hover).press_scale(0.9)
        .on_press(move |s: &mut State| s.dark = Some(!dark))
        .child(sun.child(disc).child(bite))
        .label(if s.dark.unwrap_or(dark) { "Switch to light" } else { "Switch to dark" })
}

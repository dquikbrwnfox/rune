//! The masthead: a liquid-brass seal, odometer stats and a sun that turns into a moon.

use std::f32::consts::PI;

use rune::{El, Spring, col, el, row, text};
use rune_kit::{Kit, RollingText};

use crate::model::Sess;
use crate::pal::{H1, LABEL, SMALL, STAT, Pal};
use crate::{State, cmdk, mark};

const ROLL: Spring = Spring::new(0.38, 0.12);

pub fn header(s: &State, p: &Pal, sess: &[Sess], n_clash: usize) -> El {
    let kit = Kit::neutral(p.dark);
    let hours: f32 = sess.iter().map(|x| x.hours).sum::<f32>() + 0.0; // + 0.0 turns -0 into 0
    let roll = |key: &str, v: f64, txt: String, c| RollingText::new(key, txt).value(v).font(STAT).color(c).spring(ROLL).build(&kit);
    let stat = |key: &str, v: f64, num: String, unit: &str, c| {
        row().key(format!("{key}-stat")).gap(5.0).center_y().width_leads().motion(ROLL)
            .child(roll(key, v, num, c))
            .child(text(unit.to_owned()).font(SMALL).color(p.sub))
    };
    let clash_c = if n_clash > 0 { p.danger } else { p.sub };
    row().key("header").w_full().h(60.0).px(20.0).gap(16.0).center_y().fill(p.card)
        .border(1.0, p.sep)
        .child(seal(s, p))
        .child(col().gap(1.0)
            .child(text("Timetable").font(H1).color(p.text))
            .child(text("UWI St. Augustine · Semester I").font(SMALL).color(p.sub)))
        .child(el().grow(1.0))
        .child(stat("courses", s.enr.len() as f64, s.enr.len().to_string(), if s.enr.len() == 1 { "course" } else { "courses" }, p.text))
        .child(dot(p))
        .child(stat("hours", f64::from(hours), format!("{hours:.0}"), "hrs / wk", p.text))
        .child(dot(p))
        .child(stat("clashes", n_clash as f64, n_clash.to_string(), if n_clash == 1 { "clash" } else { "clashes" }, clash_c))
        .child(el().w(8.0))
        .child(search_button(p))
        .child(lamp_button(s, p))
        .child(theme_switch(s, p))
}

fn dot(p: &Pal) -> El { el().size(3.0, 3.0).radius(1.5).fill(p.sep) }

/// The "U" seal: a brass gradient that the compositor turns into flowing liquid metal.
/// Hovering it wakes the metal for a few seconds.
fn seal(s: &State, p: &Pal) -> El {
    el().key("seal").size(34.0, 34.0).radius(10.0).center()
        .linear_gradient(2.4, [(0.0, p.brass.mix(p.raised, 0.35)), (0.55, p.brass), (1.0, p.brass.mix(p.bg, 0.45))])
        .shadow(2.0, 8.0, p.shadow).press_scale(0.92)
        .on_hover_change(|s: &mut State, h, _| if h { s.seal_gen += 1 })
        .child(rune::effect(mark::SEAL).key("seal-fx").effect_data([s.seal_gen as f32, 0.0, 0.0, 0.0])
            .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).radius(10.0).inert())
        .child(text("U").font(H1).color(rune::oklch(0.27, 0.05, 65.0)))
}

/// Lights on or off: a bulb that glows when the scene is lit.
fn lamp_button(s: &State, p: &Pal) -> El {
    let on = s.lights;
    el().key("lamp").size(34.0, 34.0).radius(17.0).center().fill(p.raised).border(1.0, if on { p.brass } else { p.sep })
        .hover(p.hover).press_scale(0.9).motion(Spring::BOUNCY)
        .shadow(0.0, if on { 16.0 } else { 0.0 }, p.brass.alpha(if on { 0.55 } else { 0.0 }))
        .on_press(|s: &mut State| s.lights = !s.lights)
        .label(if on { "Turn lights off" } else { "Turn lights on" })
        .child(col().items_center().gap(1.0)
            .child(el().key("bulb").size(12.0, 12.0).radius(6.0).fill(if on { p.brass } else { p.sub.alpha(0.4) }).scale(if on { 1.0 } else { 0.8 }).motion(Spring::BOUNCY))
            .child(el().size(6.0, 3.0).radius(1.0).fill(p.sub)))
}

fn search_button(p: &Pal) -> El {
    row().key("cmdk-btn").h(32.0).pl(10.0).pr(6.0).gap(10.0).center_y().radius(8.0)
        .fill(p.raised).border(1.0, p.sep).hover(p.hover).press_scale(0.96)
        .child(text("Find a course").font(LABEL).color(p.sub))
        .child(el().h(20.0).px(6.0).center().radius(5.0).fill(p.hover).child(text("/").font(SMALL).color(p.sub)))
        .on_press(|s: &mut State| cmdk::open(s))
}

/// A sun whose rays retract while the disc turns half a revolution and a shadow slides across
/// it into a crescent.
fn theme_switch(s: &State, p: &Pal) -> El {
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

//! What a block says when you tap or click it: the course, type and stream, day and time, rooms,
//! weeks, staff, the other options (tap to switch), the official timetable and removal.

use rune::{El, Tx, col, el, row, text};

use crate::card::{group_control, option_chips};
use crate::data::weeks_label;
use crate::model::{DAYS_LONG, Sess, clash_set, session_of, tm, week_mask};
use crate::pal::{BODY, CODE, LABEL, SMALL, TINY, TITLE, Pal, tone};
use crate::{State, remove};

fn fact(p: &Pal, label: &str, value: String) -> El {
    row().w_full().gap(10.0)
        .child(el().w(60.0).child(text(label.to_owned()).font(TINY).color(p.sub)))
        .child(text(value).font(BODY).color(p.text).grow(1.0).min_w(0.0).wrap())
}

fn button(p: &Pal, key: &str, label: &str, danger: bool, small: bool) -> El {
    let b = el().key(key.to_owned()).h(if small { 44.0 } else { 32.0 }).px(14.0).center().radius(9.0).press_scale(0.95).hover(p.hover);
    if danger { b.border(1.0, p.danger.alpha(0.7)).child(text(label.to_owned()).font(LABEL).color(p.danger)) }
    else { b.fill(p.raised).border(1.0, p.sep).child(text(label.to_owned()).font(LABEL).color(p.text)) }
}

/// The block with this session id, if it's still on the schedule.
pub fn find<'a>(sess: &'a [Sess], id: &str) -> Option<&'a Sess> { sess.iter().find(|x| x.id == id) }

pub fn content(s: &State, p: &Pal, x: &Sess, small: bool) -> El {
    let c = &s.cat[x.course];
    let e = s.enr.iter().find(|e| e.course == x.course);
    let tc = tone(e.map_or(0, |e| e.tone), p.dark);
    let se = session_of(&s.cat, x);
    let published = s.meta.published_mask();
    let stream = if (1..=3).contains(&x.kind) { c.streams(x.kind).get(x.stream) } else { None };
    let what = match x.kind {
        0 if c.groups.len() > 1 => format!("{} · group {}", x.ty.name(), c.group_id(x.stream)),
        1..=3 => format!("{} {}", x.ty.name(), stream.map_or(String::new(), |st| st.chip_text())),
        4 => if x.raw.is_empty() { "Other entry".to_owned() } else { x.raw.clone() },
        _ => x.ty.name().to_owned(),
    };
    let weeks = match se {
        Some(se) if published != 0 && se.weeks & published != published => format!("{} · not every published week", weeks_label(se.weeks)),
        Some(se) => format!("{} · every published week", weeks_label(se.weeks)),
        None => weeks_label(x.weeks),
    };
    let key = x.id.clone();
    let mut body = col().w_full().gap(10.0)
        .child(row().w_full().gap(10.0).items_start()
            .child(el().size(10.0, 10.0).radius(5.0).fill(tc).mt(5.0))
            .child(col().grow(1.0).min_w(0.0).gap(2.0)
                .child(text(c.code.clone()).font(CODE).color(tc))
                .child(text(c.name.clone()).font(TITLE).color(p.text).w_full().wrap())
                .child(text(what).font(SMALL).color(p.sub)))
            .child(el().key("detail-close").size(if small { 44.0 } else { 28.0 }, if small { 44.0 } else { 28.0 }).center().radius(8.0).hover(p.hover).press_scale(0.9)
                .label("Close").child(text("×").font(TITLE).color(p.sub))
                .on_press(|s: &mut State| s.detail = None)));
    if x.kind == 4 {
        body = body.child(text("Not a class: shown for information, and never counted as a clash.").font(SMALL).color(p.sub).w_full().wrap());
    }
    body = body.child(fact(p, "WHEN", format!("{} · {}–{} ({:.1} h)", DAYS_LONG[x.day as usize], tm(x.start), tm(x.end()), x.hours)))
        .child(fact(p, "WHERE", se.map_or_else(|| x.room.clone(), |se| if se.online() && se.rooms.iter().any(|r| !r.eq_ignore_ascii_case("Remote Teaching")) { format!("Online ({})", se.rooms.join("; ")) } else if se.rooms.len() > 1 { se.rooms.join("; ") } else { se.room_text() })))
        .child(fact(p, "WEEKS", weeks));
    if let Some(se) = se {
        if !se.staff.is_empty() { body = body.child(fact(p, "STAFF", se.staff.join(", "))); }
        let others: Vec<&String> = se.cross.iter().filter(|k| **k != c.code).collect();
        if !others.is_empty() { body = body.child(fact(p, "ALSO", format!("Listed under {}", others.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(", ")))); }
    }
    if let Some(e) = e.filter(|_| !crate::locked(s)) {
        if x.kind == 0 && c.groups.len() > 1 { body = body.child(group_control(s, p, e, small)); }
        if (1..=3).contains(&x.kind) {
            let clashes = clash_set(&s.cat, &s.enr, week_mask(s.week));
            let label = format!("OTHER OPTIONS ({})", x.ty.name().to_uppercase());
            if let Some(chips) = option_chips(s, p, e, x.kind, &label, &clashes, small) { body = body.child(chips); }
        }
    }
    let link = c.link.clone();
    let course = x.course;
    let mut actions = row().w_full().gap(8.0).flex_wrap();
    if !link.is_empty() {
        actions = actions.child(button(p, "detail-link", "Official timetable", false, small)
            .on_press_tx(move |_: &mut State, tx: &mut Tx| { let _ = tx.open_url(&link); }));
    }
    if !crate::locked(s) && e.is_some() {
        actions = actions.child(button(p, "detail-remove", "Remove course", true, small).on_press(move |s: &mut State| remove(s, course)));
    }
    col().key(format!("detail-{key}")).w_full().gap(12.0).child(body).child(actions)
}

//! The phone's Today tab: what is next, where and when, then the rest of the day. It is the
//! default tab once there is a schedule. Weekday-aware: a weekend says "No classes today" and
//! names the next class.

use rune::{El, ScrollSpec, Transition, col, el, row, text};

use crate::model::{DAYS_LONG, Sess, tm};
use crate::pal::{BODY, CODE, H1, LABEL, SMALL, TINY, TITLE, Pal, tone};
use crate::{State, open_detail};

/// Class sessions (not entries), with the twin copy of a cross-listed session dropped.
fn classes(sess: &[Sess]) -> Vec<&Sess> {
    let mut out: Vec<&Sess> = vec![];
    for x in sess.iter().filter(|x| x.kind != 4) {
        if x.twin != 0 && out.iter().any(|o| o.twin == x.twin && o.day == x.day && o.start == x.start) { continue; }
        out.push(x);
    }
    out
}

/// Whether the schedule says a class runs in teaching week `week`. A week the data doesn't
/// cover is taken to follow the usual pattern.
fn runs(x: &Sess, week: Option<i64>, published: u64) -> bool {
    match week {
        Some(w) if (0..64).contains(&w) && published >> w & 1 == 1 => x.weeks >> w & 1 == 1,
        _ => true,
    }
}

/// The day's classes in time order, for weekday `day` of teaching week `week`.
pub fn agenda<'a>(sess: &'a [Sess], day: u8, week: Option<i64>, published: u64) -> Vec<&'a Sess> {
    let mut v: Vec<&Sess> = classes(sess).into_iter().filter(|x| x.day == day && runs(x, week, published)).collect();
    v.sort_by(|a, b| a.start.total_cmp(&b.start));
    v
}

/// The next class after `hour` on weekday `day`: today's remaining ones first, then the coming
/// days (wrapping into next week). Returns how many days ahead it is.
pub fn next_class<'a>(sess: &'a [Sess], day: u8, hour: f32, week: Option<i64>, published: u64) -> Option<(u8, &'a Sess)> {
    for ahead in 0..=7u8 {
        let idx = u32::from(day) + u32::from(ahead);
        let (d, w) = ((idx % 7) as u8, week.map(|w| w + i64::from(idx / 7)));
        let next = agenda(sess, d, w, published).into_iter().find(|x| ahead > 0 || x.start > hour);
        if let Some(x) = next { return Some((ahead, x)); }
    }
    None
}

/// "in 25 min", "in 2 h 10 min".
pub fn countdown(mins: i32) -> String {
    match mins {
        m if m <= 0 => "now".to_owned(),
        m if m < 60 => format!("in {m} min"),
        m if m % 60 == 0 => format!("in {} h", m / 60),
        m => format!("in {} h {} min", m / 60, m % 60),
    }
}

fn card_base(p: &Pal) -> El { col().w_full().padding(14.0).gap(6.0).radius(14.0).fill(p.card).border(1.0, p.sep) }

fn row_for(s: &State, p: &Pal, x: &Sess, now_h: f32) -> El {
    let c = &s.cat[x.course];
    let tc = tone(s.enr.iter().find(|e| e.course == x.course).map_or(0, |e| e.tone), p.dark);
    let past = x.end() <= now_h;
    let live = x.start <= now_h && now_h < x.end();
    let id = x.id.clone();
    let stream = if (1..=3).contains(&x.kind) { c.streams(x.kind).get(x.stream).map(|st| format!(" {}", st.chip_text())) } else { None };
    row().key(format!("ag-{}", x.id)).w_full().min_h(60.0).gap(12.0).center_y().px(10.0).py(6.0).radius(12.0)
        .fill(if live { tc.alpha(0.14) } else { p.card }).border(1.0, if live { tc.alpha(0.7) } else { p.sep })
        .opacity(if past { 0.5 } else { 1.0 }).press_scale(0.98)
        .child(col().w(54.0).child(text(tm(x.start)).font(CODE).color(p.text)).child(text(tm(x.end())).font(SMALL).color(p.sub)))
        .child(el().w(4.0).h(40.0).radius(2.0).fill(tc))
        .child(col().grow(1.0).min_w(0.0).gap(1.0)
            .child(text(format!("{} · {}{}", c.code, x.ty.name(), stream.unwrap_or_default())).font(TITLE).color(p.text).truncate())
            .child(text(c.name.clone()).font(SMALL).color(p.sub).truncate())
            .child(text(x.room.clone()).font(SMALL).color(if x.room == "Online" { p.brass } else { p.sub }).truncate()))
        .child(if live { el().h(18.0).px(6.0).center().radius(5.0).fill(tc).child(text("NOW").font(TINY).color(p.bg)) } else { el() })
        .on_press(move |s: &mut State| open_detail(s, id.clone()))
}

pub fn view(s: &State, p: &Pal, sess: &[Sess]) -> El {
    let published = s.meta.published_mask();
    let (day, hour) = s.clock.unwrap_or((0, 8.0));
    let week = s.cur_week;
    let list = agenda(sess, day, week, published);
    let mut c = col().key("today").w_full().padding(12.0).gap(12.0).pb(24.0);
    let week_text = week.filter(|w| *w > 0).map_or(String::new(), |w| format!(" · week {w}"));
    c = c.child(col().w_full().gap(1.0).px(4.0)
        .child(text(format!("{}{week_text}", s.date_text)).font(H1).color(p.text))
        .child(text(format!("{} · {}", s.meta.short_term(), if s.meta.sample { "sample data".to_owned() } else { "unofficial".to_owned() })).font(SMALL).color(p.sub)));
    if let Some(w) = week.filter(|w| *w > 0 && (*w as usize) < 64) {
        if published != 0 && published >> w & 1 == 0 && !s.meta.sample {
            c = c.child(text(format!("CELCAT has not published week {w} yet, so this follows the usual weekly pattern."))
                .font(SMALL).color(p.sub).w_full().wrap().px(4.0));
        }
    }
    if s.enr.is_empty() {
        return c.child(card_base(p).child(text("No schedule yet.").font(TITLE).color(p.text))
            .child(text("Add your courses and today's classes will show here.").font(BODY).color(p.sub).w_full().wrap())
            .child(el().key("today-add").h(44.0).px(14.0).center().radius(10.0).fill(p.brass).press_scale(0.96)
                .child(text("Add courses").font(LABEL).color(p.bg)).on_press(|s: &mut State| s.tab = 2)))
            .scroll(ScrollSpec { stick_bottom: false });
    }
    // Next up: the class under way, else the next one today.
    let live = list.iter().find(|x| x.start <= hour && hour < x.end());
    let upcoming = list.iter().find(|x| x.start > hour);
    let after = next_class(sess, day, hour, week, published);
    c = c.child(match (live, upcoming, after) {
        (Some(x), _, _) => next_card(s, p, x, "NOW", format!("ends {} ({})", tm(x.end()), countdown(((x.end() - hour) * 60.0).ceil() as i32))),
        (None, Some(x), _) => next_card(s, p, x, "NEXT UP", format!("{} · {}", tm(x.start), countdown(((x.start - hour) * 60.0).ceil() as i32))),
        (None, None, Some((ahead, x))) => {
            let head = if list.is_empty() { "No classes today" } else { "No more classes today" };
            let d = DAYS_LONG[((u32::from(day) + u32::from(ahead)) % 7) as usize];
            let label = if ahead == 1 { "TOMORROW".to_owned() } else { format!("NEXT · {}", d.to_uppercase()) };
            col().key("next-none").w_full().gap(10.0)
                .child(card_base(p).child(text(head).font(H1).color(p.text)))
                .child(next_card(s, p, x, &label, format!("{} {}", &d[..3], tm(x.start))))
        }
        (None, None, None) => card_base(p).key("next-none").child(text("No classes today").font(H1).color(p.text))
            .child(text("Nothing on your schedule.").font(BODY).color(p.sub)),
    });
    if !list.is_empty() {
        c = c.child(text("TODAY'S CLASSES").font(TINY).color(p.sub).px(4.0));
        for x in list { c = c.child(row_for(s, p, x, hour)); }
    }
    c.scroll(ScrollSpec { stick_bottom: false })
}

fn next_card(s: &State, p: &Pal, x: &Sess, label: &str, when: String) -> El {
    let c = &s.cat[x.course];
    let tc = tone(s.enr.iter().find(|e| e.course == x.course).map_or(0, |e| e.tone), p.dark);
    let stream = if (1..=3).contains(&x.kind) { c.streams(x.kind).get(x.stream).map(|st| format!(" {}", st.chip_text())) } else { None };
    let id = x.id.clone();
    let walk = if x.room == "Online" { "Online, no walk needed" } else if x.room == "Room TBA" { "Room not announced yet" } else { "" };
    let mut card = card_base(p).key("next-card").fill(tc.alpha(if p.dark { 0.16 } else { 0.1 })).border(1.5, tc.alpha(0.7)).gap(4.0)
        .transition(Transition::opacity())
        .child(row().gap(8.0).center_y().child(el().h(18.0).px(6.0).center().radius(5.0).fill(tc).child(text(label.to_owned()).font(TINY).color(p.bg)))
            .child(text(when).font(LABEL).color(p.text)))
        .child(text(format!("{} · {}{}", c.code, x.ty.name(), stream.unwrap_or_default())).font(H1).color(tc))
        .child(text(c.name.clone()).font(BODY).color(p.text).w_full().wrap())
        .child(text(x.room.clone()).font(TITLE).color(p.text));
    if !walk.is_empty() { card = card.child(text(walk.to_owned()).font(SMALL).color(p.sub)); }
    card.press_scale(0.99).on_press(move |s: &mut State| open_detail(s, id.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::mk;
    use crate::data::{SKind, Session, mask};

    fn sess(day: u8, start: f32, end: f32, weeks: &[u32]) -> Sess {
        let se = Session { day, start, end, kind: SKind::Lecture, raw_type: String::new(), rooms: vec!["R".into()], weeks: mask(weeks.iter().copied()), staff: vec![], cross: vec![], twin: 0 };
        mk(0, 0, 0, &se, 0, 0)
    }

    #[test]
    fn next_class_walks_forward_and_wraps() {
        let all = [sess(0, 9.0, 10.0, &[10, 11]), sess(2, 14.0, 15.0, &[10, 11, 12])];
        let published = mask([10, 11, 12]);
        // Monday 8:00 in week 10: the 9:00 class is next, today.
        assert_eq!(next_class(&all, 0, 8.0, Some(10), published).map(|(a, x)| (a, x.start)), Some((0, 9.0)));
        // Monday 9:30: nothing left today; Wednesday is two days on.
        assert_eq!(next_class(&all, 0, 9.5, Some(10), published).map(|(a, x)| (a, x.day)), Some((2, 2)));
        // Saturday of week 11: next Monday is week 12, which the Monday class doesn't run in, so Wednesday wins.
        assert_eq!(next_class(&all, 5, 10.0, Some(11), published).map(|(a, x)| (a, x.day)), Some((4, 2)));
        // Saturday of week 12: week 13 isn't published, so the usual pattern puts Monday next.
        assert_eq!(next_class(&all, 5, 10.0, Some(12), published).map(|(a, x)| (a, x.day)), Some((2, 0)));
    }

    #[test]
    fn countdown_text() {
        assert_eq!(countdown(0), "now");
        assert_eq!(countdown(25), "in 25 min");
        assert_eq!(countdown(120), "in 2 h");
        assert_eq!(countdown(130), "in 2 h 10 min");
    }

    #[test]
    fn unpublished_weeks_follow_the_pattern() {
        let all = [sess(0, 9.0, 10.0, &[10])];
        assert_eq!(agenda(&all, 0, Some(10), mask([10])).len(), 1);
        assert_eq!(agenda(&all, 0, Some(11), mask([10, 11])).len(), 0);
        assert_eq!(agenda(&all, 0, Some(13), mask([10])).len(), 1);
    }
}

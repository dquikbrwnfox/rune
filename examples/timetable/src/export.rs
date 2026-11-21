//! Export: an iCalendar file with one event per session per teaching week (so partial weeks are
//! exact), and a plain-text summary. Times are America/Port_of_Spain (UTC-4, no daylight saving).

use crate::State;
use crate::data::{Course, Session, Stream, weeks_label};
use crate::model::{DAYS, Enr, clash_pairs, active, tm};

/// A chosen session with its stream, if it belongs to one.
pub struct Chosen<'a> { pub kind: u8, pub stream: Option<&'a Stream>, pub session: &'a Session }

/// The class sessions of one enrolment (lecture group and picked streams; not `other`).
pub fn chosen<'a>(c: &'a Course, e: &Enr) -> Vec<Chosen<'a>> {
    let mut out = vec![];
    if let Some(g) = c.groups.get(e.group) {
        out.extend(g.sessions.iter().map(|session| Chosen { kind: 0, stream: None, session }));
    }
    for kind in 1..=3u8 {
        if let Some(st) = e.pick(kind).and_then(|i| c.streams(kind).get(i)) {
            out.extend(st.sessions.iter().map(|session| Chosen { kind, stream: Some(st), session }));
        }
    }
    out
}

// Days since 1970-01-01 for a civil date, and back (Howard Hinnant's algorithms).
pub(crate) fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub(crate) fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

pub(crate) fn week1_days(iso: &str) -> i64 {
    let mut it = iso.split('-').filter_map(|x| x.parse::<i64>().ok());
    match (it.next(), it.next(), it.next()) {
        (Some(y), Some(m), Some(d)) => days_from_civil(y, m, d),
        _ => days_from_civil(2026, 8, 31),
    }
}

fn stamp(days: i64, hours: f32) -> String {
    let (y, m, d) = civil_from_days(days);
    let mins = (hours * 60.0).round() as i64;
    format!("{y:04}{m:02}{d:02}T{:02}{:02}00", mins / 60, mins % 60)
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace('\n', "\\n")
}

/// Folds a content line at 75 octets, as RFC 5545 asks.
fn fold(line: &str, out: &mut String) {
    let mut n = 0;
    for ch in line.chars() {
        let w = ch.len_utf8();
        if n + w > 74 { out.push_str("\r\n "); n = 1; }
        out.push(ch);
        n += w;
    }
    out.push_str("\r\n");
}

pub fn ics(s: &State) -> String {
    let base = week1_days(&s.meta.week1_monday);
    let now = crate::load::now_ms();
    let dtstamp = format!("{}Z", stamp((now / 86_400_000.0) as i64, ((now % 86_400_000.0) / 3_600_000.0) as f32));
    let mut out = String::new();
    for l in ["BEGIN:VCALENDAR", "VERSION:2.0", "PRODID:-//UWI Timetable (unofficial)//EN", "CALSCALE:GREGORIAN", "METHOD:PUBLISH",
        "X-WR-CALNAME:UWI Timetable", "X-WR-TIMEZONE:America/Port_of_Spain",
        "BEGIN:VTIMEZONE", "TZID:America/Port_of_Spain", "BEGIN:STANDARD", "DTSTART:19700101T000000",
        "TZOFFSETFROM:-0400", "TZOFFSETTO:-0400", "TZNAME:AST", "END:STANDARD", "END:VTIMEZONE"] {
        fold(l, &mut out);
    }
    for e in &s.enr {
        let Some(c) = s.cat.get(e.course) else { continue };
        for (n, ch) in chosen(c, e).iter().enumerate() {
            let se = ch.session;
            let what = match ch.kind { 1 => "Tutorial", 2 => "Lab", 3 => "Field trip", _ => se.kind.name() };
            let mut desc = vec![c.name.clone()];
            if let Some(st) = ch.stream { desc.push(format!("Stream: {}", st.label)); }
            desc.push(format!("Weeks: {}", weeks_label(se.weeks)));
            if !se.staff.is_empty() { desc.push(format!("Staff: {}", se.staff.join("; "))); }
            if !c.link.is_empty() { desc.push(format!("Official timetable: {}", c.link)); }
            desc.push("Unofficial: check the official CELCAT timetable.".into());
            for w in (1..64u32).filter(|w| se.weeks >> w & 1 == 1) {
                let day = base + i64::from(w - 1) * 7 + i64::from(se.day);
                for l in [
                    "BEGIN:VEVENT".to_owned(),
                    format!("UID:{}-{}-{n}-w{w}@uwi-timetable", c.id, ch.kind),
                    format!("DTSTAMP:{dtstamp}"),
                    format!("DTSTART;TZID=America/Port_of_Spain:{}", stamp(day, se.start)),
                    format!("DTEND;TZID=America/Port_of_Spain:{}", stamp(day, se.end)),
                    format!("SUMMARY:{}", esc(&format!("{} {what}", c.code))),
                    if se.rooms.is_empty() { String::new() } else { format!("LOCATION:{}", esc(&se.rooms.join("; "))) },
                    format!("DESCRIPTION:{}", esc(&desc.join("\n"))),
                    "END:VEVENT".to_owned(),
                ] {
                    if !l.is_empty() { fold(&l, &mut out); }
                }
            }
        }
    }
    fold("END:VCALENDAR", &mut out);
    out
}

pub fn download_ics(s: &State) { crate::host::download("uwi-timetable.ics", "text/calendar", &ics(s)); }

/// A plain-text version of the week, for pasting.
pub fn summary_text(s: &State) -> String {
    let mut out = format!("My UWI timetable, {} (unofficial)\n", s.meta.term_label);
    let hours: f32 = active(&s.cat, &s.enr).iter().filter(|x| x.kind != 4).map(|x| x.hours).sum();
    for e in &s.enr {
        let Some(c) = s.cat.get(e.course) else { continue };
        out.push_str(&format!("\n{} {}\n", c.code, c.name));
        for ch in chosen(c, e) {
            let se = ch.session;
            let what = match ch.stream { Some(st) => format!("{} {}", se.kind.name(), st.label), None => se.kind.name().to_owned() };
            out.push_str(&format!("  {what}: {} {}–{}, {} ({})\n", DAYS[se.day as usize], tm(se.start), tm(se.end), se.room_text(), weeks_label(se.weeks)));
        }
        for ex in &c.exams {
            out.push_str(&format!("  Exam: {} {}–{}, {} ({})\n", DAYS[ex.day as usize], tm(ex.start), tm(ex.end), ex.room_text(), weeks_label(ex.weeks)));
        }
    }
    let n = clash_pairs(&active(&s.cat, &s.enr), u64::MAX).len();
    out.push_str(&format!("\n{} courses, {hours:.0} contact hours a week, {}\n", s.enr.len(), if n == 0 { "no clashes".to_owned() } else { format!("{n} clash(es)") }));
    out
}

/// Days since 1970-01-01 for an ISO date "YYYY-MM-DD".
pub(crate) fn days_from_civil_iso(iso: &str) -> Option<i64> {
    let mut it = iso.split('-').filter_map(|x| x.parse::<i64>().ok());
    Some(days_from_civil(it.next()?, it.next()?, it.next()?))
}

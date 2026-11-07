use crate::data::{self, Course, SKind, Session, mask};
use crate::model::{Enr, active, clash_pairs, clashes, mk};
use crate::solve::{Prefs, solve};
use crate::{State, export, load, store};

fn ses(day: u8, start: f32, end: f32, weeks: &[u32]) -> Session {
    Session { day, start, end, kind: SKind::Lecture, raw_type: String::new(), rooms: vec![], weeks: mask(weeks.iter().copied()), staff: vec![], cross: vec![], twin: 0 }
}

#[test]
fn clash_needs_overlapping_time_and_weeks() {
    let a = mk(0, 0, 0, &ses(0, 9.0, 11.0, &[10, 11]), 0, 0);
    let same = mk(1, 0, 0, &ses(0, 10.0, 12.0, &[11, 12]), 0, 0);
    let other_weeks = mk(1, 0, 0, &ses(0, 10.0, 12.0, &[12]), 0, 0);
    let touching = mk(1, 0, 0, &ses(0, 11.0, 12.0, &[10]), 0, 0);
    let other_day = mk(1, 0, 0, &ses(1, 10.0, 12.0, &[10]), 0, 0);
    assert!(clashes(&a, &same));
    assert!(!clashes(&a, &other_weeks));
    assert!(!clashes(&a, &touching));
    assert!(!clashes(&a, &other_day));
}

#[test]
fn cross_listed_twins_do_not_clash_but_other_entries_never_do() {
    let mut x = ses(0, 9.0, 11.0, &[10]);
    x.twin = 7;
    let a = mk(0, 0, 0, &x, 0, 0);
    let b = mk(1, 0, 0, &x, 0, 0);
    assert!(!clashes(&a, &b));
    let mut o = mk(1, 4, 0, &ses(0, 9.0, 11.0, &[10]), 0, 0);
    o.twin = 0;
    assert!(!clashes(&a, &o));
}

#[test]
fn solver_clears_sample_clashes() {
    let cat = data::sample().courses;
    // (INFO 1600 and ECON 1000 have fixed lectures that overlap on Thursday, so they are left out.)
    let mut enr: Vec<Enr> = [0usize, 1, 2, 4, 6].into_iter().map(|i| Enr { id: cat[i].id.clone(), course: i, tone: i, group: 0, picks: [Some(0); 3] }).collect();
    for e in &mut enr { for k in 1..=3u8 { if cat[e.course].streams(k).is_empty() { e.set_pick(k, None); } } }
    let sol = solve(&cat, &enr, Prefs::default(), None);
    for ch in &sol.choices { let e = enr.iter_mut().find(|e| e.course == ch.course).unwrap(); e.group = ch.group; e.picks = ch.picks; }
    assert_eq!(sol.clashes, clash_pairs(&active(&cat, &enr), u64::MAX).len());
    assert_eq!(sol.clashes, 0, "the sample catalogue has a clash-free week");
}

#[test]
fn ics_and_share_round_trip_on_sample() {
    let mut s = State::default();
    let cat = data::sample();
    s.cat = cat.courses;
    s.meta = cat.meta;
    for i in 0..3 { crate::add(&mut s, i); }
    let ics = export::ics(&s);
    eprintln!("{}", ics.lines().skip(14).take(14).collect::<Vec<_>>().join("
"));
    assert!(ics.starts_with("BEGIN:VCALENDAR\r\n") && ics.contains("SUMMARY:COMP 1601 Lecture") && ics.contains("DTSTART;TZID=America/Port_of_Spain:20260831T090000"));
    let url = store::share_url(&s);
    let sh = store::parse_share(url.split('#').nth(1).unwrap()).unwrap();
    let (enr, missing) = store::enrol(&s.cat, &sh.courses);
    assert!(missing.is_empty());
    assert_eq!(enr.len(), 3);
    assert_eq!(enr[0].picks, s.enr[0].picks);
    assert!(export::summary_text(&s).contains("COMP 1601"));
}

/// With `TIMETABLE_DATA` set: parse timing, then the solver on the courses with the most streams.
#[test]
fn real_data_solver_timing() {
    let Ok(path) = std::env::var("TIMETABLE_DATA") else { return };
    let text = std::fs::read_to_string(path).unwrap();
    let t0 = load::now_ms();
    let cat = load::parse(&text).unwrap();
    eprintln!("parse: {:.1} ms (native, debug), {} courses", load::now_ms() - t0, cat.courses.len());
    let mut idx: Vec<usize> = (0..cat.courses.len()).collect();
    let weight = |c: &Course| c.tutorials.len() + c.labs.len() + c.field_trips.len() + c.groups.len();
    idx.sort_by_key(|&i| std::cmp::Reverse(weight(&cat.courses[i])));
    for n in [4usize, 8, 12] {
        let enr: Vec<Enr> = idx.iter().take(n).enumerate().map(|(t, &i)| Enr { id: cat.courses[i].id.clone(), course: i, tone: t, group: 0, picks: [None; 3] }).collect();
        let streams: usize = enr.iter().map(|e| weight(&cat.courses[e.course])).sum();
        let sol = solve(&cat.courses, &enr, Prefs { free_day: true, no_early: true, no_late: true, compact: true }, None);
        eprintln!("{n} heaviest courses ({streams} options): clashes {}, nodes {}, {:.1} ms, capped {}", sol.clashes, sol.nodes, sol.ms, sol.capped);
    }
}

#[test]
fn odd_codes_keep_the_full_code_and_clean_the_name() {
    assert_eq!(load::fix_code_name("PSYC 6013", "/7013, Advanced Statistics & Research Methods in Psychology"),
        ("PSYC 6013/7013".to_owned(), "Advanced Statistics & Research Methods in Psychology".to_owned()));
    assert_eq!(load::fix_code_name("GEOG 4010", "/5010/6010, Stakeholder Engagement"), ("GEOG 4010/5010/6010".to_owned(), "Stakeholder Engagement".to_owned()));
    assert_eq!(load::fix_code_name("PSYC 6102", "/ 7002, Applied Developmental Psychology Seminars").0, "PSYC 6102/7002");
    assert_eq!(load::fix_code_name("COMP 1601", "Computer Programming I"), ("COMP 1601".to_owned(), "Computer Programming I".to_owned()));
}

#[test]
fn week_filter_limits_clashes_to_that_week() {
    use crate::model::week_mask;
    let a = mk(0, 0, 0, &ses(0, 9.0, 11.0, &[10, 11]), 0, 0);
    let b = mk(1, 0, 0, &ses(0, 10.0, 12.0, &[11, 12]), 0, 0);
    let sess = [a, b];
    assert_eq!(clash_pairs(&sess, week_mask(None)).len(), 1);
    assert_eq!(clash_pairs(&sess, week_mask(Some(11))).len(), 1);
    assert_eq!(clash_pairs(&sess, week_mask(Some(10))).len(), 0);
    assert_eq!(clash_pairs(&sess, week_mask(Some(12))).len(), 0);
}

#[test]
fn grid_range_covers_the_schedule_and_does_not_jitter_on_hover() {
    use crate::model::{Ghost, end, set_range, start};
    let early = mk(0, 0, 0, &ses(0, 8.0, 10.0, &[10]), 0, 0);
    let late_ghost = Ghost { sess: mk(0, 1, 1, &ses(1, 19.0, 20.0, &[10]), 0, 0), clash: false };
    set_range(&[early.clone()], &[], 1);
    let base = (start(), end());
    assert_eq!(base, (8.0, 16.0));
    // The focused course's ghost needs 19-20h: the range grows to hold it...
    set_range(&[early.clone()], &[late_ghost], 1);
    assert_eq!((start(), end()), (8.0, 20.0));
    // ...and stays while the schedule is unchanged, even when focus moves to a course with no ghosts.
    set_range(&[early.clone()], &[], 1);
    assert_eq!((start(), end()), (8.0, 20.0));
    // A real change to the schedule resets it.
    set_range(&[early], &[], 2);
    assert_eq!((start(), end()), base);
}

#[test]
fn ics_has_one_event_per_running_week() {
    let mut s = State::default();
    let mut cat = data::sample();
    // COMP 1601's first lecture runs in weeks 10 and 12 only.
    cat.courses[0].groups[0].sessions[0].weeks = mask([10, 12]);
    s.cat = cat.courses;
    s.meta = cat.meta;
    s.meta.week1_monday = "2026-08-31".into();
    crate::add(&mut s, 0);
    let ics = export::ics(&s);
    let starts: Vec<&str> = ics.lines().filter(|l| l.starts_with("DTSTART") && l.ends_with("T090000")).collect();
    // Monday 9:00 of week 10 is 2026-11-02 and week 12 is 2026-11-16; week 11 (2026-11-09) is skipped.
    assert!(starts.iter().any(|l| l.ends_with("20261102T090000")), "{starts:?}");
    assert!(starts.iter().any(|l| l.ends_with("20261116T090000")), "{starts:?}");
    assert!(!starts.iter().any(|l| l.ends_with("20261109T090000")), "{starts:?}");
    assert!(ics.contains("LOCATION:FST 201") && ics.contains("END:VCALENDAR\r\n"));
}

#[test]
fn viewing_a_share_is_read_only_and_keep_mine_restores() {
    let mut s = State::default();
    let cat = data::sample();
    s.cat = cat.courses;
    s.meta = cat.meta;
    s.status = load::Status::Ready;
    s.restored = true;
    crate::add(&mut s, 0);
    let mine = s.enr.len();
    s.pending_share = Some(store::Share { term: s.meta.term.clone(), courses: vec![store::SavedCourse { id: s.cat[1].id.clone(), code: s.cat[1].code.clone(), ..Default::default() }] });
    store::view_share(&mut s);
    assert!(crate::locked(&s) && s.enr.len() == 1 && s.enr[0].course == 1);
    crate::add(&mut s, 2);
    crate::remove(&mut s, 1);
    assert_eq!(s.enr.len(), 1, "edits are refused while viewing");
    store::keep_mine(&mut s);
    assert!(!crate::locked(&s) && s.enr.len() == mine && s.enr[0].course == 0);
}

#[test]
fn free_days_and_contact_hours() {
    let cat = data::sample().courses;
    let mut s = State::default();
    s.cat = cat;
    crate::add(&mut s, 0);
    let sess = active(&s.cat, &s.enr);
    assert_eq!(crate::model::free_days_text(&[]), "");
    assert!(crate::model::free_days_text(&sess).ends_with("free"));
    // COMP 1601: 3 h of lectures plus one 1 h tutorial and one 2 h lab.
    assert_eq!(s.cat[0].contact_hours(), 6.0);
}

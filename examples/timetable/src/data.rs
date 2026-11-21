//! The catalogue model, plus the hand-made sample used as the offline fallback.
//! The real data comes from CELCAT through `load.rs`.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SKind { Lecture, Tutorial, Lab, FieldTrip, Seminar, Other, Exam }

impl SKind {
    pub fn name(self) -> &'static str {
        match self {
            SKind::Lecture => "Lecture", SKind::Tutorial => "Tutorial", SKind::Lab => "Lab", SKind::FieldTrip => "Field trip",
            SKind::Seminar => "Seminar", SKind::Other => "Other", SKind::Exam => "Exam",
        }
    }
    /// Three-letter block tag.
    pub fn tag(self) -> &'static str {
        match self {
            SKind::Lecture => "LEC", SKind::Tutorial => "TUT", SKind::Lab => "LAB", SKind::FieldTrip => "FLD",
            SKind::Seminar => "SEM", SKind::Other => "OTH", SKind::Exam => "EXM",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Session {
    pub day: u8,
    pub start: f32,
    pub end: f32,
    pub kind: SKind,
    pub raw_type: String,
    /// Empty means "Room TBA".
    pub rooms: Vec<String>,
    /// Bit `w` set when the session runs in teaching week `w` (0..=63).
    pub weeks: u64,
    pub staff: Vec<String>,
    /// Course codes this session is also listed under (the whole set, own code included).
    pub cross: Vec<String>,
    /// Non-zero when cross-listed: a hash of day, time, rooms and the cross-listing set, so the
    /// twin copies under two courses compare equal.
    pub twin: u64,
}

impl Session {
    pub fn hours(&self) -> f32 { self.end - self.start }
    /// Every room is a remote-teaching one ("Remote Teaching", "Remote Teaching Remedial").
    pub fn online(&self) -> bool {
        !self.rooms.is_empty() && self.rooms.iter().all(|r| { let r = r.to_lowercase(); r.starts_with("remote teaching") || r.contains("online") })
    }
    /// "Room TBA", "Online" or the rooms joined.
    pub fn room_text(&self) -> String {
        if self.rooms.is_empty() { return "Room TBA".into(); }
        if self.online() { return "Online".into(); }
        self.rooms.join("; ")
    }
}

#[derive(Clone, Debug)]
pub struct Group { pub id: String, pub sessions: Vec<Session> }

#[derive(Clone, Debug)]
pub struct Stream {
    pub id: String,
    /// Human label: "T1", or "Tue 9am" for a synthetic time option.
    pub label: String,
    pub kind: SKind,
    pub parent_group: Option<String>,
    pub synthetic: bool,
    pub sessions: Vec<Session>,
}

#[derive(Clone, Debug)]
pub struct Course {
    pub id: String,
    pub code: String,
    pub name: String,
    pub dept: String,
    pub faculty: String,
    pub link: String,
    pub groups: Vec<Group>,
    pub tutorials: Vec<Stream>,
    pub labs: Vec<Stream>,
    pub field_trips: Vec<Stream>,
    pub other: Vec<Session>,
    pub exams: Vec<Session>,
}

impl Course {
    /// Streams of a kind: 1 tutorial, 2 lab, 3 field trip.
    pub fn streams(&self, kind: u8) -> &[Stream] {
        match kind { 1 => &self.tutorials, 2 => &self.labs, 3 => &self.field_trips, _ => &[] }
    }
    pub fn group_id(&self, g: usize) -> &str { self.groups.get(g).map_or("common", |x| x.id.as_str()) }
    /// Whether a stream can be taken with lecture group `g`.
    pub fn stream_ok(&self, st: &Stream, g: usize) -> bool {
        st.parent_group.as_deref().is_none_or(|p| p == self.group_id(g))
    }
    pub fn lecture_hours(&self) -> f32 { self.groups.first().map_or(0.0, |g| g.sessions.iter().map(Session::hours).sum()) }
}

#[derive(Clone, Debug, Default)]
pub struct Meta {
    pub term: String,
    pub term_label: String,
    pub generated: String,
    pub published_at: String,
    pub weeks_published: Vec<u32>,
    /// Inferred Monday of teaching week 1, "YYYY-MM-DD".
    pub week1_monday: String,
    pub source: String,
    pub sample: bool,
}

impl Meta {
    /// "weeks 10–12", "week 11", "weeks 1–13" or "weeks 3, 5, 9".
    pub fn weeks_text(&self) -> String {
        let w = &self.weeks_published;
        match w.len() {
            0 => String::new(),
            1 => format!("week {}", w[0]),
            n if w[n - 1] - w[0] + 1 == n as u32 => format!("weeks {}–{}", w[0], w[n - 1]),
            _ => format!("weeks {}", w.iter().map(u32::to_string).collect::<Vec<_>>().join(", ")),
        }
    }
}

pub struct Catalogue { pub courses: Vec<Course>, pub meta: Meta }

/// "W3–12" style text for a week mask.
pub fn weeks_label(mask: u64) -> String {
    let mut out: Vec<String> = vec![];
    let mut w = 0u32;
    while w < 64 {
        if mask >> w & 1 == 1 {
            let s = w;
            while w + 1 < 64 && mask >> (w + 1) & 1 == 1 { w += 1; }
            out.push(if s == w { format!("{s}") } else { format!("{s}–{w}") });
        }
        w += 1;
    }
    if out.is_empty() { return "no weeks".into(); }
    format!("W{}", out.join(", "))
}

pub fn mask(weeks: impl IntoIterator<Item = u32>) -> u64 {
    weeks.into_iter().filter(|w| *w < 64).fold(0, |m, w| m | 1u64 << w)
}

// ───────── Sample catalogue (offline fallback) ─────────

const ALL_WEEKS: std::ops::RangeInclusive<u32> = 1..=13;

fn ses(day: u8, start: f32, hours: f32, room: &str, kind: SKind) -> Session {
    Session {
        day, start, end: start + hours, kind, raw_type: String::new(),
        rooms: vec![room.to_owned()], weeks: mask(ALL_WEEKS), staff: vec![], cross: vec![], twin: 0,
    }
}

fn lec(day: u8, start: f32, hours: f32, room: &str) -> Session { ses(day, start, hours, room, SKind::Lecture) }

fn stream(id: &str, kind: SKind, day: u8, start: f32, hours: f32, room: &str) -> Stream {
    Stream { id: id.into(), label: id.into(), kind, parent_group: None, synthetic: false, sessions: vec![ses(day, start, hours, room, kind)] }
}
fn tut(id: &str, day: u8, start: f32, hours: f32, room: &str) -> Stream { stream(id, SKind::Tutorial, day, start, hours, room) }
fn lab(id: &str, day: u8, start: f32, hours: f32, room: &str) -> Stream { stream(id, SKind::Lab, day, start, hours, room) }

fn course(code: &str, name: &str, dept: &str, lectures: Vec<Session>, tutorials: Vec<Stream>, labs: Vec<Stream>) -> Course {
    Course {
        id: format!("sample-{}", code.replace(' ', "")), code: code.into(), name: name.into(), dept: dept.into(),
        faculty: "Sample data".into(), link: String::new(),
        groups: vec![Group { id: "common".into(), sessions: lectures }],
        tutorials, labs, field_trips: vec![], other: vec![], exams: vec![],
    }
}

pub fn sample() -> Catalogue {
    let courses = vec![
        course("COMP 1601", "Computer Programming I", "Computing", vec![lec(0, 9.0, 2.0, "FST 201"), lec(2, 9.0, 1.0, "FST 201")],
            vec![tut("T1", 1, 11.0, 1.0, "FST 114"), tut("T2", 3, 13.0, 1.0, "FST 114"), tut("T3", 4, 10.0, 1.0, "FST 116")],
            vec![lab("LB1", 1, 14.0, 2.0, "Lab 3"), lab("LB2", 3, 8.0, 2.0, "Lab 3"), lab("LB3", 4, 14.0, 2.0, "Lab 4")]),
        course("COMP 1602", "Computer Programming II", "Computing", vec![lec(1, 9.0, 2.0, "FST 202"), lec(3, 10.0, 1.0, "FST 202")],
            vec![tut("T1", 0, 12.0, 1.0, "FST 114"), tut("T2", 2, 14.0, 1.0, "FST 116")],
            vec![lab("LB1", 2, 10.0, 2.0, "Lab 4"), lab("LB2", 4, 8.0, 2.0, "Lab 4")]),
        course("MATH 1141", "Introductory Calculus", "Mathematics", vec![lec(0, 11.0, 1.0, "FST 301"), lec(2, 11.0, 1.0, "FST 301"), lec(4, 11.0, 1.0, "FST 301")],
            vec![tut("T1", 1, 10.0, 1.0, "FST 120"), tut("T2", 1, 15.0, 1.0, "FST 120"), tut("T3", 3, 11.0, 1.0, "FST 120")], vec![]),
        course("INFO 1600", "Intro to IT Concepts", "Computing", vec![lec(3, 15.0, 2.0, "FST 205")],
            vec![tut("T1", 0, 15.0, 1.0, "FST 118"), tut("T2", 2, 16.0, 1.0, "FST 118")],
            vec![lab("LB1", 4, 12.0, 2.0, "Lab 2"), lab("LB2", 0, 17.0, 2.0, "Lab 2")]),
        course("FOUN 1101", "Caribbean Civilisation", "Foundation", vec![lec(0, 13.0, 2.0, "LRC"), lec(2, 13.0, 1.0, "LRC")],
            vec![tut("T1", 1, 12.0, 1.0, "HUM 12"), tut("T2", 4, 9.0, 1.0, "HUM 12"), tut("T3", 3, 9.0, 1.0, "HUM 14")], vec![]),
        course("FOUN 1013", "Science, Medicine & Technology", "Foundation", vec![lec(2, 15.0, 2.0, "LRC")],
            vec![tut("T1", 1, 9.0, 1.0, "HUM 14"), tut("T2", 4, 13.0, 1.0, "HUM 14")], vec![]),
        course("PHYS 1241", "Mechanics, Waves & Thermo", "Physics", vec![lec(1, 13.0, 1.0, "FST 110"), lec(3, 9.0, 1.0, "FST 110")],
            vec![tut("T1", 0, 10.0, 1.0, "FST 112"), tut("T2", 2, 12.0, 1.0, "FST 112")],
            vec![lab("LB1", 1, 15.0, 3.0, "Physics Lab 1"), lab("LB2", 3, 14.0, 3.0, "Physics Lab 1")]),
        course("ECON 1000", "Introduction to Economics", "Economics", vec![lec(0, 16.0, 2.0, "SAL 1"), lec(3, 16.0, 1.0, "SAL 1")],
            vec![tut("T1", 2, 9.0, 1.0, "SAL 8"), tut("T2", 4, 15.0, 1.0, "SAL 8")], vec![]),
    ];
    Catalogue {
        courses,
        meta: Meta {
            term: "sample".into(), term_label: "Sample data".into(), generated: String::new(), published_at: String::new(),
            weeks_published: ALL_WEEKS.collect(), week1_monday: "2026-08-31".into(), source: String::new(), sample: true,
        },
    }
}

// ───────── Display helpers ─────────

impl Session {
    /// "Tue 9am–11am".
    pub fn when_text(&self) -> String {
        format!("{} {}–{}", crate::model::DAYS[self.day.min(6) as usize], crate::model::tm(self.start), crate::model::tm(self.end))
    }
}

impl Stream {
    /// What a chip says: "T1", or "Tue 9am · FST C3" for a time option with a known room.
    pub fn chip_text(&self) -> String {
        if !self.synthetic || self.label.contains('·') { return self.label.clone(); }
        match self.sessions.first().map(Session::room_text) {
            Some(r) if r != "Room TBA" => format!("{} · {r}", self.label),
            _ => self.label.clone(),
        }
    }
    /// One line about the option: "Tue 3pm–4pm · FST C3 · W11" (several sessions are joined).
    pub fn detail_text(&self, published: u64) -> String {
        self.sessions.iter().map(|se| {
            let wk = if published != 0 && se.weeks & published != published { format!(" · {}", weeks_label(se.weeks)) } else { String::new() };
            format!("{} · {}{wk}", se.when_text(), se.room_text())
        }).collect::<Vec<_>>().join("  +  ")
    }
    /// Whether any session runs only some of the published weeks.
    pub fn partial(&self, published: u64) -> bool { published != 0 && self.sessions.iter().any(|se| se.weeks & published != published) }
}

impl Course {
    /// Class hours in a typical week: the lectures plus one stream of each kind.
    pub fn contact_hours(&self) -> f32 {
        let streams: f32 = (1..=3u8).map(|k| self.streams(k).first().map_or(0.0, |st| st.sessions.iter().map(Session::hours).sum())).sum();
        self.lecture_hours() + streams
    }
    /// Weekdays (Mon = 0) with a lecture.
    pub fn lecture_days(&self) -> [bool; 7] {
        let mut d = [false; 7];
        for se in self.groups.iter().flat_map(|g| g.sessions.iter()) { d[se.day.min(6) as usize] = true; }
        d
    }
}

impl Meta {
    /// "7 Nov 2026" from the ISO timestamp the data was generated at.
    pub fn generated_text(&self) -> String {
        const M: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
        let date = self.generated.split('T').next().unwrap_or("");
        let mut it = date.split('-').filter_map(|x| x.parse::<usize>().ok());
        match (it.next(), it.next(), it.next()) {
            (Some(y), Some(m), Some(d)) if (1..=12).contains(&m) => format!("{d} {} {y}", M[m - 1]),
            _ => String::new(),
        }
    }
    /// "Sem 1 · 2026/27" for tight spaces.
    pub fn short_term(&self) -> String {
        if self.sample { return "Sample data".to_owned(); }
        let l = self.term_label.replace("Semester ", "Sem ").replace(", ", " · ");
        match l.rsplit_once('/') {
            Some((a, b)) if b.len() == 4 => format!("{a}/{}", &b[2..]),
            _ => l,
        }
    }
    /// A bit mask of the published weeks.
    pub fn published_mask(&self) -> u64 { mask(self.weeks_published.iter().copied()) }
}

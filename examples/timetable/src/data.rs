//! Sample catalogue, mirroring the React app's mock data until the CELCAT worker lands.

#[derive(Clone, Copy)]
pub struct Slot { pub day: u8, pub start: f32, pub hours: f32, pub room: &'static str }

pub struct Stream { pub id: &'static str, pub slots: Vec<Slot> }

pub struct Course {
    pub code: &'static str,
    pub name: &'static str,
    pub lectures: Vec<Slot>,
    pub tutorials: Vec<Stream>,
    pub labs: Vec<Stream>,
}

const fn s(day: u8, start: f32, hours: f32, room: &'static str) -> Slot { Slot { day, start, hours, room } }
fn st(id: &'static str, day: u8, start: f32, hours: f32, room: &'static str) -> Stream {
    Stream { id, slots: vec![s(day, start, hours, room)] }
}

pub fn catalogue() -> Vec<Course> {
    vec![
        Course { code: "COMP 1601", name: "Computer Programming I",
            lectures: vec![s(0, 9.0, 2.0, "FST 201"), s(2, 9.0, 1.0, "FST 201")],
            tutorials: vec![st("T1", 1, 11.0, 1.0, "FST 114"), st("T2", 3, 13.0, 1.0, "FST 114"), st("T3", 4, 10.0, 1.0, "FST 116")],
            labs: vec![st("LB1", 1, 14.0, 2.0, "Lab 3"), st("LB2", 3, 8.0, 2.0, "Lab 3"), st("LB3", 4, 14.0, 2.0, "Lab 4")] },
        Course { code: "COMP 1602", name: "Computer Programming II",
            lectures: vec![s(1, 9.0, 2.0, "FST 202"), s(3, 10.0, 1.0, "FST 202")],
            tutorials: vec![st("T1", 0, 12.0, 1.0, "FST 114"), st("T2", 2, 14.0, 1.0, "FST 116")],
            labs: vec![st("LB1", 2, 10.0, 2.0, "Lab 4"), st("LB2", 4, 8.0, 2.0, "Lab 4")] },
        Course { code: "MATH 1141", name: "Introductory Calculus",
            lectures: vec![s(0, 11.0, 1.0, "FST 301"), s(2, 11.0, 1.0, "FST 301"), s(4, 11.0, 1.0, "FST 301")],
            tutorials: vec![st("T1", 1, 10.0, 1.0, "FST 120"), st("T2", 1, 15.0, 1.0, "FST 120"), st("T3", 3, 11.0, 1.0, "FST 120")],
            labs: vec![] },
        Course { code: "INFO 1600", name: "Intro to IT Concepts",
            lectures: vec![s(3, 15.0, 2.0, "FST 205")],
            tutorials: vec![st("T1", 0, 15.0, 1.0, "FST 118"), st("T2", 2, 16.0, 1.0, "FST 118")],
            labs: vec![st("LB1", 4, 12.0, 2.0, "Lab 2"), st("LB2", 0, 17.0, 2.0, "Lab 2")] },
        Course { code: "FOUN 1101", name: "Caribbean Civilisation",
            lectures: vec![s(0, 13.0, 2.0, "LRC"), s(2, 13.0, 1.0, "LRC")],
            tutorials: vec![st("T1", 1, 12.0, 1.0, "HUM 12"), st("T2", 4, 9.0, 1.0, "HUM 12"), st("T3", 3, 9.0, 1.0, "HUM 14")],
            labs: vec![] },
        Course { code: "FOUN 1013", name: "Science, Medicine & Technology",
            lectures: vec![s(2, 15.0, 2.0, "LRC")],
            tutorials: vec![st("T1", 1, 9.0, 1.0, "HUM 14"), st("T2", 4, 13.0, 1.0, "HUM 14")],
            labs: vec![] },
        Course { code: "PHYS 1241", name: "Mechanics, Waves & Thermo",
            lectures: vec![s(1, 13.0, 1.0, "FST 110"), s(3, 9.0, 1.0, "FST 110")],
            tutorials: vec![st("T1", 0, 10.0, 1.0, "FST 112"), st("T2", 2, 12.0, 1.0, "FST 112")],
            labs: vec![st("LB1", 1, 15.0, 3.0, "Physics Lab 1"), st("LB2", 3, 14.0, 3.0, "Physics Lab 1")] },
        Course { code: "ECON 1000", name: "Introduction to Economics",
            lectures: vec![s(0, 16.0, 2.0, "SAL 1"), s(3, 16.0, 1.0, "SAL 1")],
            tutorials: vec![st("T1", 2, 9.0, 1.0, "SAL 8"), st("T2", 4, 15.0, 1.0, "SAL 8")],
            labs: vec![] },
    ]
}

//! Course search over the whole catalogue: code with or without the space, name words, and
//! department or faculty. Code matches rank first, then names, then the department.

use crate::data::Course;

pub const LIMIT: usize = 30;

fn norm(s: &str) -> String { s.chars().filter(|c| !c.is_whitespace()).flat_map(char::to_lowercase).collect() }

fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase).collect()
}

/// The codes a course answers to: `PSYC 6013/7013` is also `PSYC 7013`.
fn codes(c: &Course) -> Vec<String> {
    let mut out = vec![norm(&c.code)];
    if let Some((first, rest)) = c.code.split_once('/') {
        let letters: String = first.chars().take_while(|ch| ch.is_alphabetic()).collect();
        for part in rest.split('/') { out.push(norm(&format!("{letters}{part}"))); }
    }
    out
}

/// Rank of a course for a query: lower is better, `None` is no match.
fn rank(c: &Course, q: &str, toks: &[String]) -> Option<u8> {
    let codes = codes(c);
    if codes.iter().any(|x| x == q) { return Some(0); }
    if codes.iter().any(|x| x.starts_with(q)) { return Some(1); }
    if codes.iter().any(|x| x.contains(q)) { return Some(2); }
    let lname = c.name.to_lowercase();
    if lname.starts_with(&toks.join(" ")) && !toks.is_empty() { return Some(3); }
    let name = words(&c.name);
    if !toks.is_empty() && toks.iter().all(|t| name.iter().any(|w| w.starts_with(t.as_str()))) { return Some(4); }
    if !toks.is_empty() && toks.iter().all(|t| lname.contains(t.as_str())) { return Some(5); }
    let meta = format!("{} {}", c.dept, c.faculty).to_lowercase();
    if !toks.is_empty() && toks.iter().all(|t| meta.contains(t.as_str()) || lname.contains(t.as_str())) { return Some(6); }
    None
}

/// Indexes of the best matches (at most `LIMIT`), optionally within one faculty. With an empty query and a faculty, that faculty's
/// courses in code order (the first few dozen).
pub fn search_in(cat: &[Course], query: &str, faculty: Option<&str>) -> Vec<usize> {
    let q = norm(query);
    let in_fac = |c: &Course| faculty.is_none_or(|f| c.faculty == f);
    if q.is_empty() {
        return if faculty.is_some() { cat.iter().enumerate().filter(|(_, c)| in_fac(c)).map(|(i, _)| i).take(LIMIT * 2).collect() } else { vec![] };
    }
    let toks = words(query);
    let mut hits: Vec<(u8, usize)> = cat.iter().enumerate().filter(|(_, c)| in_fac(c)).filter_map(|(i, c)| rank(c, &q, &toks).map(|r| (r, i))).collect();
    hits.sort();
    hits.into_iter().take(LIMIT).map(|h| h.1).collect()
}

/// Faculties by number of courses, most first.
pub fn faculties(cat: &[Course]) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = vec![];
    for c in cat {
        if c.faculty.trim().is_empty() { continue; }
        match out.iter_mut().find(|(f, _)| *f == c.faculty) { Some(e) => e.1 += 1, None => out.push((c.faculty.clone(), 1)) }
    }
    out.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    out
}

/// Courses added most recently (ids, newest first) that are still in the catalogue and not enrolled.
pub fn recent(cat: &[Course], ids: &[String], enrolled: impl Fn(usize) -> bool) -> Vec<usize> {
    ids.iter().filter_map(|id| cat.iter().position(|c| &c.id == id)).filter(|&i| !enrolled(i)).take(5).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::sample;

    fn search(cat: &[Course], q: &str) -> Vec<usize> { search_in(cat, q, None) }

    #[test]
    fn code_prefix_beats_name_and_spaces_dont_matter() {
        let cat = sample().courses;
        let hits = search(&cat, "comp");
        assert_eq!(cat[hits[0]].code, "COMP 1601");
        assert_eq!(search(&cat, "comp1602").first().map(|&i| cat[i].code.as_str()), Some("COMP 1602"));
        assert_eq!(search(&cat, "comp 1602").first().map(|&i| cat[i].code.as_str()), Some("COMP 1602"));
        // A name word finds it too, after the code matches.
        assert!(search(&cat, "programming").len() >= 2);
        assert!(search(&cat, "zzzz").is_empty());
    }

    #[test]
    fn odd_codes_answer_to_each_part() {
        let mut cat = sample().courses;
        cat[0].code = "PSYC 6013/7013".into();
        assert_eq!(search(&cat, "psyc7013"), vec![0]);
        assert_eq!(search(&cat, "PSYC 6013")[0], 0);
    }

    #[test]
    fn faculty_filter() {
        let mut cat = sample().courses;
        cat[1].faculty = "Humanities".into();
        assert_eq!(search_in(&cat, "", Some("Humanities")), vec![1]);
        assert_eq!(faculties(&cat)[0].0, "Sample data");
    }
}

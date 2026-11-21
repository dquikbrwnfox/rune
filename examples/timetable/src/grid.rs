//! The week: blocks, ghost streams, drag to reschedule, clash hatching, the "now" line, a day
//! zoom, and the slots the compositor paints shockwaves, sparkles and the all-clear sheen into.

use std::f32::consts::PI;

use rune::{Beat, Cx, Edge, El, LoopSpec, MotionIntent, Phase, Sequence, Shape, Spring, Transition, Tx, canvas, col, el, row, text};

use crate::data::weeks_label;
use crate::model::{DAYS, DAYS_LONG, Ghost, Sess, clash_set, clusters, describe_pair, end, ghosts, ndays, runs_in, start, tm, week_mask};
use crate::pal::{BODY, CODE, MONO, SMALL, TINY, TITLE, Pal, tone, tone_hex};
use crate::{Drag, State, mark, open_detail, pick};

thread_local! {
    /// Points per hour: 46 on desktop; on a phone, whatever fits the screen.
    static ROW_H: std::cell::Cell<f32> = const { std::cell::Cell::new(46.0) };
}
fn row_h() -> f32 { ROW_H.with(std::cell::Cell::get) }
pub fn set_row_h(v: f32) { ROW_H.with(|c| c.set(v)); }
thread_local! {
    /// Phone layout: one day at a time, paged by the pills and by swiping.
    static COMPACT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
pub fn compact() -> bool { COMPACT.with(std::cell::Cell::get) }
pub fn set_compact(v: bool) { COMPACT.with(|c| c.set(v)); }
const HEAD: f32 = 38.0;
const GUTTER: f32 = 48.0;

/// The ring that pings out of a block as it locks in.
static PING: [Beat; 3] = [Beat::new(0.0).opacity(0.0).scale(0.96), Beat::new(0.08).opacity(0.85).scale(1.0), Beat::new(0.7).opacity(0.0).scale(1.16)];

pub fn stage(s: &State, cx: &mut Cx<State>, p: &Pal, sess: &[Sess], pairs: &[(usize, usize)]) -> El {
    let small = compact();
    let calm = s.shown_calm.get();
    let mut st = col().key("stage").grow(1.0).h_full().min_w(0.0).min_h(0.0).padding(if small { 10.0 } else { 20.0 }).gap(if small { 8.0 } else { 10.0 });
    st = st.child(crate::weeks::scrubber(s, p, small));
    if let Some(&(a, b)) = pairs.first() {
        let more = if pairs.len() > 1 { format!("  +{} more", pairs.len() - 1) } else { String::new() };
        let msg = text(format!("{}{more}", describe_pair(&s.cat, &sess[a], &sess[b]))).font(BODY).color(p.text).grow(1.0).min_w(0.0);
        let fix = el().key("banner-fix").h(if small { 36.0 } else { 26.0 }).px(10.0).center().radius(7.0).fill(p.danger).press_scale(0.94)
            .child(text("Fix").font(rune::tok::TextStyle::new(12.0, 600.0)).color(p.bg)).on_press(|s: &mut State| crate::solve_all(s));
        st = st.child(row().key("banner").w_full().px(12.0).py(if small { 7.0 } else { 9.0 }).gap(10.0).center_y().radius(10.0)
            .fill(p.danger.alpha(0.12)).border(1.0, p.danger.alpha(0.7))
            .transition(Transition::offset(0.0, -10.0).combined(Transition::opacity()))
            .child(el().h(18.0).px(6.0).center().radius(5.0).fill(p.danger).child(text("CLASH").font(TINY).color(p.bg)))
            .child(if small { msg.wrap() } else { msg.truncate() })
            .child(if crate::locked(s) { el() } else { fix }));
    }
    st = st.child(grid(s, cx, p, sess, pairs));
    if !small {
        st = st.child(row().key("hints").gap(18.0).center_y()
            .child(hint(p, "Click", "a block for details"))
            .child(hint(p, "Drag", "a tutorial or lab onto a ghost"))
            .child(hint(p, if ndays() > 5 { "1–6" } else { "1–5" }, "zoom a day"))
            .child(hint(p, "T", "theme"))
            .child(hint(p, "L", "lights"))
            .child(hint(p, "C", "calm"))
            .child(hint(p, "/", "find a course")));
    }
    if calm { return st; }
    // Lightning arcs between the first pair that clash, for as long as they do.
    if let Some(&(a, b)) = pairs.first() {
        let mid = |id: &str| cx.layout(id).map(|r| [r[0] + r[2] * 0.5, r[1] + r[3] * 0.5]);
        if let (Some(pa), Some(pb)) = (mid(&sess[a].id), mid(&sess[b].id)) {
            st = st.child(rune::effect(mark::ARC).key("arc").effect_data([pa[0], pa[1], pb[0], pb[1]])
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).inert().transition(Transition::opacity()));
        }
    }
    // All clear: an aurora over the whole stage.
    if let Some(sh) = &s.sheen {
        st = st.child(rune::effect(mark::AURORA).key(format!("aurora-{}", sh.seq)).effect_data([sh.seq as f32, 0.0, 0.0, 0.0])
            .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).inert());
    }
    // The clash ripple: its slot stays in the tree, on top of the week.
    let shock = s.shock.as_ref().and_then(|x| cx.layout(&x.key).map(|r| [r[0] + r[2] * 0.5, r[1] + r[3] * 0.5, x.seq as f32, 3.0]));
    st.child(rune::effect(mark::SHOCK).key("shock").effect_data(shock.unwrap_or([0.0; 4]))
        .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).inert())
}

fn hint(p: &Pal, key: &str, what: &str) -> El {
    row().gap(6.0).center_y()
        .child(el().h(18.0).px(6.0).center().radius(5.0).border(1.0, p.sep).child(text(key.to_owned()).font(SMALL).color(p.sub)))
        .child(text(what.to_owned()).font(SMALL).color(p.sub))
}

/// Whether a session runs only some of the published weeks.
fn partial(s: &State, x: &Sess) -> bool {
    let published = crate::data::mask(s.meta.weeks_published.iter().copied());
    published != 0 && x.weeks & published != published
}

fn grid(s: &State, cx: &mut Cx<State>, p: &Pal, sess: &[Sess], pairs: &[(usize, usize)]) -> El {
    let body_h = (end() - start()) * row_h();
    let mut labels = el().w_full().h(body_h);
    for h in (start() as i32 + 1)..(end() as i32) {
        labels = labels.child(text(format!("{h:02}:00")).font(MONO).color(p.sub).absolute(6.0, (h as f32 - start()) * row_h() - 7.0));
    }
    let gutter = col().key("gutter").w(GUTTER).child(el().h(HEAD)).child(labels);

    let gh = ghosts(&s.cat, &s.enr, &clash_set(&s.cat, &s.enr, week_mask(s.week)), crate::focus_course(s));
    let days: Vec<u8> = match s.zoom { Some(d) => vec![d.min(ndays() - 1)], None if compact() => vec![crate::shown_day(s).min(ndays() - 1)], None => (0..ndays()).collect() };
    let cw = cx.layout_size(&format!("daybody-{}", days[0])).map_or(160.0, |x| x[0]);
    let target = s.drag.as_ref().and_then(|d| drop_target(s, d));

    let mut g = row().key("grid").w_full().radius(14.0).fill(p.card).border(1.0, p.sep).clip_rounded(14.0)
        .shadow(2.0, 18.0, p.shadow.alpha(0.12)).child(gutter);
    for &d in &days {
        g = g.child(day_col(s, p, d, sess, pairs, &gh, cw, target));
    }
    if let Some(sh) = s.sheen.as_ref().filter(|_| !s.shown_calm.get()) {
        g = g.child(rune::effect(mark::SHEEN).key(format!("sheen-{}", sh.seq)).effect_data([sh.seq as f32, 0.0, 0.0, 0.0])
            .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).inert());
    }
    g
}

#[allow(clippy::too_many_arguments)]
fn day_col(s: &State, p: &Pal, d: u8, sess: &[Sess], pairs: &[(usize, usize)], gh: &[Ghost], cw: f32, target: Option<usize>) -> El {
    let body_h = (end() - start()) * row_h();
    let today = s.clock.is_some_and(|c| c.0 == d);
    let zoomed = s.zoom == Some(d) || compact();
    let label = if zoomed { DAYS_LONG[d as usize] } else { DAYS[d as usize] };
    let small = compact();
    let mut head = row().key(format!("dh-{d}")).w_full().h(HEAD).gap(6.0).center()
        .child(text(label).font(TITLE).color(if today { p.brass } else { p.text }))
        .child(if today { el().key("today-pill").h(16.0).px(5.0).center().radius(4.0).fill(p.brass).child(text("TODAY").font(TINY).color(p.bg)) } else { el() })
        .child(if zoomed && !small { text("Esc").font(SMALL).color(p.sub) } else { el() });
    if !small {
        head = head.hover(p.hover).press_scale(0.97).on_press(move |s: &mut State| s.zoom = if s.zoom == Some(d) { None } else { Some(d) });
    }

    let mut body = el().key(format!("daybody-{d}")).w_full().h(body_h).border(1.0, p.sep.alpha(0.0));
    if small {
        // Swipe the day sideways to page to the next or previous one; it follows the finger, then springs.
        body = body.offset(s.swipe, 0.0).offset_motion(Spring::new(0.4, 0.15))
            .on_drag(move |s: &mut State, g: &mut rune::Drag, tx: &mut Tx| match g.phase {
                Phase::Down => {}
                Phase::Move => { if g.dx.abs() > g.dy.abs() { tx.instant(); s.swipe = g.dx * 0.85; } }
                Phase::Up => {
                    let step = if g.cancelled() || g.dx.abs() < 64.0 || g.dx.abs() < g.dy.abs() { 0 } else if g.dx < 0.0 { 1 } else { -1 };
                    let next = (i32::from(d) + step).clamp(0, i32::from(ndays()) - 1) as u8;
                    s.zoom = Some(next);
                    s.swipe = 0.0;
                    s.swipe_dir = step as i8;
                }
            });
    }
    if today {
        body = body.linear_gradient(PI, [(0.0, p.brass.alpha(0.12)), (0.6, p.brass.alpha(0.04)), (1.0, p.brass.alpha(0.0))]);
    }
    for h in 0..(end() - start()) as i32 {
        body = body.child(el().w_full().h(1.0).fill(p.sep).absolute(0.0, h as f32 * row_h()));
    }
    body = body.child(el().pin(Some(0.0), Some(0.0), None, Some(0.0)).w(1.0).fill(p.sep));

    // Ghosts under, blocks over.
    let gd: Vec<&Ghost> = gh.iter().filter(|x| x.sess.day == d).collect();
    let gs: Vec<Sess> = gd.iter().map(|x| x.sess.clone()).collect();
    let spans: Vec<(f32, f32)> = gs.iter().map(|x| (x.start, x.end())).collect();
    for cl in clusters(&spans) {
        body = body.child(cluster_el(&cl, format!("gc-{d}"), &gs, |i| ghost_el(s, p, gd[i], target)));
    }
    let bd: Vec<Sess> = sess.iter().filter(|x| x.day == d).cloned().collect();
    let clash_ids: Vec<&str> = pairs.iter().flat_map(|&(a, b)| [sess[a].id.as_str(), sess[b].id.as_str()]).collect();
    let spans: Vec<(f32, f32)> = bd.iter().map(|x| (x.start, x.end())).collect();
    for cl in clusters(&spans) {
        let narrow = cl.lanes > 1;
        body = body.child(cluster_el(&cl, format!("bc-{d}"), &bd, |i| {
            block_el(s, p, &bd[i], clash_ids.contains(&bd[i].id.as_str()), narrow, zoomed, cw)
        }));
    }
    // Hatching marks exactly the overlapping time.
    for &(a, b) in pairs {
        let (a, b) = (&sess[a], &sess[b]);
        if a.day != d { continue; }
        let (top, bot) = (a.start.max(b.start), a.end().min(b.end()));
        let hh = (bot - top) * row_h();
        let mut sh = Shape::new().view_box(100.0, hh);
        let mut x = -hh;
        while x < 100.0 { sh = sh.stroke(vec![[x, hh], [x + hh, 0.0]], 1.4, p.danger.alpha(0.55)); x += 9.0; }
        body = body.child(canvas(sh).key(format!("hatch-{}-{}", a.id, b.id)).pin(Some(2.0), Some((top - start()) * row_h()), Some(2.0), None).h(hh)
            .clip_rounded(6.0).inert().transition(Transition::reveal(Edge::Top)));
    }
    if let Some((cd, h)) = s.clock {
        if cd == d && (start()..end()).contains(&h) {
            body = body.child(el().key("now").pin(Some(0.0), Some((h - start()) * row_h() - 1.0), Some(0.0), None).h(2.0).fill(p.danger).inert()
                .child(el().key("now-dot").size(10.0, 10.0).radius(5.0).fill(p.danger).absolute(-5.0, -4.0)
                    .looping(LoopSpec::opacity(1.8, 0.0, 0.35, 1.0))));
        }
    }
    // On a phone the next day slides in from the side you swiped toward and the old one slides out.
    let enter = if small { Transition::asymmetric(
        Transition::offset(60.0 * f32::from(s.swipe_dir), 0.0).combined(Transition::opacity()),
        Transition::offset(-60.0 * f32::from(s.swipe_dir), 0.0).combined(Transition::opacity()),
    ) } else { Transition::scale(0.96).combined(Transition::opacity()) };
    col().key(format!("col-{d}")).grow(1.0).min_w(0.0).transition(enter)
        .child(head).child(body)
}

fn cluster_el(cl: &crate::model::Cluster, key: String, items: &[Sess], mut make: impl FnMut(usize) -> El) -> El {
    let mut lanes: Vec<El> = (0..cl.lanes).map(|_| col().grow(1.0).min_w(0.0).h_full()).collect();
    for &(i, lane) in &cl.items {
        let sx = &items[i];
        let top = (sx.start - cl.start) * row_h() + 1.0;
        let e = make(i).pin(Some(0.0), Some(top), Some(0.0), None).h(sx.hours * row_h() - 2.0);
        lanes[lane] = std::mem::replace(&mut lanes[lane], col()).child(e);
    }
    let mut r = row().key(format!("{key}-{}", cl.start)).gap(3.0)
        .pin(Some(3.0), Some((cl.start - start()) * row_h()), Some(3.0), None).h((cl.end - cl.start) * row_h());
    for l in lanes { r = r.child(l); }
    r
}

/// Which alternative stream a dragged block would land on: the nearest slot on the day under
/// the pointer, within an hour and a quarter.
fn drop_target(s: &State, d: &Drag) -> Option<usize> {
    let c = &s.cat[d.course];
    let group = s.enr.iter().find(|e| e.course == d.course).map_or(0, |e| e.group);
    let day = (f32::from(d.day) + (d.dx / d.cw).round()).clamp(0.0, f32::from(ndays() - 1)) as u8;
    let hour = d.start + d.dy / row_h();
    let mut best: Option<(usize, f32)> = None;
    for (i, st) in c.streams(d.kind).iter().enumerate() {
        if i == d.stream || !c.stream_ok(st, group) { continue; }
        for sl in &st.sessions {
            if sl.day != day { continue; }
            let dist = (sl.start - hour).abs();
            if dist <= 1.25 && best.is_none_or(|b| dist < b.1) { best = Some((i, dist)); }
        }
    }
    best.map(|b| b.0)
}

#[allow(clippy::too_many_arguments)]
fn block_el(s: &State, p: &Pal, x: &Sess, clash: bool, narrow: bool, wide: bool, cw: f32) -> El {
    let tone_i = s.enr.iter().find(|e| e.course == x.course).map_or(0, |e| e.tone);
    let tc = tone(tone_i, p.dark);
    let calm = s.shown_calm.get();
    let locked = crate::locked(s);
    let other = x.kind == 4;
    let edge = if clash { p.danger } else if other { p.sub } else { tc };
    let c = &s.cat[x.course];
    let dragging = s.drag.as_ref().filter(|d| d.course == x.course && d.kind == x.kind && x.kind > 0 && x.kind < 4);
    let (dx, dy) = dragging.map_or((0.0, 0.0), |d| (d.dx, d.dy));
    let kind = x.tag();
    let sid = if (1..=3).contains(&x.kind) { c.streams(x.kind).get(x.stream).map_or(String::new(), |st| st.label.clone()) } else { String::new() };
    let part = partial(s, x);
    let off = !runs_in(x, s.week);
    let id = x.id.clone();

    // Top line: code, then the week tag when it runs only some weeks, else its type and stream.
    let mut top = row().w_full().gap(6.0).center_y()
        .child(text(if other { kind.clone() } else { c.code.clone() }).font(CODE).color(if other { p.sub } else { edge }).truncate().shrink())
        .child(el().grow(1.0));
    if part {
        top = top.child(el().h(14.0).px(4.0).center().radius(4.0).fill(edge.alpha(0.2)).child(text(weeks_label(x.weeks)).font(TINY).color(edge)));
    } else if !(narrow || cw < 125.0 || other) {
        top = top.child(text(if sid.is_empty() { kind.clone() } else { format!("{kind} {sid}") }).font(TINY).color(edge.alpha(0.85)).truncate());
    }
    let fill = if clash { edge.alpha(0.22) } else if other { p.sub.alpha(0.08) } else { edge.alpha(if p.dark { 0.2 } else { 0.14 }) };
    let mut inner = col().key(x.id.clone()).matched(x.id.clone()).mark(mark::SLAB).w_full().h_full().padding(6.0).gap(1.0).radius(8.0)
        .fill(fill).border(if clash { 1.5 } else { 1.0 }, edge.alpha(if other { 0.45 } else { 0.8 }))
        .hover(p.hover).press_scale(0.97).motion(Spring::BOUNCY)
        .offset(dx, dy).offset_motion(Spring::new(0.45, 0.3))
        .opacity(if off { 0.28 } else { 1.0 }).opacity_motion(Spring::SNAPPY)
        .transition(Transition::scale(0.86).combined(Transition::opacity()).combined(Transition::offset(0.0, -8.0)))
        .enter_delay(x.ord as f32 * 0.05)
        .label(format!("{} {} {}–{}", c.code, x.tag(), tm(x.start), tm(x.end())))
        .child(top)
        .child(text(format!("{}–{}", tm(x.start), tm(x.end()))).font(MONO).color(if other { p.sub } else { p.text }).truncate());
    if other {
        // Not a class: muted, with the label CELCAT gave it in full.
        inner = inner.border_dash(2.0, 3.0);
        if x.hours * row_h() >= 40.0 { inner = inner.child(text(format!("{} · {}", c.code, x.room)).font(SMALL).color(p.sub).truncate()); }
    } else if wide && x.hours * row_h() >= 60.0 {
        // Zoomed: there's width to spare, so name and room share one line.
        inner = inner.child(row().w_full().gap(8.0).child(text(c.name.clone()).font(SMALL).color(p.text).truncate().shrink())
            .child(text(format!("· {}", x.room)).font(SMALL).color(p.sub)));
    } else if x.hours > 1.0 && !narrow {
        inner = inner.child(text(x.room.clone()).font(SMALL).color(p.sub).truncate());
    }
    if clash { inner = inner.child(el().h(15.0).px(5.0).center().radius(4.0).fill(p.danger).child(text("CLASH").font(TINY).color(p.bg))); }
    if calm && clash { inner = inner.shadow(0.0, 10.0, p.danger.alpha(0.45)); }
    inner = inner.on_hover_change(crate::track_hot(x.id.clone()));
    if dragging.is_some() || (s.drag.is_none() && s.hot.as_deref() == Some(x.id.as_str())) {
        inner = inner.child(crate::lamp(dragging.is_some()));
    }
    if dragging.is_some() {
        // Picked up: it lifts, glows neon in its own ink, and rides above everything.
        inner = inner.raise().scale(1.05).shadow(10.0, 26.0, p.shadow).fill(edge.alpha(0.26).mix(p.card, 0.25));
        if !calm {
            inner = inner.child(rune::effect(mark::GLOW).key("drag-glow").effect_bleed(60.0).effect_data([tone_hex(tone_i) as f32, 0.0, 0.0, 0.0])
                .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).inert());
        }
    }
    if (1..4).contains(&x.kind) && !locked {
        let (course, kind, stream, day, start) = (x.course, x.kind, x.stream, x.day, x.start);
        let id2 = id.clone();
        inner = inner.cursor(rune::Cursor::Grab).on_drag(move |s: &mut State, d: &mut rune::Drag, tx: &mut Tx| match d.phase {
            Phase::Down => {}
            Phase::Move => {
                tx.instant();
                s.hover = None;
                s.drag = Some(Drag { course, kind, stream, day, start, cw, dx: d.dx, dy: d.dy });
            }
            Phase::Up => {
                let moved = d.dx.abs() + d.dy.abs() > 4.0;
                if let Some(dr) = s.drag.take() {
                    if !d.cancelled() {
                        if let Some(t) = drop_target(s, &dr) { pick(s, course, kind, t) }
                    }
                }
                // A tap that never became a drag opens the block's details.
                if !moved && !d.cancelled() { open_detail(s, id2.clone()); }
            }
        });
    } else {
        inner = inner.on_press(move |s: &mut State| open_detail(s, id.clone()));
    }

    let mut wrap = el().key(format!("w-{}", x.id));
    if clash && !calm {
        wrap = wrap.live_offset(s.shake.smooth(Spring::new(0.05, 0.0)).rest(Spring::new(0.5, 0.62))).motion_intent(MotionIntent::JIGGLE);
    }
    if clash && !calm {
        // A clash burns: flames off the top edge until it's fixed (firelight comes with them).
        wrap = wrap.child(rune::effect(mark::FIRE).key("fire").effect_bleed(240.0).radius(8.0)
            .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).inert().transition(Transition::opacity()));
    }
    wrap = wrap.child(inner);
    if let Some(sp) = s.spark.as_ref().filter(|sp| sp.key == x.id && !calm) {
        let sp = sp.seq;
        let hex = tone_hex(tone_i) as f32;
        wrap = wrap
            .child(el().key(format!("ping-{sp}")).pin(Some(-3.0), Some(-3.0), Some(-3.0), Some(-3.0)).radius(11.0).border(2.0, tc).inert()
                .sequence(Sequence::new(&PING)))
            .child(rune::effect(mark::ZAP).key(format!("zap-{sp}")).effect_bleed(40.0).radius(8.0)
                .effect_data([sp as f32, hex, 0.0, 0.0]).pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).inert())
            .child(rune::effect(mark::SPARK).key(format!("spark-{sp}")).effect_bleed(60.0)
                .effect_data([sp as f32, 70.0, 6.0, hex]).pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).inert());
    }
    wrap
}

fn ghost_el(s: &State, p: &Pal, g: &Ghost, target: Option<usize>) -> El {
    let x = &g.sess;
    let me = (x.course, x.kind, x.stream);
    let drag = s.drag.as_ref().map(|d| (d.course, d.kind));
    let candidate = drag == Some((x.course, x.kind));
    let hot = s.hover == Some(me) || (candidate && target == Some(x.stream));
    let dim = (s.hover.is_some() || drag.is_some()) && !hot && !candidate;
    let tone_i = s.enr.iter().find(|e| e.course == x.course).map_or(0, |e| e.tone);
    let tc = if g.clash { p.danger } else { tone(tone_i, p.dark) };
    let id = s.cat[x.course].streams(x.kind).get(x.stream).map_or(String::new(), |st| st.label.clone());
    let hint = if hot { if g.clash { "Clashes" } else if candidate { "Drop here" } else { "Lock in" } } else { "" };
    let opacity = if hot { 1.0 } else if candidate { 0.75 } else if dim { 0.12 } else { 0.42 };
    let mut gx = col().key(format!("{}-g", x.id)).matched(x.id.clone()).radius(8.0).border(1.5, tc).border_dash(5.0, 4.0)
        .fill(tc.alpha(if hot { 0.2 } else { 0.05 })).center().gap(1.0)
        .opacity(opacity).scale(if hot { 1.05 } else { 1.0 }).motion(Spring::BOUNCY)
        .blur(if dim && drag.is_none() { 1.2 } else { 0.0 }).blur_motion(Spring::new(0.25, 0.0))
        .on_hover_change(move |st: &mut State, h, _| if st.drag.is_none() { if h { st.hover = Some(me) } else if st.hover == Some(me) { st.hover = None } })
        .on_press(move |st: &mut State| pick(st, me.0, me.1, me.2))
        .transition(Transition::opacity())
        .child(text(id).font(CODE).color(tc))
        .child(text(hint).font(SMALL).color(tc));
    if hot { gx = gx.shadow(6.0, 18.0, tc.alpha(0.35)); }
    if hot && !candidate { gx = gx.child(crate::lamp(false)); }
    if hot && candidate {
        // The drop target pulses like radar while a block hovers over it.
        gx = gx.child(rune::effect(mark::PULSE).key("radar").effect_bleed(40.0).radius(8.0)
            .effect_data([if g.clash { 0xe5533d } else { tone_hex(tone_i) } as f32, 0.0, 0.0, 0.0])
            .pin(Some(0.0), Some(0.0), Some(0.0), Some(0.0)).inert());
    }
    gx
}

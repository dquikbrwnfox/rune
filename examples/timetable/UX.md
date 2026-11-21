# Timetable UX: from demo to the tool a student actually uses

A student at UWI St. Augustine has two kinds of moments with a timetable:

1. **Registration (once a semester, high stakes):** "I have these 5–6 courses; give me a week with no
   clashes, ideally with free mornings or a free Friday." They compare options and pick
   tutorial/lab streams, sometimes lecture groups. They want it done in minutes, and kept.
2. **Daily use (every day, low effort):** "What's next, where, and when?" It's mostly on a phone,
   often walking between buildings.

The current demo nails the spectacle and the stream lock-in. It's missing real data, persistence,
week ranges, lecture groups, solving and daily-use views. This spec adds those without losing the
feel. Effects stay, because they're the identity, but they never block or obscure information.

## Principles

- **Real data first.** It loads the published CELCAT timetable for the current semester (static
  `celcat.json`). It shows what semester it is and when the data was fetched. If the data can't
  load, it falls back to the sample and says so clearly.
- **Nothing is lost.** Selections persist (localStorage) and survive reloads. A share link carries
  the whole schedule.
- **A clash is always explained.** It shows which sessions, which day and time, which weeks, and
  the one-tap fixes (the alternative streams that resolve it).
- **Minimum taps.** Search, add, fixed, done. The solver does the tedious stream juggling.
- **Phone-first daily view, desk-first registration view.** It's the same model with two
  emphases.

## Information model (UI terms)

- **Course:** a code (COMP 1601), a name, a faculty or department, and its offerings for the term.
- **Lecture group:** a set of lectures (G1, G2…), or a single "common" group. You pick one.
- **Stream:** a tutorial or lab option (T1, LB2…). It may be tied to a lecture group. You pick one
  of each kind the course has.
- **Session:** a weekday, a start and end time, a room, a type, and **weeks** (e.g. W1–W13, or
  W3–W12, or odd weeks only).
- **Clash:** the same day, overlapping times, **and overlapping weeks.** Sessions in disjoint
  weeks don't clash. The UI shows this nuance ("overlaps in weeks 3–7").

## Screens and flows

### 1. Header / masthead
- Seal, "Timetable", and a term chip ("Semester I · 2026/27"). The chip opens the data source:
  generated date and a link to the official CELCAT page.
- Stats: courses, contact hours a week, clashes (red when there are any), and free days ("Fri
  free" is a selling point).
- Actions: search (`/`), solve, share/export menu, theme, lights (desktop).

### 2. Adding courses (search)
- One search box (the palette on desktop, a full-height sheet on phones) over the whole catalogue.
- It matches code (with or without the space), name words and department. Results are ranked:
  code prefix first, then name.
- Each row shows the code, name, faculty, contact hours a week, lecture-day dots, and
  "Added" or "Add".
- An empty query shows recently added courses and "Browse by faculty" chips.
- Adding a course flies it into the course list. Its lectures cascade into the week, and it picks
  the **best non-clashing default** streams (see the solver), not simply the first stream.

### 3. Course cards (list)
- Header: code, name, contact hours, remove, and collapse.
- **Lecture group** segmented control (only when there are 2 or more groups).
- **Tutorial / Lab** chips:
  - Each chip shows its id and a mini "Tue 3pm" label on hover or long-press.
  - A chip is red-dashed if picking it would clash.
  - A chip is disabled (greyed out, with a tooltip) if it belongs to a different lecture group.
  - The selected chip is filled.
- Partial weeks show as a small "W3–12" tag on that chip or session.
- A clash summary sits on the card when it's involved ("Clashes with MATH 1141 T2, Tue 3–4pm").
  It offers a **"Fix"** button that applies the solver to this course only.

### 4. Week view (desktop) / Day view (phone)
- Mon–Fri columns. Saturday appears only when a chosen or ghost session uses it. The hour range
  fits the data (earliest to latest session, clamped to 7:00–22:00).
- Blocks: code, type and stream, time, room. They show the name when there's space, and a
  "W3–12" corner tag when the session doesn't run every week.
- Ghosts: as today. Dragging a block onto a ghost switches stream.
- **Week scrubber:** a compact "Week 1 … 13" slider or pills. It defaults to "All weeks". Picking a
  week shows only the sessions running that week (with faded others), so a student can check
  week 1 or exam-adjacent weeks.
- Clashes: as today (hatch, fire, banner), with the banner listing the weeks.
- Tapping a block opens a **detail popover/sheet:**
  - course name, type and stream, day and time, room, and weeks;
  - "Other streams" (with tap-to-switch);
  - "Find room on campus map" (link out);
  - "Remove course".

### 5. Solver ("Make it work")
- One button: it searches the lecture-group and stream choices across all added courses for the
  **fewest clashes**. Ties are broken by student preferences:
  - free days (most whole free weekdays);
  - no early starts (before 9:00) or late finishes (after 18:00);
  - compactness (fewer gaps).
- Preferences are a small popover with 3 toggles; the defaults are free day, then no early starts.
- If it finds a clash-free week, it applies it with the existing lock-in animation for every
  changed stream, and the aurora plays. If it can't, it shows the best option and what still
  clashes.
- It's backtracking with pruning. It's fine for about 10 courses times a few streams. Keep it
  under 50 ms, or cap it and say "best found".

### 6. Save, share, export
- **Autosave:** the selection (term, course codes, group and stream picks, preferences) goes to
  localStorage on every change, and restores on load. If a course disappeared from the data,
  say so.
- **Share link:** the URL hash encodes the selection compactly. Opening a link offers "Replace
  my schedule" or "View only".
- **Export:** `.ics` (a weekly RRULE per session, with exact weeks via EXDATE or per-week events),
  and a "Copy as text" summary.
- **Phone wallpaper** export (PNG of the week) is a stretch goal.

### 7. Phone daily use
- The **Today** tab becomes the default once a schedule exists. It shows:
  - "Next up" (a countdown, room, and walking hint);
  - today's sessions as an agenda list;
  - a "No more classes today" state.
- The Week tab is the day pager we have now, plus the week scrubber.
- The Courses tab is the cards, search and solver.
- Large tap targets (at least 44 pt), and swipe between days.

## Visual and motion rules for "real use"
- The effects (fire, lightning, aurora, sparkles, lamp) stay. A **Calm mode** setting, defaulting
  on when the OS asks for reduced motion, swaps fire and lightning for a static red treatment and
  turns off the aurora and sparkles.
- Effects must never cover text the student needs to read. Flames sit behind or around blocks,
  not over labels.
- Loading: a branded skeleton (masthead and empty grid) while `celcat.json` loads, then the data
  fades in.

## What the real data looks like (CELCAT, Semester 1 2026/27)

The source is `celcat.json`, built by a separate CELCAT scraper that is not in this repo (`src/load.rs`
shows the schema it reads). It has 1084 courses and 3301 sessions: 945 KB raw, 79 KB gzipped.

- **Only weeks 10–12 are published right now.** The UI must show which weeks the data covers (the
  `weeksPublished` field) and not imply a full semester. The week scrubber covers the published
  weeks. `calendar.week1Monday` gives dates, but it's inferred, so label it as such.
- **Most tutorial and lab options have no section code** (`synthetic: true`, ids like
  `"Tue 09:00"`). Show them as time options ("Tue 9am · FST C3"), not "T1".
- **Lecture groups are rare** (about 20 courses with G1/G2; the rest are `common`). Hide the group
  control when there's only one. Streams with `parentGroup` are only valid with that group.
- **Cross-listed sessions** (`crossListed[]`) appear under several courses. Don't count a session
  as clashing with its own cross-listed twin.
- **Clashes need overlapping weeks** as well as overlapping times. Week sets can be partial
  (`[11]`, `[10,12]`).
- **Days run 0–6.** Saturday occurs (118 sessions) and Sunday rarely does (6). Show a Sat or Sun
  column only when used. Times include half hours and long blocks (up to 10 h), so render the
  hour range from the data.
- **Rooms are free text:** 177 sessions have none and 101 are "Remote Teaching". Show "Online" or
  "Room TBA". There can be several rooms (`rooms[]`).
- **Non-class entries** have type `Other`, with the label in `rawType`. Show them as a distinct
  muted block style with the raw label. Exams live in `exams[]`; show them on the course card as
  "Exam: Wed 4pm, wk 10" and never in clash solving.
- **Odd codes** (e.g. `PSYC 6013/7013`, `EDLA 11**`) must still search and display.
- **Attribution:** label the app "Unofficial", show the term and the generated date, and link each
  course to its official PDF (`link`).

## Out of scope (for now)
- Exam timetables, room availability search, and course prerequisites or credits checks. These
  depend on what the data has.
- Accounts or sync across devices (the share link covers it).

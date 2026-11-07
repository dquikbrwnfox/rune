# timetable

A UWI St. Augustine timetable builder, built as a Rune showcase. Lectures are fixed. Tutorial and
lab streams show as dashed ghost blocks that you lock in. Clashes are hard to miss. It loads the
published CELCAT timetable for the semester (`celcat.json`), solves for a clash-free week, remembers
your schedule, and shares and exports it. If the data can't load, a small built-in sample (`src/data.rs`)
stands in and the app says so.

```sh
cargo run -p timetable                                         # native
TIMETABLE_DATA=/path/to/celcat.json bash examples/timetable/build-web.sh   # web (WebGPU), from the repo root
python -m http.server 8002 -d target/web-tt
```

`build-web.sh` runs `scripts/web-build.sh timetable --opt --out target/web-tt` (`--opt` needs wasm-opt) and copies
`$TIMETABLE_DATA` to `target/web-tt/celcat.json`. Without `TIMETABLE_DATA` it warns and skips the copy. Native reads
the same file from `TIMETABLE_DATA`, and falls back to the sample when it's unset.

`--opt` runs `wasm-opt -Oz` (`cargo install wasm-opt`): about 7 MB raw, 3 MB gzipped, against 12.5 MB without it.

## Deploy to Cloudflare

`wrangler.jsonc` serves `target/web-tt` as a static-assets Worker: no Worker code, so it fits the
free plan (each file well under the 25 MiB asset limit). Build with `--opt`, then from this folder:

```sh
npx wrangler deploy
```

WebGPU needs HTTPS, which `*.workers.dev` provides.

## What to try

| Do | See |
| --- | --- |
| "Try a sample first year" | Course cards pop in. Each course's lectures cascade into the week. The header stats roll like an odometer. |
| Hover a dashed ghost or a stream chip | It springs forward, rival ghosts blur out of focus, and "Lock in" appears. |
| Click a ghost | The ghost flies into place as a solid block with an electric border zap, a ring pings out, a 70-star burst flashes the room with light. |
| Pick a stream that overlaps | Both blocks shudder and catch fire for as long as the clash lasts, lightning arcs between them, firelight flickers over the week, a vermilion ripple rings out, the overlap is hatched, and a banner names it. |
| Drag a tutorial or lab block | It lifts with a neon glow, follows the pointer and a radar pulse marks the ghost it would land on. Drop it to switch streams; anywhere else it springs home. |
| Clear the last clash | An aurora washes over the week for a few seconds, then a sheen sweeps across it. |
| `L`, or the bulb | Lights on/off. Lit: blocks and cards stand up and cast soft shadows, a low sun with god rays by day and a cool moon at night. Hover or drag a block or card and a warm lamp comes up over it (it glides between them and fades when you leave). Off: the flat paper look with grain. |
| `/` (or Ctrl/Cmd-K, or "Find a course") | A frosted command palette. Arrow keys move and Return adds; the course flies out of the palette into the sidebar. |
| `1`–`5`, or click a day header | That day zooms to full width with more detail. Esc goes back. |
| `T`, or the sun | Paper ("Registrar's Ledger") springs to night ("Night Shift"): every colour animates in OKLab and the sun turns into a moon. |
| Look at the "U" seal | Struck brass: a gradient, a bevelled rim and a soft gloss in plain paint. It has no shader layer and draws no frames of its own. |

Today's column gets a brass wash and a TODAY tag. During class hours a red "now" line breathes across it.

## Using it (the UX pass, see `UX.md`)

- **Course cards** (`card.rs`): a lecture-group segmented control (only with two or more groups); chips per tutorial, lab and field trip with human labels
  ("T1", or "Tue 9am · FST C3" for a time option). A chip is red-dashed when it would clash, greyed out with a tooltip when it belongs to another group,
  and carries a "W11" tag when it runs only some of the published weeks. The line under the chips says day, time, room and weeks for the hovered chip
  (a preview on the grid too) or else the selected one. More than five options become a scrolling strip on a phone. A clash shows what it is with **Fix**;
  the card also has the exam line, collapse and remove.
- **Block details** (`detail.rs`, `menus.rs`): click or tap a block for a popover (a bottom sheet on a phone): name, type and stream, day and time, rooms
  (Online / Room TBA), weeks, staff, other options to switch to, **Official timetable** and **Remove course**.
  Non-class entries are muted and dashed, with their raw label.
- **Stable grid**: the hour range covers every enrolled session; the focused course's ghosts widen it only if they must, and it never shrinks until the
  schedule changes, so hovering from card to card doesn't move the grid.
- **Week scrubber** (`weeks.rs`): **All weeks** or one pill per published week. A week fades the sessions that don't run in it and limits the clash
  count and banner to it. It says "CELCAT has published weeks 10–12 so far", and the pill for the current week has a dot.
- **Make it work** with **Prefs** (free day, no early starts, no late finishes, compact). The result is "No clashes · 2 changes" or
  "Best found: 1 clash left — A lecture vs B tutorial (Mon 9am–10am, W10–12)".
- **Share** menu: copy link, download .ics, copy as text, with a toast. An incoming link offers **Replace my schedule**, **View only** (read-only, with
  **Keep mine** and **Make this mine**) or **Dismiss**, and says when the link was made for another term.
- **Calm mode** (**Calm** button or `C`; on by default when the system asks for reduced motion): no fire, lightning, aurora, sparkles or shake. A clash keeps
  its hatching, red border, **CLASH** badge and a static red glow. It is saved with the schedule.
- **Search**: code (with or without the space) first, then name, then department; odd codes (`PSYC 6013/7013`) answer to each part. Results show
  faculty, contact hours and lecture-day dots. Empty, the palette offers **Browse by faculty** chips and recently added courses. Arrow keys scroll the list.
- **Loading and data**: a skeleton in the app's own colours while `celcat.json` loads; the term chip opens the data source (fetched date,
  published weeks, "Unofficial — data from UWI CELCAT", a link to CELCAT); the sample fallback is flagged in red.

## On a phone

Under 720 pt wide the app switches to a phone layout (`mobile.rs`). The desktop layout is unchanged.

- **Masthead:** the seal, the title with the stats (or a CLASH chip), the term chip, and search and theme buttons.
- **Today tab** (`today.rs`, the default once there is a schedule): a **Next up** card (or **Now**) with a countdown and the room, then today's classes.
  A weekend or an empty day says "No classes today" and shows the next class. If the current week isn't published it says so and follows the usual pattern.
- **Week tab:** the week scrubber, then day pills (red dot for a day with a clash, brass for today) over one day at a time. Swipe sideways to page between days; the day follows your finger, then the next one slides in from that side. The hour height shrinks to fit the screen, so the day never scrolls.
- **Courses tab:** **Make it work**, **Prefs**, **Share**, **Calm** and the lamp, then search and the course cards, with 40 pt chips for touch.
- **Palette:** opens as a full-height sheet. Adding a course jumps to the Week tab so you see it land.
- **Lighting:** starts off to save fill-rate and battery on high-DPR screens. The bulb turns it on, with a calmer rig for phones: brighter ambient, a higher sun, softer god rays, lower slabs, and the hover lamp only while you drag (touch has no hover-out).
- **Effects:** everything else still plays, with small flames so the text stays readable (or **Calm**).
- **Safe areas:** the layout stays inside them (notch, home bar).

## Data

`celcat.json` comes from a separate CELCAT scraper that is not in this repo; the schema is what `src/load.rs` parses.
The app fetches `celcat.json` from the page's own origin after the first frame (`load.rs`), parses it into
`Course { groups, tutorials, labs, field_trips, other, exams }` and swaps it in on the next `poll`. The status
(Loading, Ready, Fallback with a reason) shows under the title and in a notice bar. About 1,000 KB parses in tens of
milliseconds; the timing is logged to the console as `timetable: N courses, fetch X ms, parse Y ms`.

- Sessions carry a week bitmask. Sessions in disjoint weeks never clash. The header says which weeks are published.
- Most tutorial and lab options are unlabelled in CELCAT, so their label is a time ("Tue 9am"), with the room added when two
  options share a time. Streams tied to a lecture group (`G1 T2`) are greyed out for the other group.
- Non-class entries (`other`: help desks, review centres) show as muted blocks and never clash or count as contact hours.
  Exams (`exams`) show on the course card ("Exam: Wed 4pm–7pm · W11") and take no part in clashes or solving.
- A cross-listed session shows under each course it belongs to (same time, room and listing). The two copies are one session, not a clash.
- The grid's hours (clamped 7:00–22:00, at least 8 tall) and days (Saturday and Sunday only when used) come from the data
  (`model::set_range`). Ghost streams show only for the course you last touched (hovered, picked from, or added), since
  a real course can offer dozens of time options.

## Solver, saving, sharing, exporting

- **Solver** (`solve.rs`): depth-first search with branch and bound over lecture group and one stream per kind, per course.
  Cost is 10,000 per clashing pair, then preferences (`Prefs`: fewest days used, no starts before 9, no finishes after 18,
  fewer gaps; the default is free days then no early starts), then 1 per change from the current picks. It stops after 400,000 nodes or 40 ms
  and returns the best found ("best found" in the notice). Adding a course fits it around what's already picked; **Make it work**
  re-picks everything (`solve_all`), and a card's **Fix** re-picks that course (`fix_course`). Both apply through `pick`/`set_group`,
  so the lock-in animation plays.
- **Autosave** (`store.rs`): on every change, to localStorage `uwi-timetable:v1`: term, per course {id, code, group, stream ids},
  preferences, theme override, lights, calm. It restores after the catalogue loads, matching by id and then by code, and lists
  courses it couldn't find. It only saves against the real catalogue, so the offline sample never overwrites a real schedule.
- **Share**: **Share** copies `<page>#s=<base64url of compact JSON>` (`{"t": term, "c": [[id, code, group, tutorial, lab, field trip], ...]}`).
  Opening one sets `State::pending_share`; the notice bar offers **Replace my schedule** (`store::apply_share`), **View only** (`store::view_share`) or **Dismiss**.
- **Export** (`export.rs`): **.ics** downloads `uwi-timetable.ics` (America/Port_of_Spain, one event per session per teaching week
  from `week1Monday`, which is inferred, so check the dates); **Copy text** puts `summary_text` on the clipboard.

## Layout

- `model.rs`: sessions, ghosts, clashes, lanes, the visible hour range. `data.rs`: the model and the offline sample. `load.rs`: fetch and parse.
  `solve.rs`, `store.rs`, `export.rs`, `search.rs`, `host.rs` (localStorage, hash, download): see above.
- `lib.rs`: state, `poll` (clash triggers, timed effects, clock), keys, the view. `actions.rs`: every edit (all refused while viewing a shared schedule).
- `header.rs`, `side.rs`, `card.rs`, `grid.rs`, `cmdk.rs`, `results.rs`, `weeks.rs`, `detail.rs`, `menus.rs`, `today.rs`, `skeleton.rs`: views;
  `mobile.rs`: the phone layout. `pal.rs`: both themes.
- `gfx.rs`: the compositor. Lights on, the UI is drawn linear into an HDR texture and lit by rune-light (slabs, sun, lamp, firelight), then rune-fx layers (fire, lightning, aurora, zap, sparkles, neon glow, radar, ripple, sheen) are drawn over it. Lights off, effects slot into the normal paint order (plus paper grain),
  behind the default `fx` feature. Each one-shot effect carries a sequence number; the compositor
  asks for frames only while one plays, so an idle window draws nothing.

## Notes

- Building this on the web needed two fixes in Rune (`changelog/unreleased/2026-11-06-rune-fx-wasm-sync.md`):
  rune-fx's pipeline compile required a `Sync` device, and web windows never recorded effect marks.
- The clash ripple is a `PulseStyle::Ripple`, not the backdrop `Shockwave` distortion, which drew
  black on the web build.
- `web-build.sh --dev` fails under Git Bash on Windows (quoted `--config` keys); the release build
  is fast once kache has Rune cached.
- The wasm target must be installed on the pinned toolchain (`rustup target add wasm32-unknown-unknown` in this repo).
- Browser automation that inserts text (rather than sending key events) doesn't reach a canvas
  app's shortcuts; real keystrokes do.
- Additive effects blow out on paper, so fire, smoke, lightning and the extra point lights are scaled
  down by day and up at night. Lights are tuned in `gfx.rs` (`Settings`, sun and lamp). Fire, lightning and the
  hover lamp (while it fades or glides) keep frames coming; an idle window still draws nothing.

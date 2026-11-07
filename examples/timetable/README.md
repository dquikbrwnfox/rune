# timetable

A UWI St. Augustine timetable builder, built as a Rune showcase. Lectures are fixed. Tutorial and
lab streams show as dashed ghost blocks that you lock in. Clashes are hard to miss. The data is a
sample catalogue in `src/data.rs`.

```sh
cargo run -p timetable                                         # native
bash scripts/web-build.sh timetable --opt --out target/web-tt  # web (WebGPU); --opt needs wasm-opt
python -m http.server 8002 -d target/web-tt
```

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
| Hover the "U" seal | The liquid brass flows for a few seconds. |

Today's column gets a brass wash and a TODAY tag. During class hours a red "now" line breathes across it.

## On a phone

Under 720 pt wide the app switches to a phone layout (`mobile.rs`). The desktop layout is unchanged.

- **Masthead:** the seal, the title, a stats line with a CLASH chip, and search, lamp and theme buttons.
- **Week tab:** day pills (red dot for a day with a clash, brass for today) over one day at a time. Swipe sideways to page between days; the day follows your finger, then the next one slides in from that side. The hour height shrinks to fit the screen, so the day never scrolls.
- **Courses tab:** the course cards and stream chips at full width, with 36 pt chips for touch.
- **Palette:** opens as a full-width sheet. Adding a course jumps to the Week tab so you see it land.
- **Lighting:** starts off to save fill-rate and battery on high-DPR screens. The bulb turns it on.
- **Effects:** everything else still plays, with smaller flames so the text stays readable.
- **Safe areas:** the layout stays inside them (notch, home bar).

## Layout

- `model.rs`: sessions, ghosts, clashes, lanes. `data.rs`: sample catalogue.
- `lib.rs`: state, actions, `poll` (clash triggers, timed effects, clock), keys.
- `header.rs`, `side.rs`, `grid.rs`, `cmdk.rs`: views; `mobile.rs`: the phone layout. `pal.rs`: both themes.
- `gfx.rs`: the compositor. Lights on, the UI is drawn linear into an HDR texture and lit by rune-light (slabs, sun, lamp, firelight), then rune-fx layers (fire, lightning, aurora, zap, sparkles, neon glow, radar, ripple, liquid-metal seal, sheen) are drawn over it. Lights off, effects slot into the normal paint order (plus paper grain),
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

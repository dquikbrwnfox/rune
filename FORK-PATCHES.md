# Changes this fork carries on top of upstream `dev`

Upstream doesn't take contributions, so these stay here and we only pull. After each
`git merge origin/dev`, check that each one is still in place (the first two are small enough to
redo by hand if upstream rewrites the file around them).

1. **`crates/rune-fx/src/pipes.rs`**: `in_parallel` has a wasm32 version without the `Sync` bound
   (wgpu's device isn't `Sync` on the web). Without it `rune-fx` doesn't compile for wasm.
2. **`crates/rune/src/platform/web/app.rs`**, where the web runtime is created: after
   `rt.configure(runtime);`, `rt.track_marks = compositor.is_some();`. Without it every effect
   slot reads as mark 0 on the web, so a compositor can't tell its effects apart. (It was in
   `platform/web.rs` until upstream split that file.)
3. **`scripts/web-build.sh`**: normalises backslashes (`tr '\' '/'`) before matching the embedded
   Inter path in the build's dependency file, so the full Inter faces are copied beside `app.js` on
   Windows.
4. **`examples/timetable`** is ours entirely. Its `index.html` sets `globalThis.RUNE_CPU_FIRST =
   "off"`: since upstream made full-window web apps start on the CPU and ask for the GPU after the
   first frame, a page with a compositor can stall before the GPU is ever asked (blank page;
   `performance.getEntriesByType("mark")` stops at `web.started`). `examples/timetable/build-web.sh`
   installs that page.

If upstream fixes 1 or 2 itself, drop ours. Each new `dev` also needs a rebuild and a look at the
lit scene, the effects and the phone layout: the example leans on `Renderer` internals
(`draw_target_range_with`, `encode_layers`, `add_format`) that upstream reworks from time to time.

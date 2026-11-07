//! The compositor. With lights on, the UI is drawn linear into an HDR texture and rune-light lights
//! it into the window: course blocks and cards stand up as slabs that cast soft shadows, a warm lamp
//! comes up over the block or card you hover or drag, with god rays streaming past it, a low sun rakes across
//! the paper by day, fire on a clash lights the week orange, and a lock-in flashes. rune-fx layers
//! (fire, lightning, sparkles, glow, aurora, ripple, liquid metal) are drawn over the lit frame.
//!
//! One-shot effects carry a sequence number in their slot data: the first frame that sees a new one
//! stamps its start, and the compositor asks for frames only while something is still moving.

use std::collections::HashMap;

use rune::compose::{Compositor, Frame, is_glyph};
use rune::render::{EffectPhase, EffectSite, KIND_EFFECT, KIND_RECT};
use rune_fx::*;
use rune_light::{Light, LightRenderer, Prim, Scene as LightScene, Settings};

use crate::{State, mark};

const HDR: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

struct Lit {
    size: (u32, u32),
    base: wgpu::TextureView,
    text: wgpu::TextureView,
    light: LightRenderer,
    cleared: bool,
}

#[derive(Default)]
pub struct Gfx {
    fx: Option<FxRenderer>,
    lit: Option<Lit>,
    /// Start time of each (mark, sequence) seen.
    t0: HashMap<(u32, u32), f32>,
    busy_until: f32,
    last_t: f32,
    /// Anything continuous this frame (fire, lightning, the lamp still gliding, the sun moving).
    live: bool,
    lamp: Option<[f32; 2]>,
    /// 0..1: the hover lamp fading in over a hot block or card, and out when nothing is.
    lamp_on: f32,
    lamp_drag: bool,
    /// Phone layout: smaller fire.
    small: bool,
    /// Per-frame lists, kept to reuse their allocations.
    slots: Vec<Slot>,
    prims: Vec<Prim>,
    lights: Vec<Light>,
    layers: Vec<FxLayer>,
    at: Vec<u32>,
    placements: Vec<FxSlot>,
    /// 0 = day, 1 = night; eases toward the theme so the sun sets and the lamp comes up.
    night: f32,
    dt_t: f32,
}

/// A slot the view put in the tree: its mark, rect and radius in points, data, scale, clip.
struct Slot { n: usize, m: u32, r: [f32; 4], rad: f32, d: [f32; 4], k: f32, clip: [f32; 4], fade: [u32; 4] }

fn lin(hex: u32) -> [f32; 3] {
    let c = |v: u32| { let s = v as f32 / 255.0; if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) } };
    [c(hex >> 16 & 0xff), c(hex >> 8 & 0xff), c(hex & 0xff)]
}

impl Gfx {
    /// When this one-shot started; a new one starts now and keeps frames coming for `life`.
    fn stamp(&mut self, m: u32, seq: u32, t: f32, life: f32) -> f32 {
        let t0 = *self.t0.entry((m, seq)).or_insert(t);
        self.busy_until = self.busy_until.max(t0 + life);
        t0
    }

    fn layer(&mut self, s: &Slot, t: f32, lights: &mut Vec<Light>) -> Option<FxLayer> {
        let [x, y, w, h] = s.r;
        let (d, k) = (s.d, s.k);
        let shape = Shape::rounded(x, y, w, h, s.rad);
        let centre = [(x + w * 0.5) * k, (y + h * 0.5) * k];
        let fx = match s.m {
            mark::SEAL => {
                self.stamp(s.m, d[0] as u32, t, 3.5);
                Fx::fill(LiquidMetalParams { speed: 0.7, ..LiquidMetalParams::gold() })
            }
            mark::SHOCK => {
                let seq = d[2] as u32;
                if seq == 0 { return None; }
                let t0 = self.stamp(s.m, seq, t, 1.5);
                Fx::pulse(PulseParams {
                    style: PulseStyle::Ripple { at: [d[0], d[1]] },
                    color: rgb(0xff5a2a), core: rgb(0xffd2a0), count: 3, reach: 1100.0, width: 4.0, intensity: 0.85,
                    t0, duration: 1.4, ..Default::default()
                })
            }
            mark::SPARK => {
                // Lock-in: a burst of stars and a flash of light.
                let t0 = self.stamp(s.m, d[0] as u32, t, 1.4);
                let age = t - t0;
                if age < 0.7 {
                    let a = (1.0 - age / 0.7).powi(2);
                    lights.push(Light::point(centre[0], centre[1], 90.0 * k).color(lin(d[3] as u32)).intensity(1.2 * a).radius(10.0 * k).range(420.0 * k));
                }
                return Some(Fx::sparkles(SparkleParams { burst: Some(t0), count: d[1] as u32, size: d[2], color: rgb(d[3] as u32), margin: 46.0, ..Default::default() }).on(shape).at(t));
            }
            mark::ZAP => {
                let t0 = self.stamp(s.m, d[0] as u32, t, ElectricParams::ZAP_LIFE + 0.1);
                Fx::electric(ElectricParams { style: ElectricStyle::Border, zap: Some(t0), color: rgb(d[1] as u32), intensity: 1.6, glow: 10.0, amplitude: 7.0, ..Default::default() })
            }
            mark::SHEEN => {
                let t0 = self.stamp(s.m, d[0] as u32, t, 1.6);
                return Some(Fx::shimmer(ShimmerParams { intensity: 0.45, period: 1.6, band: 150.0, feather: 70.0, pause: 0.0, ..Default::default() }).on(shape).at(t - t0));
            }
            mark::AURORA => {
                // All clear: an aurora washes over the week and fades.
                let t0 = self.stamp(s.m, d[0] as u32, t, 3.0);
                let a = ((t - t0) / 0.5).min(1.0) * (1.0 - ((t - t0 - 1.8) / 1.2).clamp(0.0, 1.0));
                if a <= 0.0 { return None; }
                Fx::aurora(AuroraParams { intensity: (0.22 + 0.4 * self.night) * a, speed: 0.6, ..Default::default() })
            }
            mark::FIRE => {
                // A clash burns until it's fixed, and its firelight flickers over the week.
                self.live = true;
                let flick = 0.8 + 0.2 * (t * 13.0 + x).sin() * (t * 7.3 + y).cos();
                lights.push(Light::point(centre[0], (y + h * 0.3) * k, 120.0 * k).color(lin(0xff6a1a)).intensity(0.3 * flick).radius(18.0 * k).range(380.0 * k));
                let n = self.night;
                Fx::fire(FireParams { style: FireStyle::Top, height: if self.small { 60.0 } else { 130.0 }, intensity: 0.5 + 0.8 * n, smoke: 0.08 + 0.4 * n, embers: 40, scale: 26.0, ..FireParams::default() })
            }
            mark::ARC => {
                // Lightning between the two blocks that clash.
                self.live = true;
                let mid = [(d[0] + d[2]) * 0.5 * k, (d[1] + d[3]) * 0.5 * k];
                let flick = 0.6 + 0.4 * (t * 31.0).sin().abs();
                lights.push(Light::point(mid[0], mid[1], 100.0 * k).color(lin(0x9fc8ff)).intensity(0.15 * flick).radius(8.0 * k).range(320.0 * k));
                Fx::electric(ElectricParams { style: ElectricStyle::Arc { a: [d[0], d[1]], b: [d[2], d[3]] }, amplitude: 14.0, glow: 5.0 + 4.0 * self.night, intensity: 0.6 + 0.8 * self.night, color: rgb(0x6fa8ff), ..Default::default() })
            }
            mark::GLOW => {
                self.live = true;
                Fx::glow(GlowParams { style: GlowStyle::Neon, color: rgb(d[0] as u32), intensity: 1.1, radius: 22.0, ..Default::default() })
            }
            mark::PULSE => {
                self.live = true;
                Fx::pulse(PulseParams { style: PulseStyle::Radar, color: rgb(d[0] as u32), period: 1.1, reach: 26.0, ..Default::default() })
            }
            mark::GRAIN => Fx::grain(GrainParams { amount: d[0], fps: 0.0, ..Default::default() }),
            _ => return None,
        };
        Some(fx.on(shape).at(t))
    }
}

impl Compositor<State> for Gfx {
    fn animating(&self) -> bool { self.live || self.last_t < self.busy_until }

    fn render(&mut self, f: &mut Frame<'_, State>) {
        let size = f.size;
        let t = f.rt.t as f32;
        let dt = (t - self.dt_t).clamp(0.0, 0.1);
        self.dt_t = t;
        self.last_t = t;
        self.live = false;

        // ---- slots, slabs and the lighting switch (lists reuse last frame's allocations)
        let mut slots = std::mem::take(&mut self.slots);
        let mut prims = std::mem::take(&mut self.prims);
        slots.clear();
        prims.clear();
        let (mut lights_on, mut dark) = (false, false);
        let mut k = 1.0;
        for (n, i) in f.rt.instances.iter().enumerate() {
            let m = f.rt.marks.get(n).copied().unwrap_or(0) & 0xff;
            k = i.params[3].max(0.01);
            if i.params[1] == KIND_EFFECT {
                let r = i.radii.iter().fold(0.0f32, |a, v| a.max(v.abs())) / k;
                if m == mark::LIGHTS { lights_on = i.uv[0] > 0.5; dark = i.uv[1] > 0.5; self.small = i.uv[2] > 0.5; continue; }
                slots.push(Slot { n, m, r: i.rect.map(|v| v / k), rad: r, d: i.uv, k, clip: i.clip, fade: i.fade });
            } else if (m == mark::SLAB || m == mark::CARD) && i.params[1] == KIND_RECT && !is_glyph(i) && i.rect[2] > 24.0 {
                let h = if m == mark::SLAB { 14.0 } else { 7.0 } * k;
                prims.push(Prim::from_rune(i.rect, i.radii).raised(h).bevel(3.0 * k, 2.0 * k).gloss(0.55, 0.35));
            }
        }
        let (w, h) = (size.0 as f32, size.1 as f32);

        // ---- lights: a raking sun by day, moonlight by night; the theme flip moves between them.
        // A lamp comes up over whatever block or card you hover or drag, and glides between them.
        let goal = if dark { 1.0 } else { 0.0 };
        self.night += (goal - self.night) * (1.0 - (-dt * 3.0).exp());
        if (goal - self.night).abs() > 0.002 { self.live = true } else { self.night = goal }
        let target = slots.iter().find(|s| s.m == mark::LAMP).map(|s| {
            let [x, y, sw, sh] = s.r;
            ([(x + sw * 0.5) * s.k, (y + sh * 0.5) * s.k], s.d[0] > 0.5)
        });
        let on_goal = if target.is_some() { 1.0 } else { 0.0 };
        let rate = if on_goal > self.lamp_on { 10.0 } else { 5.0 };
        self.lamp_on += (on_goal - self.lamp_on) * (1.0 - (-dt * rate).exp());
        if (on_goal - self.lamp_on).abs() < 0.003 { self.lamp_on = on_goal } else { self.live = true }
        if let Some((p, drag)) = target {
            self.lamp_drag = drag;
            let lamp = self.lamp.get_or_insert(p);
            if self.lamp_on < 0.05 { *lamp = p } // arriving from dark: start where it's needed
            let ease = 1.0 - (-dt * 12.0).exp();
            lamp[0] += (p[0] - lamp[0]) * ease;
            lamp[1] += (p[1] - lamp[1]) * ease;
            if (p[0] - lamp[0]).abs() + (p[1] - lamp[1]).abs() > 0.5 { self.live = true }
        }
        let lamp = self.lamp.unwrap_or([w * 0.5, h * 0.5]);
        let (night, lamp_on) = (self.night, self.lamp_on);
        let mut lights = std::mem::take(&mut self.lights);
        lights.clear();
        if lights_on {
            let day = 1.0 - night;
            if day > 0.01 {
                // Low afternoon sun from the upper left; it sinks to the right as night falls.
                let sx = -0.15 * w + night * 1.3 * w;
                lights.push(Light::point(sx, -0.35 * h, 520.0 * k).color(lin(0xffdca0)).intensity(24.0 * day).radius(28.0 * k).range(8000.0 * k).shafts(0.55 * day).specular(0.7));
            }
            if night > 0.01 {
                lights.push(Light::point(0.92 * w, -0.4 * h, 1000.0 * k).color(lin(0x8fa8ff)).intensity(9.0 * night).radius(90.0 * k).range(8000.0 * k).shafts(0.35 * night));
            }
            if lamp_on > 0.01 {
                // Brighter and lower while dragging, so the lifted block throws a long shadow.
                let (z, boost) = if self.lamp_drag { (150.0, 1.25) } else { (210.0, 1.0) };
                let i = (1.4 + 2.0 * night) * boost * lamp_on;
                lights.push(Light::point(lamp[0], lamp[1], z * k).color(lin(0xffc27a)).intensity(i).radius(16.0 * k).range(1100.0 * k)
                    .shafts((0.45 + 0.55 * night) * lamp_on).specular(1.2));
            }
        }

        // ---- effect layers (which may add lights of their own)
        let (mut layers, mut at, mut placements) = (std::mem::take(&mut self.layers), std::mem::take(&mut self.at), std::mem::take(&mut self.placements));
        layers.clear();
        at.clear();
        placements.clear();
        let mut fire = false;
        for s in &slots {
            if s.m == mark::LAMP || (lights_on && s.m == mark::GRAIN) { continue; }
            let Some(l) = self.layer(s, t, &mut lights) else { continue };
            fire |= s.m == mark::FIRE;
            at.push(s.n as u32);
            layers.push(l);
            placements.push(FxSlot::default().scale(s.k).clip(s.clip, s.fade));
        }
        lights.truncate(rune_light::MAX_LIGHTS);
        if self.t0.len() > 256 { self.t0.retain(|_, t0| *t0 > t - 10.0); }
        self.draw(f, &slots, &prims, &lights, &layers, &at, &placements, fire, lights_on, k);
        (self.slots, self.prims, self.lights, self.layers, self.at, self.placements) = (slots, prims, lights, layers, at, placements);
    }
}

impl Gfx {
    #[allow(clippy::too_many_arguments)]
    fn draw(&mut self, f: &mut Frame<'_, State>, _slots: &[Slot], prims: &[Prim], lights: &[Light], layers: &[FxLayer], at: &[u32], placements: &[FxSlot], fire: bool, lights_on: bool, k: f32) {
        let (device, queue, size) = (f.device, f.queue, f.size);
        let (t, night) = (f.rt.t as f32, self.night);

        let fx = self.fx.get_or_insert_with(|| FxRenderer::new(device, f.format, FxOptions { dither: true }));
        fx.set_fire_sim(device, fire);
        fx.set_srgb_blend(!f.format.is_srgb());
        fx.add_format(device, HDR);
        let clear = f.rt.clear_color();

        if !lights_on || lights.is_empty() {
            // ---- unlit: Rune draws, effects slot in at their places in the painter stream
            fx.prepare_with_slots(device, queue, &FxFrame { viewport: [size.0, size.1], scale: 1.0, layers: &layers, backdrop: None, sources: &[] }, &placements);
            if fire { fx.simulate(queue, f.encoder, None); }
            f.ui.prepare(device, queue, &mut f.rt.text.atlas, &f.rt.instances, size);
            f.ui.draw_with_backdrop(f.encoder, f.target, clear, &mut |phase, s: &EffectSite<'_>| {
                let Some(i) = at.iter().position(|n| *n == s.index) else { return };
                match phase {
                    EffectPhase::Backdrop(enc) => { if let Some(bd) = s.backdrop { fx.blur_backdrop(device, queue, enc, bd, i); } }
                    EffectPhase::Draw(pass) => fx.draw_layer_in(pass, s.format, s.origin, i),
                }
            });
            return;
        }

        // ---- lit: the UI linear into HDR, rune-light into the window, effects over it
        if self.lit.as_ref().is_none_or(|l| l.size != size) {
            let tex = |label| device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d { width: size.0.max(1), height: size.1.max(1), depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: HDR,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            }).create_view(&Default::default());
            self.lit = Some(Lit { size, base: tex("ui base"), text: tex("ui text"), light: LightRenderer::new(device, queue, size.0, size.1, f.format), cleared: false });
        }
        let Some(lit) = self.lit.as_mut() else { return };
        let srgb = !f.format.is_srgb();
        f.ui.set_srgb_blend(false); // linear colour into the float target
        f.ui.add_format(device, HDR);
        f.ui.prepare(device, queue, &mut f.rt.text.atlas, &f.rt.instances, size);
        let count = f.rt.instances.len() as u32;
        f.ui.encode_layers(f.encoder);
        let bg = clear.map(|c| c as f64);
        f.ui.draw_target_range_with(f.encoder, &lit.base, HDR, wgpu::LoadOp::Clear(wgpu::Color { r: lin1(bg[0]), g: lin1(bg[1]), b: lin1(bg[2]), a: 1.0 }), 0..count, &mut |_, _| {});
        f.ui.set_srgb_blend(srgb);
        if !lit.cleared {
            let _ = f.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ui text clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &lit.text, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None, multiview_mask: None,
            });
            lit.cleared = true;
        }
        let settings = if night > 0.5 {
            Settings { ambient: [0.3, 0.31, 0.36], rim: 0.9, contact_ao: 0.5, shafts_over: 0.55, albedo_floor: 0.06, background_gloss: 0.5, time: t, ..Default::default() }
        } else {
            Settings { ambient: [0.68, 0.66, 0.64], rim: 0.8, contact_ao: 0.25, shafts_over: 0.45, albedo_floor: 0.03, time: t, ..Default::default() }
        };
        let drawn = lit.light.render(device, queue, f.encoder, &LightScene { prims: &prims, lights: &lights, settings, emission: None }, &lit.base, &lit.text, f.target);
        if !drawn { return; }
        fx.prepare(device, queue, &FxFrame { viewport: [size.0, size.1], scale: k, layers: &layers, backdrop: None, sources: &[] });
        if fire { fx.simulate(queue, f.encoder, None); }
        let mut pass = f.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fx over lit"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: f.target, depth_slice: None, resolve_target: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
            depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None, multiview_mask: None,
        });
        for i in 0..layers.len() { fx.draw_layer(&mut pass, i); }
    }
}

/// The clear colour arrives sRGB-encoded; the float target wants it linear.
fn lin1(c: f64) -> f64 { if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) } }

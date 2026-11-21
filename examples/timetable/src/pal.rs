//! Two looks for one app. Light is "Registrar's Ledger": warm paper, navy ink, brass.
//! Dark is "Night Shift": deep ink, phosphor amber, signal vermilion. Every colour springs
//! between them in OKLab when the theme flips. No violet anywhere.

use rune::tok::TextStyle;
use rune::{Color, oklch, oklcha};

#[derive(Clone, Copy)]
pub struct Pal {
    pub dark: bool,
    pub bg: Color,
    pub card: Color,
    pub raised: Color,
    pub text: Color,
    pub sub: Color,
    pub sep: Color,
    pub brass: Color,
    pub danger: Color,
    pub hover: Color,
    pub scrim: Color,
    pub shadow: Color,
}

pub fn pal(dark: bool) -> Pal {
    if dark {
        Pal {
            dark,
            bg: oklch(0.165, 0.018, 245.0),
            card: oklch(0.205, 0.02, 245.0),
            raised: oklch(0.245, 0.022, 245.0),
            text: oklch(0.95, 0.012, 85.0),
            sub: oklch(0.7, 0.015, 245.0),
            sep: oklcha(1.0, 0.0, 0.0, 0.08),
            brass: oklch(0.8, 0.13, 78.0),
            danger: oklch(0.7, 0.19, 30.0),
            hover: oklcha(1.0, 0.0, 0.0, 0.05),
            scrim: oklcha(0.08, 0.02, 245.0, 0.55),
            shadow: oklcha(0.0, 0.0, 0.0, 0.5),
        }
    } else {
        Pal {
            dark,
            bg: oklch(0.955, 0.014, 85.0),
            card: oklch(0.985, 0.008, 85.0),
            raised: oklch(0.997, 0.004, 85.0),
            text: oklch(0.25, 0.03, 255.0),
            sub: oklch(0.5, 0.02, 255.0),
            sep: oklcha(0.25, 0.03, 255.0, 0.11),
            brass: oklch(0.64, 0.11, 78.0),
            danger: oklch(0.57, 0.19, 30.0),
            hover: oklcha(0.25, 0.03, 255.0, 0.05),
            scrim: oklcha(0.3, 0.03, 85.0, 0.25),
            shadow: oklcha(0.25, 0.04, 60.0, 0.22),
        }
    }
}

/// Course inks: slate, green, ochre, copper, teal, olive, raspberry, sea.
pub fn tone(i: usize, dark: bool) -> Color {
    const HUES: [(f32, f32); 8] =
        [(240.0, 0.09), (160.0, 0.12), (78.0, 0.12), (50.0, 0.13), (195.0, 0.1), (125.0, 0.11), (355.0, 0.13), (215.0, 0.1)];
    let (h, c) = HUES[i % 8];
    oklch(if dark { 0.78 } else { 0.5 }, c, h)
}

/// The same inks as sRGB hex, for shader colours.
pub fn tone_hex(i: usize) -> u32 {
    const HEX: [u32; 8] = [0x5b7aa6, 0x2f9a6c, 0xb8892c, 0xc0673a, 0x2a9aa0, 0x7a8f33, 0xc04a6c, 0x3a86b0];
    HEX[i % 8]
}

pub const CODE: TextStyle = TextStyle::new(12.0, 700.0).mono();
pub const MONO: TextStyle = TextStyle::new(10.5, 500.0).mono();
pub const SMALL: TextStyle = TextStyle::new(10.5, 500.0);
pub const TINY: TextStyle = TextStyle::new(9.0, 800.0).tracking(0.6);
pub const BODY: TextStyle = TextStyle::new(13.0, 450.0);
pub const LABEL: TextStyle = TextStyle::new(12.0, 600.0);
pub const TITLE: TextStyle = TextStyle::new(13.0, 650.0);
pub const H1: TextStyle = TextStyle::new(15.0, 800.0);
pub const STAT: TextStyle = TextStyle::new(12.0, 600.0).mono().features("'tnum' 1");

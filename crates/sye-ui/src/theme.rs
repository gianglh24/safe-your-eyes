//! Cozy Design System tokens for `safe-your-eyes`, adapted from `DESIGN.md` for `nsh`.
//!
//! Uses warm oatmeal/espresso light tones and dark-roast/warm-cream dark tones,
//! eliminating cold `#FFF`/`#000` values and enforcing WCAG AAA contrast on coral CTAs.

use nsh_core::{
    element::{AdaptiveColor, Rgba, rgb, rgba},
    geometry::{Points, px},
};

/// Author support URL shown in Settings (About & Support) and documentation.
pub const SUPPORT_URL: &str = "https://buymeacoffee.com/bbeeezzzzz";

/// Converts `#rrggbb` + opacity into an `Rgba` value.
pub(crate) const fn rgb_a(hex: u32, alpha: f32) -> Rgba {
    let a = (alpha * 255.0 + 0.5) as u32;
    rgba((hex << 8) | (a & 0xFF))
}

/// Pairs light and dark `Rgba` tones into an `AdaptiveColor`.
const fn tone(light: Rgba, dark: Rgba) -> AdaptiveColor {
    AdaptiveColor {
        light: light.to_hsla(),
        dark: dark.to_hsla(),
    }
}

/// Semantic Cozy palette tokens (Light / Dark).
pub(crate) mod colors {
    use super::{AdaptiveColor, rgb, rgb_a, rgba, tone};

    /// Main canvas background (oatmeal `#FAEBDA` / dark roast `#18140E`).
    pub(crate) const BACKGROUND_BODY: AdaptiveColor = tone(rgb(0xFAEBDA), rgb(0x18140E));
    /// Card, panel, and sticky toolbar surface (`#FBF1E1` / `#231D16`).
    pub(crate) const BACKGROUND_SURFACE: AdaptiveColor = tone(rgb(0xFBF1E1), rgb(0x231D16));
    /// Muted track, input fill, and subtle highlight (`#F3DFC8` / `#30261D`).
    pub(crate) const BACKGROUND_MUTED: AdaptiveColor = tone(rgb(0xF3DFC8), rgb(0x30261D));

    /// Floating Sage Sidebar background (`#C3CBB9` / `#29332A`).
    pub(crate) const SIDEBAR_BG: AdaptiveColor = tone(rgb(0xC3CBB9), rgb(0x29332A));
    /// Active navigation pill inside the Floating Sage Sidebar (`#F8E8D3` / `#404A41`).
    pub(crate) const SIDEBAR_ACTIVE: AdaptiveColor = tone(rgb(0xF8E8D3), rgb(0x404A41));

    /// Primary coral CTA accent (`#EF8F63` / `#EE8F64`).
    pub(crate) const ACCENT: AdaptiveColor = tone(rgb(0xEF8F63), rgb(0xEE8F64));
    /// WCAG AAA dark espresso text on `ACCENT` (`#261101` / `#18140E`, ~7.6:1 contrast).
    pub(crate) const ON_ACCENT: AdaptiveColor = tone(rgb(0x261101), rgb(0x18140E));

    /// Warm primary text (`#261101` / `#FFFEF7`).
    pub(crate) const TEXT_PRIMARY: AdaptiveColor = tone(rgb(0x261101), rgb(0xFFFEF7));
    /// Warm secondary text (`#5B4C3B` / `#C2B7B1`).
    pub(crate) const TEXT_SECONDARY: AdaptiveColor = tone(rgb(0x5B4C3B), rgb(0xC2B7B1));
    /// Warm muted/disabled text (`#968574` / `#70655C`).
    pub(crate) const TEXT_DISABLED: AdaptiveColor = tone(rgb(0x968574), rgb(0x70655C));

    /// Semantic status colors.
    pub(crate) const WARNING: AdaptiveColor = tone(rgb(0xD99220), rgb(0xF5C156));
    pub(crate) const ERROR: AdaptiveColor = tone(rgb(0xC8433B), rgb(0xF27B70));

    /// Soft tinted backgrounds for status banners and destructive buttons.
    pub(crate) const WARNING_SOFT: AdaptiveColor =
        tone(rgb_a(0xD99220, 0.14), rgb_a(0xF5C156, 0.16));
    pub(crate) const ERROR_SOFT: AdaptiveColor = tone(rgb_a(0xC8433B, 0.12), rgb_a(0xF27B70, 0.16));
    pub(crate) const ACCENT_SOFT: AdaptiveColor =
        tone(rgb_a(0xEF8F63, 0.18), rgb_a(0xEE8F64, 0.18));

    /// Hairline and emphasized borders.
    pub(crate) const BORDER: AdaptiveColor = tone(rgba(0x5B4C3B22), rgb(0x3E3831));
    pub(crate) const BORDER_EMPHASIZED: AdaptiveColor = tone(rgb(0xDFCBB5), rgb(0x52493F));

    /// Switch knob color.
    pub(crate) const SWITCH_KNOB: AdaptiveColor = tone(rgb(0xFFFEF7), rgb(0xFFFEF7));

    /// Overlay always uses restful dark-roast tones so eyes relax during breaks.
    pub(crate) const OVERLAY_CANVAS: AdaptiveColor = tone(rgb(0x18140E), rgb(0x18140E));
    pub(crate) const OVERLAY_CARD: AdaptiveColor = tone(rgb(0x231D16), rgb(0x231D16));
    pub(crate) const OVERLAY_TEXT: AdaptiveColor = tone(rgb(0xFFFEF7), rgb(0xFFFEF7));
    pub(crate) const OVERLAY_MUTED: AdaptiveColor = tone(rgb(0xC2B7B1), rgb(0xC2B7B1));
    pub(crate) const OVERLAY_TRACK: AdaptiveColor = tone(rgb(0x30261D), rgb(0x30261D));
}

/// Pastel KPI card color pairs (Sage, Peach, Butter, Mist).
pub(crate) mod kpi {
    use super::{AdaptiveColor, rgb, tone};

    pub(crate) const SAGE_BG: AdaptiveColor = tone(rgb(0xE2DEC4), rgb(0x31392C));
    pub(crate) const SAGE_ACCENT: AdaptiveColor = tone(rgb(0x7EAB9A), rgb(0x5E8C7A));

    pub(crate) const PEACH_BG: AdaptiveColor = tone(rgb(0xFBCBB4), rgb(0x432F28));
    pub(crate) const PEACH_ACCENT: AdaptiveColor = tone(rgb(0xF48A65), rgb(0xE5866B));

    pub(crate) const BUTTER_BG: AdaptiveColor = tone(rgb(0xFAE1B5), rgb(0x443A26));
    pub(crate) const BUTTER_ACCENT: AdaptiveColor = tone(rgb(0xFAC76A), rgb(0xD9A74E));

    pub(crate) const MIST_BG: AdaptiveColor = tone(rgb(0xD2D7DA), rgb(0x2B333D));
    pub(crate) const MIST_ACCENT: AdaptiveColor = tone(rgb(0xA2B3C1), rgb(0x6D8296));
}

/// Cozy border-radius scale matching `DESIGN.md`.
pub(crate) mod radii {
    use super::{Points, px};

    pub(crate) const ELEMENT: Points = px(12.0);
    pub(crate) const CONTAINER: Points = px(18.0);
    pub(crate) const SIDEBAR: Points = px(22.0);
    pub(crate) const PAGE: Points = px(28.0);
    pub(crate) const PILL: Points = px(9999.0);
}

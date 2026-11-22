//! Reusable Cozy styled UI primitives built on `nsh_base` and `theme`.

use nsh_base::{button, input, link, switch};
use nsh_core::prelude::*;

use crate::theme::{SUPPORT_URL, colors, radii};

/// Pastel KPI card color variant matching `DESIGN.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KpiTone {
    Sage,
    Peach,
    Butter,
    Mist,
}

impl KpiTone {
    fn bg(self) -> AdaptiveColor {
        match self {
            Self::Sage => crate::theme::kpi::SAGE_BG,
            Self::Peach => crate::theme::kpi::PEACH_BG,
            Self::Butter => crate::theme::kpi::BUTTER_BG,
            Self::Mist => crate::theme::kpi::MIST_BG,
        }
    }

    fn accent(self) -> AdaptiveColor {
        match self {
            Self::Sage => crate::theme::kpi::SAGE_ACCENT,
            Self::Peach => crate::theme::kpi::PEACH_ACCENT,
            Self::Butter => crate::theme::kpi::BUTTER_ACCENT,
            Self::Mist => crate::theme::kpi::MIST_ACCENT,
        }
    }
}

/// Cozy primary pill button (warm coral fill + WCAG AAA dark espresso text).
pub(crate) fn primary_button(
    label: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .on_press(on_press)
        .h(px(36.0))
        .px_4()
        .items_center()
        .justify_center()
        .rounded(radii::PILL)
        .bg(colors::ACCENT)
        .hover(|s| s.opacity(0.92))
        .active(|s| s.opacity(0.84))
        .disabled_style(|s| s.opacity(0.45))
        .child(
            text(label)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors::ON_ACCENT),
        )
}

/// Cozy secondary pill button (warm muted surface + emphasized border).
pub(crate) fn secondary_button(
    label: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .on_press(on_press)
        .h(px(36.0))
        .px_4()
        .items_center()
        .justify_center()
        .rounded(radii::PILL)
        .bg(colors::BACKGROUND_MUTED)
        .border_1()
        .border_color(colors::BORDER_EMPHASIZED)
        .hover(|s| s.opacity(0.9))
        .active(|s| s.opacity(0.8))
        .disabled_style(|s| s.opacity(0.45))
        .child(
            text(label)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(colors::TEXT_PRIMARY),
        )
}

/// Cozy ghost pill button for tertiary/cancel actions.
pub(crate) fn ghost_button(
    label: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .on_press(on_press)
        .h(px(36.0))
        .px_3()
        .items_center()
        .justify_center()
        .rounded(radii::PILL)
        .hover(|s| s.bg(colors::BACKGROUND_MUTED))
        .active(|s| s.opacity(0.8))
        .disabled_style(|s| s.opacity(0.45))
        .child(
            text(label)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(colors::TEXT_SECONDARY),
        )
}

/// Cozy destructive pill button for remove/delete actions.
pub(crate) fn destructive_button(
    label: impl IntoReactive<SharedString>,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .on_press(on_press)
        .h(px(30.0))
        .px_3()
        .items_center()
        .justify_center()
        .rounded(radii::PILL)
        .bg(colors::ERROR_SOFT)
        .hover(|s| s.opacity(0.88))
        .active(|s| s.opacity(0.76))
        .child(
            text(label)
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors::ERROR),
        )
}

/// Warm rounded Cozy card container (`18px` radius).
pub(crate) fn cozy_card() -> View {
    view()
        .rounded(radii::CONTAINER)
        .bg(colors::BACKGROUND_SURFACE)
        .border_1()
        .border_color(colors::BORDER)
        .p_4()
        .gap_3()
}

/// Pastel KPI summary card (`18px` radius) with accent indicator dot, label, and metric.
pub(crate) fn pastel_kpi_card(
    tone: KpiTone,
    label: &'static str,
    value: impl IntoReactive<SharedString>,
) -> impl IntoElement {
    view()
        .flex_1()
        .rounded(radii::CONTAINER)
        .bg(tone.bg())
        .p_3()
        .gap_1()
        .child(
            view()
                .flex_row()
                .items_center()
                .gap_2()
                .child(
                    view()
                        .w(px(8.0))
                        .h(px(8.0))
                        .rounded(radii::PILL)
                        .bg(tone.accent()),
                )
                .child(
                    text(label)
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(colors::TEXT_SECONDARY),
                ),
        )
        .child(
            text(value)
                .text_lg()
                .font_weight(FontWeight::BOLD)
                .text_color(colors::TEXT_PRIMARY),
        )
}

/// Cozy toggle switch with warm track and coral active fill.
pub(crate) fn cozy_switch(
    label: &'static str,
    checked: impl Fn() -> bool + 'static,
    on_change: impl Fn(bool) + 'static,
) -> impl IntoElement {
    view()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_3()
        .py_1()
        .child(
            text(label)
                .text_sm()
                .font_weight(FontWeight::MEDIUM)
                .text_color(colors::TEXT_PRIMARY),
        )
        .child(
            switch()
                .checked(checked)
                .on_change(move |val: &bool| on_change(*val))
                .accessibility_label(label)
                .track_style(|s| {
                    s.w(px(42.0))
                        .h(px(24.0))
                        .rounded(radii::PILL)
                        .bg(colors::BACKGROUND_MUTED)
                        .border_1()
                        .border_color(colors::BORDER_EMPHASIZED)
                        .checked(|c| c.bg(colors::ACCENT).border_color(colors::ACCENT))
                })
                .thumb_style(|s| {
                    s.w(px(18.0))
                        .h(px(18.0))
                        .rounded(radii::PILL)
                        .bg(colors::SWITCH_KNOB)
                }),
        )
}

/// Cozy styled single-line text input (`12px` radius).
pub(crate) fn cozy_input(
    placeholder: &'static str,
    value: impl Fn() -> String + 'static,
    on_change: impl Fn(&str) + 'static,
) -> impl IntoElement {
    input()
        .value(value)
        .on_change(on_change)
        .placeholder(placeholder)
        .placeholder_color(|| color(colors::TEXT_DISABLED))
        .caret_color(|| color(colors::ACCENT))
        .h(px(36.0))
        .px_3()
        .text_sm()
        .text_color(colors::TEXT_PRIMARY)
        .rounded(radii::ELEMENT)
        .bg(colors::BACKGROUND_MUTED)
        .border_1()
        .border_color(colors::BORDER_EMPHASIZED)
        .focus(|s| s.border_color(colors::ACCENT))
}

/// Sidebar navigation pill inside the Floating Sage Sidebar.
pub(crate) fn sidebar_nav_item(
    label: &'static str,
    selected: impl Fn() -> bool + 'static,
    on_press: impl Fn() + 'static,
) -> impl IntoElement {
    button()
        .on_press(on_press)
        .selected(selected)
        .h(px(38.0))
        .px_3()
        .items_start()
        .justify_center()
        .rounded(radii::ELEMENT)
        .hover(|s| s.bg(colors::SIDEBAR_ACTIVE).opacity(0.85))
        .selected_style(|s| s.bg(colors::SIDEBAR_ACTIVE))
        .child(
            text(label)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors::TEXT_PRIMARY),
        )
}

/// Warm status banner for warnings or validation errors.
pub(crate) fn status_banner(
    is_error: bool,
    message: impl IntoReactive<SharedString>,
) -> impl IntoElement {
    let (bg, fg) = if is_error {
        (colors::ERROR_SOFT, colors::ERROR)
    } else {
        (colors::WARNING_SOFT, colors::WARNING)
    };
    view().rounded(radii::ELEMENT).bg(bg).px_3().py_2().child(
        text(message)
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(fg),
    )
}

/// Direct Buy Me a Coffee link styled as a Cozy coral pill CTA.
pub(crate) fn support_link(label: &'static str) -> impl IntoElement {
    link()
        .href(SUPPORT_URL)
        .h(px(38.0))
        .px_4()
        .items_center()
        .justify_center()
        .rounded(radii::PILL)
        .bg(colors::ACCENT)
        .hover(|s| s.opacity(0.92))
        .active(|s| s.opacity(0.84))
        .child(
            text(label)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(colors::ON_ACCENT),
        )
}

/// Encodes `SUPPORT_URL` into a square boolean QR matrix (`true` = dark module).
pub(crate) fn build_support_qr_matrix() -> Option<(usize, Vec<bool>)> {
    let code = qrcode::QrCode::new(SUPPORT_URL.as_bytes()).ok()?;
    let width = code.width();
    let modules = code
        .to_colors()
        .into_iter()
        .map(|c| c == qrcode::Color::Dark)
        .collect();
    Some((width, modules))
}

/// Scannable Buy Me a Coffee QR code component with Cozy warm cream quiet zone and espresso modules.
pub(crate) fn support_qr_card() -> impl IntoElement {
    let qr_bg = rgb(0xFFFEF7);
    let qr_fg = rgb(0x261101);
    let module_px = 4.0_f32;

    let mut grid = view().gap(px(0.0));
    if let Some((width, modules)) = build_support_qr_matrix() {
        for row_idx in 0..width {
            let mut row = view().flex_row().gap(px(0.0));
            for col_idx in 0..width {
                let is_dark = modules
                    .get(row_idx * width + col_idx)
                    .copied()
                    .unwrap_or(false);
                row = row.child(view().w(px(module_px)).h(px(module_px)).bg(if is_dark {
                    qr_fg
                } else {
                    qr_bg
                }));
            }
            grid = grid.child(row);
        }
    }

    view()
        .items_center()
        .gap_2()
        .child(
            view()
                .p_3()
                .rounded(radii::CONTAINER)
                .bg(qr_bg)
                .border_1()
                .border_color(colors::BORDER_EMPHASIZED)
                .child(grid),
        )
        .child(
            text("Scan to Buy Me a Coffee")
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(colors::TEXT_SECONDARY),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_qr_matrix_encodes_buy_me_a_coffee_url() {
        let matrix = build_support_qr_matrix();
        assert!(matrix.is_some());
        if let Some((width, modules)) = matrix {
            assert!(width >= 21, "QR code width must be at least Version 1 (21)");
            assert_eq!(modules.len(), width * width);
            // Top-left finder pattern corner (0, 0) is always dark in standard QR codes.
            assert_eq!(modules.first().copied(), Some(true));
        }
    }
}

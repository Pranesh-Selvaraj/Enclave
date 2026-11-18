//! Enclave's shared visual language for the desktop shell.
//!
//! The palette and lockup follow the Android app and `docs/brand/`: warm ink,
//! paper text, a single violet accent and the official keyhole lockup. The
//! lockup is the official vector source, rasterised at the exact display size.

use std::sync::Arc;

use eframe::egui;
use egui::{
    Color32, FontData, FontDefinitions, FontFamily, FontId, Frame, Image, Margin, Response,
    RichText, Stroke, TextStyle, Ui, Vec2,
};

pub const BG: Color32 = Color32::from_rgb(0x12, 0x12, 0x11);
pub const SURFACE: Color32 = Color32::from_rgb(0x1B, 0x1A, 0x19);
pub const SURFACE_VARIANT: Color32 = Color32::from_rgb(0x24, 0x23, 0x22);
pub const PAPER: Color32 = Color32::from_rgb(0xEC, 0xE9, 0xE4);
pub const MUTED: Color32 = Color32::from_rgb(0xB9, 0xB4, 0xAC);
pub const ACCENT: Color32 = Color32::from_rgb(0xA9, 0x9B, 0xFF);
pub const ON_ACCENT: Color32 = Color32::from_rgb(0x1B, 0x1A, 0x18);
pub const OUTLINE: Color32 = Color32::from_rgb(0x3A, 0x38, 0x35);
pub const ERROR: Color32 = Color32::from_rgb(0xF2, 0xB8, 0xB5);

pub const LOCKUP_RATIO: f32 = 347.1 / 48.0;

pub fn semibold() -> FontFamily {
    FontFamily::Name("enclave-sans-semibold".into())
}

pub fn medium() -> FontFamily {
    FontFamily::Name("enclave-sans-medium".into())
}

/// Install Inter, JetBrains Mono and the Enclave palette into an egui context.
pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    for (name, bytes) in [
        (
            "enclave-sans-regular",
            include_bytes!("../assets/fonts/Inter-Regular.ttf").as_slice(),
        ),
        (
            "enclave-sans-medium",
            include_bytes!("../assets/fonts/Inter-Medium.ttf").as_slice(),
        ),
        (
            "enclave-sans-semibold",
            include_bytes!("../assets/fonts/Inter-SemiBold.ttf").as_slice(),
        ),
        (
            "enclave-sans-bold",
            include_bytes!("../assets/fonts/Inter-Bold.ttf").as_slice(),
        ),
        (
            "enclave-mono",
            include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf").as_slice(),
        ),
    ] {
        fonts
            .font_data
            .insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "enclave-sans-regular".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "enclave-mono".to_owned());
    fonts.families.insert(
        semibold(),
        vec![
            "enclave-sans-semibold".to_owned(),
            "enclave-sans-regular".to_owned(),
        ],
    );
    fonts.families.insert(
        medium(),
        vec![
            "enclave-sans-medium".to_owned(),
            "enclave-sans-regular".to_owned(),
        ],
    );
    ctx.set_fonts(fonts);

    ctx.set_theme(egui::ThemePreference::Dark);
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.visuals = visuals();
    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(14.5, FontFamily::Name("enclave-sans-medium".into())),
    );
    style
        .text_styles
        .insert(TextStyle::Heading, FontId::new(26.0, semibold()));
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 9.0);
    ctx.set_style_of(egui::Theme::Dark, style.clone());
    ctx.set_style_of(egui::Theme::Light, style);
}

fn visuals() -> egui::Visuals {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = SURFACE_VARIANT;
    visuals.faint_bg_color = SURFACE_VARIANT;
    visuals.override_text_color = Some(PAPER);
    visuals.hyperlink_color = ACCENT;
    visuals.error_fg_color = ERROR;
    visuals.warn_fg_color = Color32::from_rgb(0xFB, 0xBF, 0x24);
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.noninteractive.bg_fill = BG;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, OUTLINE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, MUTED);
    visuals.widgets.inactive.bg_fill = SURFACE_VARIANT;
    visuals.widgets.inactive.weak_bg_fill = SURFACE_VARIANT;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, OUTLINE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, PAPER);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x2E, 0x2C, 0x29);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x2E, 0x2C, 0x29);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT.gamma_multiply(0.75));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, PAPER);
    visuals.widgets.active.bg_fill = Color32::from_rgb(0x33, 0x31, 0x2D);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, PAPER);
    visuals.widgets.open.bg_fill = SURFACE_VARIANT;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, OUTLINE);
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
    ] {
        widget.corner_radius = egui::CornerRadius::same(9);
    }
    visuals
}

/// A soft radial falloff stored as a texture so the glow has no rings or
/// polygon edges when it is stretched behind the lockup.
pub fn load_glow(ctx: &egui::Context) -> egui::TextureHandle {
    const SIZE: usize = 256;
    let mut pixels = Vec::with_capacity(SIZE * SIZE * 4);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = (x as f32 + 0.5) / SIZE as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / SIZE as f32 * 2.0 - 1.0;
            let distance = (dx * dx + dy * dy).sqrt();
            let alpha = ((1.0 - distance).max(0.0).powf(2.2) * 255.0) as u8;
            pixels.extend_from_slice(&[255, 255, 255, alpha]);
        }
    }
    let image = egui::ColorImage::from_rgba_unmultiplied([SIZE, SIZE], &pixels);
    ctx.load_texture("enclave-glow", image, egui::TextureOptions::LINEAR)
}

/// Paint the official vector lockup, with the accent glow used on Android's gate.
pub fn lockup(
    ui: &mut Ui,
    glow_texture: egui::TextureId,
    width: f32,
    glow_alpha: f32,
    opacity: f32,
) {
    let height = width / LOCKUP_RATIO;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width * 1.75, height * 2.3), egui::Sense::hover());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, "Enclave lockup"));
    if glow_alpha > 0.001 {
        let glow_size = Vec2::splat(width * 1.75);
        let glow_rect = egui::Rect::from_center_size(rect.center(), glow_size);
        Image::from_texture(egui::load::SizedTexture::new(glow_texture, glow_size))
            .tint(Color32::from_rgba_unmultiplied(
                ACCENT.r(),
                ACCENT.g(),
                ACCENT.b(),
                (glow_alpha * 255.0) as u8,
            ))
            .paint_at(ui, glow_rect);
    }
    let image_rect = egui::Rect::from_center_size(rect.center(), Vec2::new(width, height));
    Image::new(egui::include_image!("../assets/brand/lockup-dark.svg"))
        .tint(Color32::from_white_alpha((opacity * 255.0) as u8))
        .paint_at(ui, image_rect);
}

/// A labelled, outlined password/text field matching the Android gate.
pub fn field(
    ui: &mut Ui,
    id: egui::Id,
    label: &str,
    value: &mut String,
    password: bool,
) -> Response {
    let label = ui.label(RichText::new(label).size(12.5).color(MUTED));
    let frame = Frame::new()
        .fill(SURFACE_VARIANT)
        .stroke(Stroke::new(1.0, OUTLINE))
        .corner_radius(10)
        .inner_margin(Margin::symmetric(12, 11));
    let inner = frame.show(ui, |ui| {
        ui.set_min_height(24.0);
        ui.add(
            egui::TextEdit::singleline(value)
                .id(id)
                .password(password)
                .frame(Frame::NONE)
                .desired_width(f32::INFINITY)
                .hint_text(if password { "Enter your password" } else { "" })
                .font(FontId::new(15.5, FontFamily::Proportional)),
        )
    });
    if inner.inner.has_focus() {
        ui.painter().rect_stroke(
            inner.response.rect,
            egui::CornerRadius::same(10),
            Stroke::new(1.0, ACCENT),
            egui::StrokeKind::Outside,
        );
    }
    ui.add_space(9.0);
    inner.inner.labelled_by(label.id)
}

/// The full-width violet action button used on the gate and recovery screen.
pub fn primary_button(ui: &mut Ui, label: &str, enabled: bool, width: f32) -> bool {
    let fill = if enabled { ACCENT } else { SURFACE_VARIANT };
    let text = if enabled { ON_ACCENT } else { MUTED };
    ui.add_enabled_ui(enabled, |ui| {
        ui.add_sized(
            [width, 50.0],
            egui::Button::new(
                RichText::new(label)
                    .size(15.0)
                    .family(semibold())
                    .color(text),
            )
            .fill(fill)
            .stroke(Stroke::NONE)
            .corner_radius(10),
        )
    })
    .inner
    .clicked()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_packaged_lockup_is_vector() {
        let svg = include_str!("../assets/brand/lockup-dark.svg");
        assert!(svg.contains("<path"));
        assert!(!svg.contains("<text"));
    }

    #[test]
    fn the_password_field_exposes_its_label_to_accesskit() {
        let context = egui::Context::default();
        context.enable_accesskit();
        let mut value = String::new();
        let mut field_id = None;
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            let response = field(
                ui,
                egui::Id::unique("accesskit-password"),
                "Password",
                &mut value,
                true,
            );
            field_id = Some(response.id);
        });
        output.textures_delta.clear();
        let update = output
            .platform_output
            .accesskit_update
            .expect("accesskit tree update");
        let id = field_id.expect("field id").accesskit_id();
        let node = update
            .nodes
            .iter()
            .find(|(node_id, _)| *node_id == id)
            .map(|(_, node)| node)
            .expect("password field node");
        assert_eq!(node.role(), egui::accesskit::Role::PasswordInput);
        assert!(!node.labelled_by().is_empty());
    }
}

/*!
Author: Andres Schumann, aschumann at virtualcable dot es
*/
use eframe::egui::{Color32, Context, Margin, Stroke, ThemePreference, Visuals, vec2};

pub const ACCENT: Color32 = Color32::from_rgb(0x00, 0x55, 0xaa);
pub const MUTED: Color32 = Color32::from_rgb(0x66, 0x66, 0x66);
pub const ERROR: Color32 = Color32::from_rgb(0xaa, 0x00, 0x00);
pub const WHITE: Color32 = Color32::WHITE;
pub const TEXT: Color32 = Color32::from_rgb(0x1f, 0x1f, 0x1f);

pub const PANEL_PADDING: f32 = 24.0;
pub const SPACING: f32 = 12.0;
pub const LABEL_WIDTH: f32 = 140.0;
pub const ROW_HEIGHT: f32 = 32.0;
pub const BUTTON_HEIGHT: f32 = 34.0;
pub const BUTTON_WIDTH: f32 = 100.0;
pub const TITLE_SIZE: f32 = 22.0;
pub const STATUS_SIZE: f32 = 13.0;
pub const LOGO_SIZE: f32 = 48.0;

pub fn panel_margin() -> Margin {
    Margin::same(PANEL_PADDING as i8)
}

pub fn apply(ctx: &Context) {
    ctx.set_theme(ThemePreference::Light);
    ctx.set_visuals(Visuals::light());
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = vec2(SPACING, SPACING);
        style.visuals.selection.bg_fill = ACCENT;
        style.visuals.selection.stroke = Stroke::new(1.0, WHITE);
        style.visuals.override_text_color = Some(TEXT);
        style.visuals.hyperlink_color = ACCENT;
    });
}

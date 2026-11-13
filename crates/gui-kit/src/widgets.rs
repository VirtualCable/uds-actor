/*!
Author: Andres Schumann, aschumann at virtualcable dot es
*/
use eframe::egui::{self, Button, Id, Image, RichText, TextureHandle, Ui, vec2};

use crate::status::{Level, Status};
use crate::theme;

pub fn header(ui: &mut Ui, logo: &TextureHandle, title: &str) {
    ui.horizontal(|ui| {
        ui.add(Image::new(logo).fit_to_exact_size(vec2(theme::LOGO_SIZE, theme::LOGO_SIZE)));
        ui.label(
            RichText::new(title)
                .size(theme::TITLE_SIZE)
                .strong()
                .color(theme::ACCENT),
        );
    });
}

pub fn row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui, Id) -> R) -> R {
    ui.horizontal(|ui| {
        let label_id = ui
            .add_sized(
                vec2(theme::LABEL_WIDTH, theme::ROW_HEIGHT),
                egui::Label::new(label),
            )
            .id;
        add(ui, label_id)
    })
    .inner
}

pub fn primary_button(text: &str, width: f32) -> Button<'_> {
    Button::new(RichText::new(text).color(theme::WHITE))
        .fill(theme::ACCENT)
        .min_size(vec2(width, theme::BUTTON_HEIGHT))
}

pub fn button(text: &str, width: f32) -> Button<'_> {
    Button::new(text).min_size(vec2(width, theme::BUTTON_HEIGHT))
}

pub fn status_line(ui: &mut Ui, status: &Status) {
    let color = match status.level {
        Level::Error => theme::ERROR,
        Level::Busy => theme::ACCENT,
        Level::Ready => theme::MUTED,
    };
    ui.vertical_centered(|ui| {
        if status.is_busy() {
            ui.spinner();
        }
        ui.label(
            RichText::new(&status.text)
                .size(theme::STATUS_SIZE)
                .color(color),
        );
    });
}

pub fn about(ui: &mut Ui, product: &str, version: &str) {
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(product)
                .size(theme::TITLE_SIZE)
                .strong()
                .color(theme::ACCENT),
        );
        ui.label(format!("Version {version}"));
        ui.label("Copyright (c) Virtual Cable S.L.U.");
    });
}

pub fn warning(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).strong().color(theme::ERROR));
}

pub fn hint(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).color(theme::MUTED));
}

pub fn section_title(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).strong());
}

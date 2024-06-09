use egui::{
    include_image, Button, Color32, CursorIcon, ImageSource, Response, RichText, Sense, Vec2,
};
use egui_extras::{Column, TableBuilder};
use egui_toast::ToastKind;

use super::{
    log_file_reader::LogFileReader,
    log_view::{LogViewTabTrait, LogViewerState},
};

pub struct LogEntryContextTab {}

impl LogEntryContextTab {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }

    fn add_tool_button(
        ui: &mut egui::Ui,
        image_source: ImageSource<'_>,
        hover_text: &str,
    ) -> Response {
        ui.add_sized(
            Vec2::new(14.0, 14.0),
            Button::image(image_source).frame(false),
        )
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(hover_text)
    }
}

impl LogViewTabTrait for LogEntryContextTab {
    fn title(&self) -> egui::WidgetText {
        "Context".into()
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        log_reader: &mut LogFileReader,
        viewer_state: &mut LogViewerState,
    ) {
        if viewer_state.selected_line_num.is_none() {
            ui.label("Select an entry.");
            return;
        }

        let read_log_entry = log_reader.read_entry(viewer_state.selected_line_num.unwrap());
        if read_log_entry.is_none() {
            ui.label("Failed to read entry.");
            return;
        }

        let log_entry = read_log_entry.unwrap();

        TableBuilder::new(ui)
            .striped(true)
            .sense(Sense::hover())
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::auto().at_least(60.0))
            .column(Column::auto())
            .column(Column::remainder())
            .body(|mut body| {
                for entry in log_entry.object.entries() {
                    let key_str = entry.0;
                    let value_str = entry.1.to_string();
                    let line_count = value_str.chars().filter(|c| *c == '\n').count() + 1;
                    body.row((line_count as f32) * 16.0, |mut row| {
                        row.col(|ui| {
                            let column_is_shown =
                                viewer_state.displayed_columns.iter().any(|s| s == key_str);
                            if !column_is_shown {
                                let add_icon =
                                    include_image!("../../assets/icons8-add-48-white.png");
                                if Self::add_tool_button(ui, add_icon, "Add Column").clicked() {
                                    viewer_state.displayed_columns.push(key_str.to_string());

                                    viewer_state.add_toast(
                                        ToastKind::Info,
                                        format!("Added column '{}'", key_str).into(),
                                        2.0,
                                    );
                                }
                            }
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(key_str).color(Color32::WHITE).monospace());
                        });
                        row.col(|ui| {
                            let copy_icon = include_image!("../../assets/icons8-copy-48-white.png");
                            if Self::add_tool_button(ui, copy_icon, "Copy Value").clicked() {
                                ui.output_mut(|o| {
                                    o.copied_text = value_str.clone();
                                });

                                viewer_state.add_toast(
                                    ToastKind::Info,
                                    "Copied value to clipboard.".into(),
                                    2.0,
                                );
                            }
                        });
                        row.col(|ui| {
                            ui.label(RichText::new(value_str.trim()).monospace());
                        });
                    });
                }
            });
    }
}

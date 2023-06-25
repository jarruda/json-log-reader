use egui::{Color32, RichText};
use egui_extras::{Column, TableBuilder};

use super::{
    log_file_reader::LogFileReader,
    log_view::{LogSelectionState, LogViewTabResponse, LogViewTabTrait},
};

pub struct LogEntryContextTab {}

impl LogEntryContextTab {
    pub fn new() -> Box<Self> {
        Box::new(Self {})
    }
}

impl LogViewTabTrait for LogEntryContextTab {
    fn title(&self) -> egui::WidgetText {
        "Context".into()
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        _log_reader: &mut LogFileReader,
        selection_state: &LogSelectionState,
    ) -> LogViewTabResponse {
        if let Some(log_entry) = selection_state.selected_log_entry.as_ref() {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                TableBuilder::new(ui)
                    .striped(true)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::auto().at_least(60.0))
                    .column(Column::remainder())
                    .body(|mut body| {
                        for entry in log_entry.object.entries() {
                            let key_str = entry.0;
                            let value_str = entry.1.as_str().unwrap_or_default().trim();
                            let line_count = value_str.chars().filter(|c| *c == '\n').count() + 1;
                            body.row((line_count as f32) * 16.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(
                                        RichText::new(key_str).color(Color32::WHITE).monospace(),
                                    );
                                });
                                row.col(|ui| {
                                    ui.label(RichText::new(value_str).monospace());
                                });
                            });
                        }
                    });
            });
        }

        Default::default()
    }
}

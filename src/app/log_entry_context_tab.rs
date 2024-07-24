use super::log_view::{LogViewTabTrait, LogViewerState};
use crate::app::log_source::{LogEntry, LogEntryId, LogSource};
use egui::{Color32, CursorIcon, Response, RichText, Sense, Ui};
use egui_extras::{Column, TableBuilder};
use egui_toast::ToastKind;

pub struct LogEntryContextTab {
    current_selection: (Option<LogEntryId>, usize),
}

impl LogEntryContextTab {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            current_selection: Default::default(),
        })
    }

    fn add_tool_button(ui: &mut Ui, text: &str, hover_text: &str) -> Response {
        ui.button(text)
            .on_hover_text(hover_text)
            .on_hover_cursor(CursorIcon::PointingHand)
    }

    fn ui_entry(&mut self, ui: &mut Ui, entry: &LogEntry, viewer_state: &mut LogViewerState) {
        let row_height_padding = 6.0;
        let row_content_height = 14.0;

        TableBuilder::new(ui)
            .striped(true)
            .min_scrolled_height(0.0)
            .max_scroll_height(f32::INFINITY)
            .auto_shrink(false)
            .sense(Sense::hover())
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::auto().at_least(60.0))
            .column(Column::auto())
            .column(Column::remainder())
            .body(|mut body| {
                for entry in entry.object.entries() {
                    let key_str = entry.0;
                    let value_str = entry.1.to_string();
                    let line_count = value_str.chars().filter(|c| *c == '\n').count() + 1;
                    body.row(
                        (line_count as f32) * row_content_height + row_height_padding,
                        |mut row| {
                            row.col(|ui| {
                                let column_is_shown =
                                    viewer_state.displayed_columns.iter().any(|s| s == key_str);
                                if !column_is_shown {
                                    if Self::add_tool_button(ui, "➕", "Add Column").clicked() {
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
                                if Self::add_tool_button(ui, "🗐", "Copy Value").clicked() {
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
                        },
                    );
                }
            });
    }
}

impl LogViewTabTrait for LogEntryContextTab {
    fn title(&self) -> egui::WidgetText {
        "📓 Context".into()
    }

    fn ui(
        &mut self,
        ui: &mut Ui,
        log_source: &mut dyn LogSource,
        viewer_state: &mut LogViewerState,
    ) {
        if self.current_selection.0 != viewer_state.selected_entry {
            self.current_selection = match viewer_state.selected_entry {
                None => (None, 0),
                Some(ref selected_entry_id) => {
                    match log_source.find_entry_index(selected_entry_id) {
                        None => (None, 0),
                        Some(entry_index) => (viewer_state.selected_entry.clone(), entry_index),
                    }
                }
            };
        }

        if self.current_selection.0.is_none() {
            ui.label("Select an entry.");
            return;
        }

        log_source.use_entry(self.current_selection.1, &mut |e| match e {
            Ok(e) => {
                self.ui_entry(ui, e, viewer_state);
            }
            Err(_) => {
                ui.label("Failed to read entry.");
            }
        });
    }
}

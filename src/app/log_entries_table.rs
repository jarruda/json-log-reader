use std::sync::Arc;

use egui::{Align, Button, Color32, CursorIcon, Response, RichText, Ui};
use egui::Frame;
use egui_extras::{Column, TableBuilder, TableRow};
use egui_toast::ToastKind;

use crate::app::log_view::{ColumnTextColor, LogViewerState};

use super::log_file_reader::{LineNumber, LogFileReader};

pub struct LogEntriesTable {
    selected_line: Option<usize>,
    scroll_to_selected: bool,
    sync_line_selection: bool,
    tail_log: bool,
}

impl LogEntriesTable {
    fn add_tool_button(ui: &mut Ui, text: &str, hover_text: &str) -> Response {
        ui.button(text)
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text(hover_text)
    }

    pub fn new() -> Self {
        Self {
            selected_line: None,
            scroll_to_selected: false,
            sync_line_selection: true,
            tail_log: false,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        log_file_reader: &mut LogFileReader,
        viewer_state: &mut LogViewerState,
        filtered_entries: Option<&[LineNumber]>,
        add_toolbar_contents: impl FnOnce(&mut Ui),
    ) {
        self.toolbar_ui(ui, log_file_reader, viewer_state, add_toolbar_contents);

        let total_rows = match filtered_entries {
            Some(lines) => lines.len(),
            None => log_file_reader.line_count(),
        };

        let mut table_builder = TableBuilder::new(ui)
            .max_scroll_height(f32::INFINITY)
            .cell_layout(egui::Layout::left_to_right(Align::Center))
            .striped(true)
            .auto_shrink(false)
            .min_scrolled_height(0.0)
            .sense(egui::Sense::click());

        let mut col_iter = viewer_state.displayed_columns.iter().peekable();
        while let Some(col_key) = col_iter.next() {
            let is_last_col = col_iter.peek().is_none();
            let col_style = viewer_state
                .column_styles
                .get(col_key)
                .unwrap_or(Default::default());
            let col_desc: Column = if is_last_col {
                Column::remainder()
            } else if col_style.auto_size {
                Column::auto()
            } else {
                Column::initial(150.0).resizable(true).clip(true)
            };
            table_builder = table_builder.column(col_desc);
        }

        if self.tail_log {
            if let Some(row) = self.last_row_index(log_file_reader, filtered_entries) {
                table_builder = table_builder.scroll_to_row(row, Some(Align::BOTTOM));
            }
        }

        if self.sync_line_selection && self.selected_line != viewer_state.selected_line_num {
            if let Some(selected_line) = viewer_state.selected_line_num {
                if let Some(selected_row) = self.find_row_for_line(selected_line, filtered_entries)
                {
                    self.selected_line = viewer_state.selected_line_num;

                    if !self.tail_log {
                        table_builder =
                            table_builder.scroll_to_row(selected_row, Some(Align::Center));
                    }
                }
            }
            self.scroll_to_selected = false;
        }

        table_builder
            .header(24.0, |mut row| {
                let columns_displayed_count = viewer_state.displayed_columns.len();
                let mut columns_to_remove: Vec<String> = vec![];
                let mut from: Option<Arc<String>> = None;
                let mut to: Option<(String, usize)> = None;

                for displayed_column in &viewer_state.displayed_columns {
                    row.col(|ui| {
                        ui.dnd_drop_zone::<String, ()>(Frame::default(), |ui| {
                            let response = ui
                                .dnd_drag_source(
                                    ui.id().with(displayed_column),
                                    displayed_column.to_string(),
                                    |ui| {
                                        ui.set_min_width(50.0);
                                        ui.label(RichText::new(displayed_column).strong());
                                    },
                                )
                                .response;

                            if let (Some(pointer), Some(hovered_payload)) = (
                                ui.input(|i| i.pointer.interact_pos()),
                                response.dnd_hover_payload::<String>(),
                            ) {
                                if &*hovered_payload != displayed_column {
                                    let rect = response.rect;

                                    // Preview insertion:
                                    let stroke = egui::Stroke::new(5.0, Color32::GOLD);

                                    let insert_col_idx = if pointer.x < rect.center().x {
                                        // Insert before
                                        ui.painter().vline(rect.left(), rect.y_range(), stroke);
                                        0
                                    } else {
                                        // Insert after
                                        ui.painter().vline(rect.right(), rect.y_range(), stroke);
                                        1
                                    };

                                    if let Some(dragged_payload) = response.dnd_release_payload() {
                                        // The user dropped onto this item.
                                        from = Some(dragged_payload);
                                        to = Some((displayed_column.clone(), insert_col_idx));
                                    }
                                }
                            }
                        });

                        if columns_displayed_count > 1 {
                            if Self::add_tool_button(ui, "❌", "Remove Column").clicked() {
                                columns_to_remove.push(displayed_column.clone());
                            }
                        }
                    });
                }

                if let (Some(ref from), Some(ref to)) = (from, to) {
                    // Remove dragged column
                    viewer_state
                        .displayed_columns
                        .iter()
                        .position(|c| *c == **from)
                        .map(|i| viewer_state.displayed_columns.remove(i));

                    // Insert dragged column to new location
                    viewer_state
                        .displayed_columns
                        .iter()
                        .position(|c| *c == to.0)
                        .map(|i| {
                            viewer_state
                                .displayed_columns
                                .insert(i + to.1, from.to_string())
                        });
                }

                if !columns_to_remove.is_empty() {
                    viewer_state
                        .displayed_columns
                        .retain(|c| !columns_to_remove.contains(c));

                    viewer_state.add_toast(ToastKind::Info, "Removed column.".into(), 2.0);
                }
            })
            .body(|body| {
                body.rows(16.0, total_rows, |mut row| {
                    let row_idx = row.index();
                    let line_number = match filtered_entries {
                        Some(lines) => lines[row_idx],
                        None => row_idx,
                    };

                    row.set_selected(self.selected_line == Some(line_number));

                    Self::ui_logline(log_file_reader, viewer_state, &mut row, line_number);

                    if row.response().clicked() {
                        self.selected_line = Some(line_number);
                        if self.sync_line_selection {
                            viewer_state.selected_line_num = self.selected_line;
                        }
                    }
                });
            });
    }

    /// Maps a line number to a table row.
    /// If there is a set of filtered lines set, a binary search is performed to
    /// find the correct row. Otherwise, the line number is returned as the row.
    fn find_row_for_line(
        &self,
        line_number: LineNumber,
        filtered_entries: Option<&[LineNumber]>,
    ) -> Option<usize> {
        match filtered_entries {
            Some(lines) => Some(lines.binary_search(&line_number).ok()?),
            None => Some(line_number),
        }
    }

    fn last_row_index(
        &self,
        log_file_reader: &LogFileReader,
        filtered_entries: Option<&[LineNumber]>,
    ) -> Option<usize> {
        match filtered_entries {
            Some(lines) => {
                if lines.is_empty() {
                    None
                } else {
                    Some(lines.len() - 1)
                }
            }
            None => Some(log_file_reader.line_count() - 1),
        }
    }

    fn ui_logline(
        log_file_reader: &mut LogFileReader,
        viewer_state: &mut LogViewerState,
        row: &mut TableRow<'_, '_>,
        line_num: LineNumber,
    ) -> Option<()> {
        let log_line_opt = log_file_reader.read_line(line_num);

        if log_line_opt.is_none() {
            row.col(|ui| {
                ui.label(
                    RichText::new("⚠ Failed to read from log file.")
                        .color(ui.visuals().warn_fg_color),
                );
            });
            return None;
        }

        let log_line = log_line_opt.unwrap();

        match LogFileReader::parse_logline(&log_line) {
            Some(log_entry) => {
                for column_str in &viewer_state.displayed_columns {
                    row.col(|ui| {
                        let column_value = &log_entry.object[column_str];
                        let full_col_text = if column_value.is_empty() {
                            String::new()
                        } else {
                            column_value.to_string()
                        };
                        let mut column_text = if let Some(split) = full_col_text.split_once('\n') {
                            split.0
                        } else {
                            &full_col_text
                        };

                        let column_style = viewer_state
                            .column_styles
                            .get(column_str)
                            .unwrap_or(Default::default());

                        if column_style.trim {
                            column_text = column_text.trim();
                        }

                        let mut rich_text = RichText::new(column_text).monospace();
                        rich_text = match column_style.color {
                            ColumnTextColor::Color(color) => rich_text.color(color),
                            ColumnTextColor::BySeverity => rich_text.color(color_from_loglevel(
                                log_entry.object["level"].as_str().unwrap_or("INFO"),
                            )),
                        };
                        ui.label(rich_text);
                    });
                }
            }
            None => {
                row.col(|ui| {
                    ui.label(
                        RichText::new(log_line.trim())
                            .monospace()
                            .color(Color32::WHITE),
                    );
                });
            }
        }

        Some(())
    }
    fn toolbar_ui(
        &mut self,
        ui: &mut Ui,
        _log_file_reader: &mut LogFileReader,
        _log_viewer_state: &mut LogViewerState,
        add_toolbar_contents: impl FnOnce(&mut Ui) + Sized,
    ) {
        ui.horizontal(|ui| {
            if ui
                .add(Button::new("⏬").selected(self.tail_log))
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text("Tail Log")
                .clicked()
            {
                self.tail_log = !self.tail_log;
            };
            if ui
                .add(Button::new("🔁").selected(self.sync_line_selection))
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text("Sync Selection")
                .clicked()
            {
                self.sync_line_selection = !self.sync_line_selection;
            };

            add_toolbar_contents(ui);
        });
        ui.separator();
    }
}

fn color_from_loglevel(level: &str) -> Color32 {
    match level {
        "ERROR" => Color32::LIGHT_RED,
        "WARNING" => Color32::GOLD,
        "INFO" => Color32::LIGHT_GREEN,
        "DEBUG" => Color32::LIGHT_BLUE,
        "FATAL" => Color32::RED,
        _ => Color32::DEBUG_COLOR,
    }
}

use std::sync::Arc;

use egui::Frame;
use egui::{Align, Button, Color32, CursorIcon, Response, RichText, Ui};
use egui_extras::{Column, TableBuilder, TableRow};
use egui_toast::ToastKind;

use crate::app::log_source::{LogEntryId, LogSource, ReadEntryError};
use crate::app::log_view::{ColumnTextColor, LogViewerState};

pub struct LogEntriesTable {
    selected_entry: Option<LogEntryId>,
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
            selected_entry: None,
            sync_line_selection: true,
            tail_log: false,
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        log_source: &mut dyn LogSource,
        viewer_state: &mut LogViewerState,
        add_toolbar_contents: impl FnOnce(&mut Ui),
    ) {
        self.toolbar_ui(ui, add_toolbar_contents);

        let total_rows = log_source.entry_count();

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
            let entry_count = log_source.entry_count();
            if entry_count > 0 {
                table_builder = table_builder.scroll_to_row(entry_count - 1, Some(Align::BOTTOM));
            }
        }

        if self.sync_line_selection && self.selected_entry != viewer_state.selected_entry {
            if let Some(ref selected_entry) = viewer_state.selected_entry {
                if let Some(selected_row) = log_source.find_entry_index(selected_entry) {
                    self.selected_entry = viewer_state.selected_entry.clone();

                    if !self.tail_log {
                        table_builder =
                            table_builder.scroll_to_row(selected_row, Some(Align::Center));
                    }
                }
            }
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

                    self.ui_logline(log_source, viewer_state, &mut row, row_idx);
                });
            });
    }

    fn ui_logline(
        &mut self,
        log_source: &mut dyn LogSource,
        viewer_state: &mut LogViewerState,
        row: &mut TableRow<'_, '_>,
        entry_index: usize,
    ) {
        log_source.use_entry(entry_index, &mut |entry| match entry {
            Err(e) => match e {
                ReadEntryError::ReadFailure => {
                    row.col(|ui| {
                        ui.label(
                            RichText::new("⚠ Failed to read from log file.")
                                .color(ui.visuals().warn_fg_color),
                        );
                    });
                }
                ReadEntryError::ParseFailure(raw_str) => {
                    row.col(|ui| {
                        ui.label(
                            RichText::new(raw_str.trim())
                                .monospace()
                                .color(Color32::WHITE),
                        );
                    });
                }
            },
            Ok(entry) => {
                // TODO optimize conversion to Id
                let log_entry_id = Some(LogEntryId::from(entry));
                row.set_selected(self.selected_entry == log_entry_id);
                
                for column_str in &viewer_state.displayed_columns {
                    row.col(|ui| {
                        let column_value = &entry.object[column_str];
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
                                entry.object["level"].as_str().unwrap_or("INFO"),
                            )),
                        };
                        ui.label(rich_text);
                    });
                }
                
                if row.response().clicked() {
                    self.selected_entry = log_entry_id;
                    if self.sync_line_selection {
                        viewer_state.selected_entry = self.selected_entry.clone();
                    }
                }
            }
        });
    }
    fn toolbar_ui(&mut self, ui: &mut Ui, add_toolbar_contents: impl FnOnce(&mut Ui) + Sized) {
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

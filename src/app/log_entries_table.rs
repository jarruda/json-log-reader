use egui::{Align, Color32, RichText, Ui};
use egui_extras::{Column, TableBuilder, TableRow};

use crate::app::log_view::{ColumnTextColor, LogViewerState};

use super::log_file_reader::{LineNumber, LogFileReader};

#[derive(Default)]
pub struct LogEntriesResponse {
    pub selected_line_num: Option<LineNumber>,
}

pub struct LogEntriesTable<'a> {
    filtered_lines: Option<&'a [LineNumber]>,
    selected_line: Option<usize>,
    scroll_to_selected: bool,
}

impl<'a> LogEntriesTable<'a> {
    pub fn new() -> Self {
        Self {
            filtered_lines: None,
            selected_line: None,
            scroll_to_selected: false,
        }
    }

    pub fn scroll_to_selected(mut self) -> Self {
        self.scroll_to_selected = true;
        self
    }

    pub fn select_line(mut self, row: Option<usize>) -> Self {
        self.selected_line = row;
        self
    }

    pub fn filtered_lines(mut self, lines: &'a [LineNumber]) -> Self {
        self.filtered_lines = Some(lines);
        self
    }

    pub fn ui(
        &mut self,
        ui: &mut Ui,
        log_file_reader: &mut LogFileReader,
        viewer_state: &mut LogViewerState,
    ) -> LogEntriesResponse {
        let mut response: LogEntriesResponse = Default::default();

        let total_rows = match self.filtered_lines {
            Some(lines) => lines.len(),
            None => log_file_reader.line_count() as usize,
        };

        let mut table_builder = TableBuilder::new(ui)
            .max_scroll_height(f32::INFINITY)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
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

        if self.scroll_to_selected {
            if let Some(selected_line) = self.selected_line {
                if let Some(selected_row) = self.find_row_for_line(selected_line) {
                    table_builder = table_builder.scroll_to_row(selected_row, Some(Align::Center));
                }
            }
        }

        table_builder
            .header(18.0, |mut row| {
                for displayed_column in &viewer_state.displayed_columns {
                    row.col(|ui| {
                        ui.label(RichText::new(displayed_column).strong());
                    });
                }
            })
            .body(|body| {
                body.rows(16.0, total_rows, |mut row| {
                    let row_idx = row.index();
                    let line_number = match self.filtered_lines {
                        Some(lines) => lines[row_idx],
                        None => row_idx,
                    };

                    row.set_selected(self.selected_line == Some(line_number));

                    if let Some(logline_response) =
                        Self::ui_logline(log_file_reader, viewer_state, &mut row, line_number)
                    {
                        if let Some(selected_line_num) = logline_response.selected_line_num {
                            viewer_state.selected_line_num = Some(selected_line_num);
                            response.selected_line_num = Some(selected_line_num);
                        }
                    }
                });
            });

        response
    }

    /// Maps a line number to a table row.
    /// If there is a set of filtered lines set, a binary search is performed to
    /// find the correct row. Otherwise, the line number is returned as the row.
    fn find_row_for_line(&self, line_number: LineNumber) -> Option<usize> {
        match self.filtered_lines {
            Some(lines) => Some(lines.binary_search(&line_number).ok()?),
            None => Some(line_number),
        }
    }

    fn ui_logline(
        log_file_reader: &mut LogFileReader,
        viewer_state: &mut LogViewerState,
        row: &mut TableRow<'_, '_>,
        line_num: LineNumber,
    ) -> Option<LogEntriesResponse> {
        let mut response: LogEntriesResponse = Default::default();
        let log_line = log_file_reader.read_line(line_num)?;

        match LogFileReader::parse_logline(&log_line) {
            Some(log_entry) => {
                for column_str in &viewer_state.displayed_columns {
                    row.col(|ui| {
                        let column_value = &log_entry.object[column_str];
                        let full_col_text = if column_value.is_empty() { String::new() } else { column_value.to_string() };
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
                row.col(|_| {});
                row.col(|_| {});
                row.col(|ui| {
                    ui.label(
                        RichText::new(log_line.trim())
                            .monospace()
                            .color(Color32::WHITE),
                    );
                });
            }
        }

        if row.response().clicked() {
            response = LogEntriesResponse {
                selected_line_num: Some(line_num),
            };
        }

        Some(response)
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

use egui::{Align, Color32, RichText, Ui};
use egui_extras::{Column, TableBuilder, TableRow};

use super::log_file_reader::{LineNumber, LogEntry, LogFileReader};

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

    pub fn ui(&mut self, ui: &mut Ui, log_file_reader: &mut LogFileReader) -> LogEntriesResponse {
        let mut response: LogEntriesResponse = Default::default();

        let total_rows = match self.filtered_lines {
            Some(lines) => lines.len(),
            None => log_file_reader.line_count() as usize,
        };

        let mut table_builder = TableBuilder::new(ui)
            .max_scroll_height(f32::INFINITY)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::initial(150.0).resizable(true).clip(true))
            .column(Column::remainder().clip(true))
            .sense(egui::Sense::click());

        if self.scroll_to_selected {
            if let Some(selected_line) = self.selected_line {
                if let Some(selected_row) = self.find_row_for_line(selected_line) {
                    table_builder = table_builder.scroll_to_row(selected_row, Some(Align::Center));
                }
            }
        }

        table_builder
            .header(18.0, |mut row| {
                row.col(|ui| {
                    ui.label("Time");
                });
                row.col(|ui| {
                    ui.label("Tag");
                });
                row.col(|ui| {
                    ui.label("Message");
                });
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
                        Self::ui_logline(log_file_reader, &mut row, line_number)
                    {
                        if let Some(selected_line_num) = logline_response.selected_line_num {
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
        row: &mut TableRow<'_, '_>,
        line_num: LineNumber
    ) -> Option<LogEntriesResponse> {
        let mut response: LogEntriesResponse = Default::default();
        let log_line = log_file_reader.read_line(line_num)?;

        match LogFileReader::parse_logline(&log_line) {
            Some(log_entry) => {
                let LogEntry { timestamp, object } = log_entry;
                row.col(|ui| {
                    ui.label(RichText::new(timestamp).color(Color32::WHITE).monospace());
                });
                row.col(|ui| {
                    let tag = object["tag"].as_str().unwrap_or_default();
                    ui.label(RichText::new(tag).color(Color32::KHAKI).monospace());
                });
                row.col(|ui| {
                    let full_msg = object["message"].as_str().unwrap_or_default().trim();
                    let msg = if let Some(m) = full_msg.split_once('\n') {
                        m.0
                    } else {
                        full_msg
                    };
                    let level = object["level"].as_str().unwrap_or("FATAL");
                    ui.label(
                        RichText::new(msg.trim())
                            .color(color_from_loglevel(level))
                            .monospace(),
                    );
                });
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

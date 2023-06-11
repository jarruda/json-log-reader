use egui::{Align, Color32, RichText, Ui};
use egui_extras::{Column, TableBuilder, TableRow};
use json::JsonValue;

use super::log_file_reader::LogFileReader;

pub struct LogEntriesTable<'a> {
    log_file_reader: &'a mut LogFileReader,
    selected_line_num: Option<u64>,
    filtered_lines: Option<&'a [u64]>,
}

impl<'a> LogEntriesTable<'a> {
    pub fn new(log_file_reader: &'a mut LogFileReader) -> Self {
        Self {
            log_file_reader,
            selected_line_num: None,
            filtered_lines: None
        }
    }

    pub fn filtered_lines(&mut self, lines: &'a [u64]) -> &mut Self {
        self.filtered_lines = Some(lines);
        self
    }

    pub fn selected_line_num(&mut self, selected_line_num: Option<u64>) -> &mut Self {
        self.selected_line_num = selected_line_num;
        self
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        let total_rows = match self.filtered_lines {
            Some(lines) => lines.len(),
            None => self.log_file_reader.line_count() as usize,
        };

        let table_builder = TableBuilder::new(ui)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::initial(150.0).resizable(true).clip(true))
            .column(Column::remainder().clip(true));

        // TODO: implement scroll-to-row

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
                body.rows(16.0, total_rows, |row_idx, mut row| {
                    let line_number = match self.filtered_lines {
                        Some(lines) => lines[row_idx],
                        None => row_idx as u64,
                    };
                    let selected = Some(line_number) == self.selected_line_num;
                    Self::ui_logline(&mut self.log_file_reader, &mut row, line_number, selected);
                });
            });
    }

    fn ui_logline(
        log_file_reader: &mut LogFileReader,
        row: &mut TableRow<'_, '_>,
        line_num: u64,
        selected: bool,
    ) -> Option<()> {
        let line = log_file_reader.line(line_num)?;
        match parse_logline(&line) {
            Some((time, log_entry)) => {
                row.col(|ui| {
                    ui.label(RichText::new(time).color(Color32::WHITE).monospace());
                });
                row.col(|ui| {
                    let tag = log_entry["tag"].as_str().unwrap_or_default();
                    ui.label(RichText::new(tag).color(Color32::KHAKI).monospace());
                });
                row.col(|ui| {
                    let full_msg = log_entry["message"].as_str().unwrap_or_default().trim();
                    let msg = if let Some(m) = full_msg.split_once('\n') {
                        m.0
                    } else {
                        full_msg
                    };
                    let level = log_entry["level"].as_str().unwrap_or("FATAL");

                    if ui
                        .selectable_label(
                            selected,
                            RichText::new(msg.trim())
                                .color(color_from_loglevel(level))
                                .monospace(),
                        )
                        .clicked()
                    {
                        // TODO: selection out
                        // self.select_row(Some(line_num), Some(log_entry));
                    }
                });
            }
            None => {
                row.col(|_| {});
                row.col(|_| {});
                row.col(|ui| {
                    ui.label(RichText::new(line.trim()).monospace().color(Color32::WHITE));
                });
            }
        }

        Some(())
    }
}

fn parse_logline(line: &str) -> Option<(&str, JsonValue)> {
    let split_idx = line.find(' ')?;
    let (timestamp, json_content) = line.split_at(split_idx);
    let log_entry = json::parse(json_content).ok()?;

    if log_entry.is_object() {
        Some((timestamp, log_entry))
    } else {
        None
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

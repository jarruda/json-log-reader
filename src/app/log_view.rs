use std::{
    io,
    path::{Path, PathBuf},
};

use egui::{Align, Color32, RichText, Ui};
use egui_dock::{DockArea, NodeIndex, TabViewer, Tree};
use egui_extras::{Column, TableBuilder, TableRow};
use json::JsonValue;
use log::debug;

use super::log_file_reader::LogFileReader;
use super::search_window::SearchWindow;

#[derive(PartialEq)]
enum LogViewTab {
    LogEntries,
    LogEntryContext,
    FilteredLogEntries(String, Vec<u64>),
}

pub struct LogView {
    tree: Tree<LogViewTab>,
    lines_view: LogLinesView,
    file_path: PathBuf,
}

struct LogLinesView {
    log_file_path: PathBuf,
    log_file_reader: LogFileReader,
    status_text: RichText,
    selected_line_num: Option<u64>,
    selected_log_entry: Option<JsonValue>,
    search_window: SearchWindow,
    tabs_to_open: Vec<(LogViewTab, NodeIndex)>,
    selected_line_changed: Option<u64>,
}

impl TabViewer for LogLinesView {
    type Tab = LogViewTab;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match tab {
            LogViewTab::LogEntries => self.ui_entries(ui),
            LogViewTab::LogEntryContext => {
                self.ui_entry_context(ui);
            }
            LogViewTab::FilteredLogEntries(_, ref entries) => self.ui_filtered_entries(ui, entries),
        }
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            LogViewTab::LogEntries => "Log".into(),
            LogViewTab::LogEntryContext => "Context".into(),
            LogViewTab::FilteredLogEntries(ref search_term, _) => {
                format!("Search '{}'", search_term).into()
            }
        }
    }

    fn add_popup(&mut self, ui: &mut Ui, node: NodeIndex) {
        ui.set_min_width(100.0);

        if ui.button("Log").clicked() {
            self.tabs_to_open.push((LogViewTab::LogEntries, node));
        }
        if ui.button("Context").clicked() {
            self.tabs_to_open.push((LogViewTab::LogEntryContext, node));
        }
    }
}

impl LogView {
    pub fn open(file_path: &Path) -> io::Result<Self> {
        let mut tree = Tree::new(vec![LogViewTab::LogEntries]);
        tree.split_below(NodeIndex::root(), 0.8, vec![LogViewTab::LogEntryContext]);

        Ok(LogView {
            tree,
            file_path: file_path.to_owned(),
            lines_view: LogLinesView::open(file_path)?,
        })
    }

    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    pub fn ui(self: &mut Self, ui: &mut Ui) {
        DockArea::new(&mut self.tree)
            .show_add_buttons(true)
            .show_add_popup(true)
            .scroll_area_in_tabs(false)
            .show_inside(ui, &mut self.lines_view);

        for (tab_type, destination_node) in self.lines_view.tabs_to_open.drain(..) {
            match self.tree.find_tab(&tab_type) {
                Some((existing_tab_node_index, _)) => {
                    self.tree.set_focused_node(existing_tab_node_index);
                }
                None => {
                    self.tree.set_focused_node(destination_node);
                    self.tree.push_to_focused_leaf(tab_type);
                }
            }
        }
    }

    pub fn open_search(&mut self) {
        self.lines_view.open_search()
    }
}

impl LogLinesView {
    pub fn open(filepath: &Path) -> io::Result<Self> {
        let mut log_view = LogLinesView {
            log_file_path: filepath.to_owned(),
            log_file_reader: LogFileReader::open(filepath)?,
            status_text: RichText::new(""),
            search_window: SearchWindow::new(),
            selected_line_num: None,
            selected_log_entry: None,
            tabs_to_open: vec![],
            selected_line_changed: None,
        };
        match log_view.log_file_reader.load() {
            Ok(line_count) => {
                log_view.status_text = RichText::new(format!("Loaded {} lines.", line_count));
            }
            Err(e) => {
                log_view.status_text =
                    RichText::new(format!("Failed to load lines from file: {}", e))
                        .color(Color32::RED);
            }
        }
        Ok(log_view)
    }

    fn ui_entries(self: &mut Self, ui: &mut Ui) {
        let line_count = self.log_file_reader.line_count();

        let mut table_builder = TableBuilder::new(ui)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::initial(150.0).resizable(true).clip(true))
            .column(Column::remainder().clip(true));

        if self.search_window.selection_changed() {
            let row_idx = self.selected_line_num.unwrap_or(0);
            debug!("Scrolling to row {}", row_idx);
            table_builder = table_builder.scroll_to_row(row_idx as usize, Some(Align::Center));
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
                body.rows(16.0, line_count as usize, |row_idx, mut row| {
                    self.show_logline(&mut row, row_idx);
                });
            });

        ui.allocate_space(ui.available_size());

        self.search_window.show(ui.ctx(), &self.log_file_path);

        if self.search_window.selection_changed() {
            self.select_row(self.search_window.selected_search_result_row(), None);
        }

        if self.search_window.wants_open_results() {
            self.tabs_to_open.push((
                LogViewTab::FilteredLogEntries(
                    self.search_window.search_term().to_owned(),
                    self.search_window.search_results().to_owned(),
                ),
                NodeIndex::root(),
            ));
        }
    }

    fn ui_filtered_entries(self: &mut Self, ui: &mut Ui, lines: &[u64]) {
        let line_count = lines.len();
        // let line_count = self.search_window.search_result_count();

        let mut table_builder = TableBuilder::new(ui)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(Column::auto())
            .column(Column::initial(150.0).resizable(true).clip(true))
            .column(Column::remainder().clip(true));

        if self.search_window.selection_changed() {
            let row_idx = self.selected_line_num.unwrap_or(0);
            debug!("Scrolling to row {}", row_idx);
            table_builder = table_builder.scroll_to_row(row_idx as usize, Some(Align::Center));
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
                body.rows(16.0, line_count, |row_idx, mut row| {
                    let line_number = lines[row_idx];
                    self.show_logline(&mut row, line_number as usize);
                });
            });

        ui.allocate_space(ui.available_size());
    }

    fn ui_entry_context(&self, ui: &mut Ui) -> Option<()> {
        let log_entry = self.selected_log_entry.as_ref()?;

        egui::ScrollArea::horizontal().show(ui, |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto().at_least(60.0))
                .column(Column::remainder())
                .body(|mut body| {
                    for entry in log_entry.entries() {
                        let key_str = entry.0;
                        let value_str = entry.1.as_str().unwrap_or_default().trim();
                        let line_count = value_str.chars().filter(|c| *c == '\n').count() + 1;
                        body.row((line_count as f32) * 16.0, |mut row| {
                            row.col(|ui| {
                                ui.label(RichText::new(key_str).color(Color32::WHITE).monospace());
                            });
                            row.col(|ui| {
                                ui.label(RichText::new(value_str).monospace());
                            });
                        });
                    }
                });
        });

        Some(())
    }

    pub fn open_search(&mut self) {
        self.search_window.open();
    }

    fn show_logline(self: &mut Self, row: &mut TableRow<'_, '_>, row_idx: usize) -> Option<()> {
        let line = self.log_file_reader.line(row_idx as u64)?;
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
                            self.selected_line_num == Some(row_idx as u64),
                            RichText::new(msg.trim())
                                .color(color_from_loglevel(level))
                                .monospace(),
                        )
                        .clicked()
                    {
                        self.select_row(Some(row_idx as u64), Some(log_entry));
                    }
                });
            }
            None => {
                row.col(|_| {});
                row.col(|_| {});
                row.col(|ui| {
                    ui.label(line.trim());
                });
            }
        }

        Some(())
    }

    fn select_row(&mut self, row_idx: Option<u64>, log_entry: Option<JsonValue>) {
        self.selected_line_num = row_idx;
        self.selected_log_entry = log_entry;
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

use std::{
    io,
    path::{Path, PathBuf},
};

use egui::{Color32, RichText, Ui};
use egui_dock::{DockArea, NodeIndex, TabViewer, Tree};
use egui_extras::{Column, TableBuilder};
use json::JsonValue;

use super::{log_entries_table::LogEntriesTable, search_window::SearchWindow};
use super::{log_file_reader::LogFileReader, search_window::SearchOptions};

struct FilteredLogEntriesTabState {
    search_term: String,
    matched_line_nums: Vec<u64>,
    search_options: SearchOptions,
}

impl PartialEq for FilteredLogEntriesTabState {
    fn eq(&self, other: &Self) -> bool {
        return self == other;
    }
}

#[derive(PartialEq)]
enum LogViewTab {
    LogEntries,
    LogEntryContext,
    FilteredLogEntries(FilteredLogEntriesTabState),
}

pub struct LogView {
    tree: Tree<LogViewTab>,
    log_view_context: LogViewContext,
    file_path: PathBuf,
}

struct LogViewContext {
    log_file_path: PathBuf,
    log_file_reader: LogFileReader,
    status_text: RichText,
    selected_line_num: Option<u64>,
    selected_log_entry: Option<JsonValue>,
    selected_line_changed: bool,
    search_window: SearchWindow,
    tabs_to_open: Vec<(LogViewTab, NodeIndex)>,
}

impl TabViewer for LogViewContext {
    type Tab = LogViewTab;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match tab {
            LogViewTab::LogEntries => self.ui_entries(ui),
            LogViewTab::LogEntryContext => {
                self.ui_entry_context(ui);
            }
            LogViewTab::FilteredLogEntries(ref mut state) => self.ui_filtered_entries(ui, state),
        }
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            LogViewTab::LogEntries => "Log".into(),
            LogViewTab::LogEntryContext => "Context".into(),
            LogViewTab::FilteredLogEntries(ref state) => {
                format!("Search '{}'", state.search_term).into()
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
            log_view_context: LogViewContext::open(file_path)?,
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
            .show_inside(ui, &mut self.log_view_context);

        self.log_view_context.selected_line_changed = false;
        self.log_view_context.ui_search(ui);

        for (tab_type, destination_node) in self.log_view_context.tabs_to_open.drain(..) {
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
        self.log_view_context.open_search()
    }
}

impl LogViewContext {
    pub fn open(filepath: &Path) -> io::Result<Self> {
        let mut log_view = LogViewContext {
            log_file_path: filepath.to_owned(),
            log_file_reader: LogFileReader::open(filepath)?,
            status_text: RichText::new(""),
            search_window: SearchWindow::new(),
            selected_line_num: None,
            selected_log_entry: None,
            selected_line_changed: false,
            tabs_to_open: vec![],
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

    fn ui_search(&mut self, ui: &mut Ui) {
        self.search_window.show(ui.ctx(), &self.log_file_path);

        if self.search_window.selection_changed() {
            self.select_row(self.search_window.selected_search_result_row(), None);
        }

        if self.search_window.wants_open_results() {
            let dest_index = NodeIndex::root().right();

            self.tabs_to_open.push((
                LogViewTab::FilteredLogEntries(FilteredLogEntriesTabState {
                    search_term: self.search_window.search_term().to_owned(),
                    matched_line_nums: self.search_window.search_results().to_owned(),
                    search_options: Default::default(),
                }),
                dest_index,
            ));
        }
    }

    fn ui_entries(self: &mut Self, ui: &mut Ui) {
        LogEntriesTable::new(&mut self.log_file_reader)
            .selected_line_num(self.selected_line_num)
            .ui(ui);

        // if self.selected_line_changed {
        //     let row_idx = self.selected_line_num.unwrap_or(0);
        //     debug!("Scrolling to row {}", row_idx);
        //     table_builder = table_builder.scroll_to_row(row_idx as usize, Some(Align::Center));
        // }
    }

    fn ui_filtered_entries(&mut self, ui: &mut Ui, state: &mut FilteredLogEntriesTabState) {
        LogEntriesTable::new(&mut self.log_file_reader)
            .filtered_lines(&state.matched_line_nums)
            .selected_line_num(self.selected_line_num)
            .ui(ui);
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

    fn select_row(&mut self, line_num: Option<u64>, log_entry: Option<JsonValue>) {
        self.selected_line_num = line_num;
        self.selected_log_entry = log_entry;
        self.selected_line_changed = true;
    }
}

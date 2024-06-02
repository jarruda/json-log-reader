use std::{
    io,
    path::{Path, PathBuf},
};

use egui::{Color32, Id, RichText, Ui};
use egui_dock::{DockArea, DockState, NodeIndex, SurfaceIndex, TabViewer};

use super::log_file_reader::LogFileReader;
use super::{
    filtered_log_entries_tab::FilteredLogEntriesTab,
    log_entries_tab::LogEntriesTab,
    log_entry_context_tab::LogEntryContextTab,
    log_file_reader::{LineNumber, LogEntry},
};

#[derive(Default)]
struct FilteredLogEntriesTabState {}

impl PartialEq for FilteredLogEntriesTabState {
    // Only equal if the reference is the same
    fn eq(&self, other: &Self) -> bool {
        return self == other;
    }
}

#[derive(Default)]
pub struct LogSelectionState {
    pub selected_line_num: Option<LineNumber>,
    pub selected_log_entry: Option<LogEntry>,
}

#[derive(Default)]
pub struct LogViewTabResponse {
    pub selected_line_num: Option<LineNumber>,
}

pub trait LogViewTabTrait {
    fn title(&self) -> egui::WidgetText;
    fn ui(
        &mut self,
        ui: &mut Ui,
        log_reader: &mut LogFileReader,
        selection_state: &LogSelectionState,
    ) -> LogViewTabResponse;
}

/// LogView owns a tree view that can be populated with tabs
/// to view and interact with a log file.
/// Tabs are one of the LogViewTab enum.
pub struct LogView {
    tree: DockState<Box<dyn LogViewTabTrait>>,
    log_view_context: LogViewContext,
    file_path: PathBuf,
}

struct LogViewContext {
    log_file_path: PathBuf,
    log_file_reader: LogFileReader,
    status_text: RichText,
    tabs_to_open: Vec<(Box<dyn LogViewTabTrait>, SurfaceIndex, NodeIndex)>,
    selection_state: LogSelectionState,
}

impl TabViewer for LogViewContext {
    type Tab = Box<dyn LogViewTabTrait>;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        let response = tab.ui(ui, &mut self.log_file_reader, &self.selection_state);
        if let Some(_) = response.selected_line_num {
            self.set_selection(response.selected_line_num);
        }
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title()
    }

    fn add_popup(&mut self, ui: &mut Ui, surface_index: SurfaceIndex, node: NodeIndex) {
        ui.set_min_width(100.0);

        if ui.button("Log").clicked() {
            self.tabs_to_open.push((LogEntriesTab::new(), surface_index, node));
        }
        if ui.button("Context").clicked() {
            self.tabs_to_open.push((LogEntryContextTab::new(), surface_index, node));
        }
    }
}

impl LogView {
    pub fn open(file_path: &Path) -> io::Result<Self> {
        let mut tree: DockState<Box<dyn LogViewTabTrait>> = DockState::new(vec![LogEntriesTab::new()]);
        tree.main_surface_mut().split_below(NodeIndex::root(), 0.8, vec![LogEntryContextTab::new()]);

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
            .id(Id::new(&self.file_path))
            .show_add_buttons(true)
            .show_add_popup(true)
            .show_inside(ui, &mut self.log_view_context);

        for (tab_type, destination_surface, destination_node) in self.log_view_context.tabs_to_open.drain(..) {
            self.tree.set_focused_node_and_surface((destination_surface, destination_node));
            self.tree.push_to_focused_leaf(tab_type);
        }
    }

    pub fn open_search(&mut self) {
        self.log_view_context.open_search()
    }
}

impl LogViewContext {
    pub fn open(filepath: &Path) -> io::Result<Self> {
        puffin::profile_function!();
        
        let mut log_view = LogViewContext {
            log_file_path: filepath.to_owned(),
            log_file_reader: LogFileReader::open(filepath)?,
            status_text: Default::default(),
            tabs_to_open: vec![],
            selection_state: Default::default(),
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

    fn set_selection(&mut self, selected_line_num: Option<LineNumber>) {
        self.selection_state.selected_line_num = selected_line_num;

        if let Some(selected_line_num) = selected_line_num {
            self.selection_state.selected_log_entry =
                self.log_file_reader.read_entry(selected_line_num);
        }
    }

    pub fn open_search(&mut self) {
        let dest_surface = SurfaceIndex::main();
        let dest_node = NodeIndex::root().right();

        self.tabs_to_open.push((
            FilteredLogEntriesTab::new(self.log_file_path.clone()),
            dest_surface,
            dest_node,
        ));
    }
}

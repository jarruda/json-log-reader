use std::{
    io,
    path::{Path, PathBuf},
};
use std::collections::HashMap;
use std::default::Default;

use egui::{Align2, Color32, Direction, Id, Ui, WidgetText};
use egui_dock::{DockArea, DockState, NodeIndex, SurfaceIndex, TabViewer};
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};

use super::{
    filtered_log_entries_tab::FilteredLogEntriesTab,
    log_entries_tab::LogEntriesTab,
    log_entry_context_tab::LogEntryContextTab,
    log_file_reader::{LineNumber, LogEntry},
};
use super::log_file_reader::LogFileReader;

#[derive(Default)]
struct FilteredLogEntriesTabState {}

impl PartialEq for FilteredLogEntriesTabState {
    // Only equal if the reference is the same
    fn eq(&self, other: &Self) -> bool {
        return self == other;
    }
}

pub struct LogSource {}

pub struct LogViewerState {
    pub selected_line_num: Option<LineNumber>,
    pub displayed_columns: Vec<String>,
    pub column_styles: HashMap<String, ColumnStyle>,
    pub toasts: Toasts,
}

impl LogViewerState {
    pub fn add_toast(&mut self, kind: ToastKind, text: WidgetText, duration_in_seconds: f64) {
        self.toasts.add(Toast {
            kind,
            text,
            options: ToastOptions::default().duration_in_seconds(duration_in_seconds),
        });
    }
}

#[derive(Clone)]
pub enum ColumnTextColor {
    Color(Color32),
    BySeverity,
}

#[derive(Clone)]
pub struct ColumnStyle {
    pub color: ColumnTextColor,
    pub auto_size: bool,
    pub trim: bool,
}

impl Default for ColumnStyle {
    fn default() -> Self {
        let default_col_style_ref: &ColumnStyle = Default::default();
        (*default_col_style_ref).clone()
    }
}

impl Default for LogViewerState {
    fn default() -> Self {
        Self {
            selected_line_num: None,
            displayed_columns: vec!["t".into(), "tag".into(), "message".into()],
            column_styles: HashMap::from([
                (
                    "t".to_string(),
                    ColumnStyle {
                        color: ColumnTextColor::Color(Color32::WHITE),
                        auto_size: true,
                        ..Default::default()
                    },
                ),
                (
                    "tag".to_string(),
                    ColumnStyle {
                        color: ColumnTextColor::Color(Color32::KHAKI),
                        auto_size: false,
                        ..Default::default()
                    },
                ),
                (
                    "message".to_string(),
                    ColumnStyle {
                        color: ColumnTextColor::BySeverity,
                        auto_size: false,
                        ..Default::default()
                    },
                ),
            ]),
            toasts: Toasts::new()
                .anchor(Align2::CENTER_BOTTOM, (0.0, -25.0))
                .direction(Direction::BottomUp),
        }
    }
}

impl Default for &'static ColumnStyle {
    fn default() -> Self {
        static SINGLETON: ColumnStyle = ColumnStyle {
            color: ColumnTextColor::Color(Color32::WHITE),
            auto_size: false,
            trim: true,
        };
        &SINGLETON
    }
}

pub trait LogViewTabTrait {
    fn title(&self) -> egui::WidgetText;
    fn ui(
        &mut self,
        ui: &mut Ui,
        log_reader: &mut LogFileReader,
        viewer_state: &mut LogViewerState,
    );
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
    tabs_to_open: Vec<(Box<dyn LogViewTabTrait>, SurfaceIndex, NodeIndex)>,
    viewer_state: LogViewerState,
}

impl TabViewer for LogViewContext {
    type Tab = Box<dyn LogViewTabTrait>;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        tab.ui(ui, &mut self.log_file_reader, &mut self.viewer_state);
    }

    fn add_popup(&mut self, ui: &mut Ui, surface_index: SurfaceIndex, node: NodeIndex) {
        ui.set_min_width(100.0);

        if ui.button("Log").clicked() {
            self.tabs_to_open
                .push((LogEntriesTab::new(), surface_index, node));
        }
        if ui.button("Context").clicked() {
            self.tabs_to_open
                .push((LogEntryContextTab::new(), surface_index, node));
        }
    }
}

impl LogView {
    pub fn open(file_path: &Path) -> io::Result<Self> {
        let mut tree: DockState<Box<dyn LogViewTabTrait>> =
            DockState::new(vec![LogEntriesTab::new()]);
        let new_nodes = tree.main_surface_mut().split_below(
            NodeIndex::root(),
            0.8,
            vec![LogEntryContextTab::new()],
        );
        tree.main_surface_mut().split_right(
            new_nodes[1],
            0.5,
            vec![FilteredLogEntriesTab::new(file_path.to_owned())]
        );

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

        for (tab_type, destination_surface, destination_node) in
            self.log_view_context.tabs_to_open.drain(..)
        {
            self.tree
                .set_focused_node_and_surface((destination_surface, destination_node));
            self.tree.push_to_focused_leaf(tab_type);
        }

        self.log_view_context.viewer_state.toasts.show(ui.ctx());
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
            tabs_to_open: vec![],
            viewer_state: Default::default(),
        };
        match log_view.log_file_reader.load() {
            Ok(line_count) => {
                log_view.viewer_state.add_toast(
                    ToastKind::Info,
                    format!("File load complete. Loaded {} lines.", line_count).into(),
                    10.0,
                );
            }
            Err(e) => {
                log_view.viewer_state.add_toast(
                    ToastKind::Error,
                    format!("Failed to load lines from file: {}", e).into(),
                    10.0,
                );
            }
        }
        Ok(log_view)
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

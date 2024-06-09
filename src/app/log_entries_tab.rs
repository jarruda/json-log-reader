use crate::app::log_file_reader::LineNumber;
use super::{
    log_entries_table::LogEntriesTable,
    log_file_reader::{LogFileReader},
    log_view::{LogViewerState, LogViewTabTrait},
};

pub struct LogEntriesTab {
    selected_line: Option<LineNumber>
}

impl LogEntriesTab {
    pub fn new() -> Box<Self> {
        Box::new(Self { selected_line: None })
    }
}

impl LogViewTabTrait for LogEntriesTab {
    fn title(&self) -> egui::WidgetText {
        "Log".into()
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        log_reader: &mut LogFileReader,
        viewer_state: &mut LogViewerState,        
    ) {
        
        let mut log_entries_table =
            LogEntriesTable::new().select_line(viewer_state.selected_line_num);

        if self.selected_line != viewer_state.selected_line_num {
            self.selected_line = viewer_state.selected_line_num;
            log_entries_table = log_entries_table.scroll_to_selected();
        }

        log_entries_table.ui(ui, log_reader, viewer_state);
    }
}

use super::{
    log_entries_table::LogEntriesTable,
    log_file_reader::{LineNumber, LogFileReader},
    log_view::{LogSelectionState, LogViewTabResponse, LogViewTabTrait},
};

pub struct LogEntriesTab {
    selected_line_num: Option<LineNumber>,
}

impl LogEntriesTab {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            selected_line_num: None,
        })
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
        selection_state: &LogSelectionState,
    ) -> LogViewTabResponse {
        let mut log_entries_table =
            LogEntriesTable::new().select_line(selection_state.selected_line_num);

        if selection_state.selected_line_num != self.selected_line_num {
            log_entries_table = log_entries_table.scroll_to_selected();
            self.selected_line_num = selection_state.selected_line_num;
        }

        let response = log_entries_table.ui(ui, log_reader);

        // Save a selection that came from this tab immediately to prevent scrolling
        if let Some(selected_line_num) = response.selected_line_num {
            self.selected_line_num = Some(selected_line_num);
        }

        LogViewTabResponse {
            selected_line_num: response.selected_line_num,
        }
    }
}

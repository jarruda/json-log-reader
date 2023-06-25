use super::{
    log_entries_table::LogEntriesTable,
    log_file_reader::{LineNumber, LogFileReader},
    log_view::{LogSelectionState, LogViewTabResponse, LogViewTabTrait},
    search_window::SearchOptions,
};

pub struct FilteredLogEntriesTab {
    selected_line_num: Option<LineNumber>,
    search_term: String,
    matched_line_nums: Vec<LineNumber>,
    search_options: SearchOptions,
}

impl FilteredLogEntriesTab {
    pub fn new(search_term: String, matched_line_nums: Vec<LineNumber>) -> Self {
        Self {
            selected_line_num: None,
            search_term,
            matched_line_nums,
            search_options: Default::default(),
        }
    }
}

impl LogViewTabTrait for FilteredLogEntriesTab {
    fn title(&self) -> egui::WidgetText {
        format!("Search: {}", self.search_term).into()
    }

    fn ui(
        &mut self,
        ui: &mut egui::Ui,
        log_reader: &mut LogFileReader,
        selection_state: &LogSelectionState,
    ) -> LogViewTabResponse {
        let mut log_entries_table = LogEntriesTable::new()
            .filtered_lines(&self.matched_line_nums)
            .select_line(selection_state.selected_line_num);

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

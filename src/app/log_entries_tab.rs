use super::{
    log_entries_table::LogEntriesTable,
    log_file_reader::LogFileReader,
    log_view::{LogViewTabTrait, LogViewerState},
};

pub struct LogEntriesTab {
    log_entries_table: LogEntriesTable,
}

impl LogEntriesTab {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            log_entries_table: LogEntriesTable::new(),
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
        viewer_state: &mut LogViewerState,
    ) {
        self.log_entries_table
            .ui(ui, log_reader, viewer_state, None, |_| {});
    }
}

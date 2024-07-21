use super::{
    log_entries_table::LogEntriesTable,
    log_view::{LogViewTabTrait, LogViewerState},
};
use crate::app::log_source::LogSource;
use egui::Ui;

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
        "📃 Log".into()
    }

    fn ui(
        &mut self,
        ui: &mut Ui,
        log_source: &mut dyn LogSource,
        viewer_state: &mut LogViewerState,
    ) {
        self.log_entries_table
            .ui(ui, log_source, viewer_state, |_| {});
    }
}

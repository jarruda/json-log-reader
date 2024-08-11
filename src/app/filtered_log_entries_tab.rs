use std::ops::DerefMut;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use egui::{Button, CursorIcon, Ui};

use crate::app::log_source::{LogSource, SearchOptions};

use super::{
    log_entries_table::LogEntriesTable,
    log_view::{LogViewTabTrait, LogViewerState},
};

pub struct FilteredLogEntriesTab {
    editable_search_term: String,
    search_term: String,
    search_options: SearchOptions,
    log_entries_table: LogEntriesTable,
    repeat_search: bool,
    last_search_time: Option<SystemTime>,
    filtered_source: Option<Arc<Mutex<dyn LogSource>>>,
}

impl FilteredLogEntriesTab {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            search_term: Default::default(),
            search_options: Default::default(),
            editable_search_term: Default::default(),
            log_entries_table: LogEntriesTable::new(),
            repeat_search: true,
            last_search_time: None,
            filtered_source: None,
        })
    }

    fn search(
        options: &SearchOptions,
        log_source: &mut dyn LogSource,
        search_text: &str,
    ) -> Arc<Mutex<dyn LogSource>> {
        log_source.filter_entries(search_text.to_string(), options.clone())
    }

    fn execute_search(&mut self, log_source: &mut dyn LogSource) {
        self.search_term = self.editable_search_term.clone();
        self.last_search_time = Some(SystemTime::now());

        if self.search_term.is_empty() {
            self.filtered_source = None;
            return;
        }

        self.filtered_source = Some(Self::search(
            &self.search_options,
            log_source,
            &self.search_term,
        ));
    }

    fn ui_search(&mut self, ui: &mut Ui, log_source: &mut dyn LogSource) {
        ui.horizontal(|ui| {
            ui.label("Search text:");

            if ui
                .text_edit_singleline(&mut self.editable_search_term)
                .lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                self.execute_search(log_source);
            }

            if ui
                .selectable_label(self.search_options.case_sensitive, "Case")
                .clicked()
            {
                self.search_options.case_sensitive = !self.search_options.case_sensitive;
            }
            if ui
                .selectable_label(self.search_options.whole_word, "Word")
                .clicked()
            {
                self.search_options.whole_word = !self.search_options.whole_word;
            }
            if ui
                .selectable_label(self.search_options.regex, "Regex")
                .clicked()
            {
                self.search_options.regex = !self.search_options.regex;
            }

            if ui.button("Search").clicked() {
                self.execute_search(log_source);
            }

            if !self.search_term.is_empty() {
                match self.filtered_source {
                    None => ui.label("No results"),
                    Some(ref s) => ui.label(format!("{} results", s.lock().unwrap().entry_count())),
                };
            }
        });

        ui.separator();
    }
}

impl LogViewTabTrait for FilteredLogEntriesTab {
    fn title(&self) -> egui::WidgetText {
        if self.search_term.is_empty() {
            "🔍 Search".into()
        } else {
            format!("🔍 Search: {}", self.search_term).into()
        }
    }

    fn ui(
        &mut self,
        ui: &mut Ui,
        log_source: &mut dyn LogSource,
        viewer_state: &mut LogViewerState,
    ) {
        let mut repeat_search = self.repeat_search;

        self.ui_search(ui, log_source);

        /* TODO
        if repeat_search && log_source.load_time_point().is_some() {
            let search_needed = match self.last_search_time {
                None => true,
                Some(last_search_time) => last_search_time < log_source.load_time_point().unwrap(),
            };
            if search_needed {
                self.execute_search(log_source);
            }
        }
        */

        if let Some(ref filtered_source) = self.filtered_source {
            let mut filtered_source = filtered_source.lock().unwrap();
            filtered_source.sync();
            
            self.log_entries_table.ui(
                ui,
                filtered_source.deref_mut(),
                viewer_state,
                |ui| {
                    if ui
                        .add(Button::new("⟳").selected(repeat_search))
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text("Repeat Search on Change")
                        .clicked()
                    {
                        repeat_search = !repeat_search;
                    };
                },
            );
        }

        self.repeat_search = repeat_search;
    }
}

use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};

use egui::Ui;
use grep::searcher::{sinks::Lossy, Searcher};
use grep_regex::RegexMatcherBuilder;
use log::error;

use super::{
    log_entries_table::LogEntriesTable,
    log_file_reader::{LineNumber, LogFileReader},
    log_view::{LogSelectionState, LogViewTabResponse, LogViewTabTrait},
};

#[derive(Debug)]
enum SearchError {
    IoError(io::Error),
    GrepError(grep_regex::Error),
}

impl From<io::Error> for SearchError {
    fn from(value: io::Error) -> Self {
        SearchError::IoError(value)
    }
}

impl From<grep_regex::Error> for SearchError {
    fn from(value: grep_regex::Error) -> Self {
        SearchError::GrepError(value)
    }
}

type SearchResult = Result<Vec<LineNumber>, SearchError>;

struct SearchOptions {
    case_sensitive: bool,
    whole_word: bool,
    regex: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            whole_word: false,
            regex: false,
        }
    }
}

pub struct FilteredLogEntriesTab {
    log_file_path: PathBuf,
    selected_line_num: Option<LineNumber>,
    editable_search_term: String,
    search_term: String,
    search_results: Vec<LineNumber>,
    search_options: SearchOptions,
}

impl FilteredLogEntriesTab {
    pub fn new(log_file_path: PathBuf) -> Box<Self> {
        Box::new(Self {
            log_file_path,
            selected_line_num: None,
            search_term: Default::default(),
            search_results: vec![],
            search_options: Default::default(),
            editable_search_term: Default::default(),
        })
    }

    fn search(options: &SearchOptions, file_path: &Path, search_text: &str) -> SearchResult {
        // If regex is turned off, escape the search text to literals.
        let escaped_search_text = if !options.regex {
            Some(regex::escape(search_text))
        } else {
            None
        };

        // Take a reference to escaped text (present if regex searching is off), or the search text if it's on.
        let pattern = if let Some(ref escaped_text) = escaped_search_text {
            escaped_text
        } else {
            search_text
        };

        // Build a grep matcher and searcher matching the options
        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(!options.case_sensitive)
            .word(options.whole_word)
            .build(&pattern)?;
        let mut searcher = Searcher::new();

        // Store line numbers of all matches
        let mut matches: Vec<LineNumber> = vec![];

        searcher.search_file(
            matcher,
            &File::open(file_path)?,
            Lossy(|line_num, _line| {
                let zero_based_line_num = line_num - 1;
                matches.push(zero_based_line_num as LineNumber);
                Ok(true)
            }),
        )?;

        Ok(matches)
    }

    fn execute_search(&mut self) {
        self.search_term = self.editable_search_term.clone();

        match Self::search(&self.search_options, &self.log_file_path, &self.search_term) {
            Ok(results) => {
                self.search_results = results;
            }
            Err(error) => error!("Failed to search: {:?}", error),
        }
    }

    fn ui_search(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Search text:");

            if ui
                .text_edit_singleline(&mut self.editable_search_term)
                .lost_focus()
                && ui.input(|i| i.key_pressed(egui::Key::Enter))
            {
                self.execute_search();
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
                self.execute_search();
            }

            if !self.search_term.is_empty() {
                match self.search_results.is_empty() {
                    true => ui.label("No results"),
                    false => ui.label(format!("{} results", self.search_results.len())),
                };
            }
        });

        ui.separator();
    }
}

impl LogViewTabTrait for FilteredLogEntriesTab {
    fn title(&self) -> egui::WidgetText {
        format!("Search: {}", self.search_term).into()
    }

    fn ui(
        &mut self,
        ui: &mut Ui,
        log_reader: &mut LogFileReader,
        selection_state: &LogSelectionState,
    ) -> LogViewTabResponse {
        self.ui_search(ui);

        let mut log_entries_table = LogEntriesTable::new()
            .filtered_lines(&self.search_results)
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

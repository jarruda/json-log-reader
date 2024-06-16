use std::time::SystemTime;
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};

use egui::{include_image, Button, CursorIcon, Ui};
use grep::searcher::{sinks::Lossy, Searcher};
use grep_regex::RegexMatcherBuilder;
use log::error;

use super::{
    log_entries_table::LogEntriesTable,
    log_file_reader::{LineNumber, LogFileReader},
    log_view::{LogViewTabTrait, LogViewerState},
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
    editable_search_term: String,
    search_term: String,
    search_results: Vec<LineNumber>,
    search_options: SearchOptions,
    log_entries_table: LogEntriesTable,
    repeat_search: bool,
    last_search_time: Option<SystemTime>,
}

impl FilteredLogEntriesTab {
    pub fn new(log_file_path: PathBuf) -> Box<Self> {
        Box::new(Self {
            log_file_path,
            search_term: Default::default(),
            search_results: vec![],
            search_options: Default::default(),
            editable_search_term: Default::default(),
            log_entries_table: LogEntriesTable::new(),
            repeat_search: true,
            last_search_time: None,
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
        self.last_search_time = Some(SystemTime::now());

        if self.search_term.is_empty() {
            self.search_results.clear();
            return;
        }

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
        if self.search_term.is_empty() {
            "Search".into()
        } else {
            format!("Search: {}", self.search_term).into()
        }
    }

    fn ui(
        &mut self,
        ui: &mut Ui,
        log_reader: &mut LogFileReader,
        viewer_state: &mut LogViewerState,
    ) {
        let mut repeat_search = self.repeat_search;

        self.ui_search(ui);

        if repeat_search && log_reader.load_time_point().is_some() {
            let search_needed = match self.last_search_time {
                None => true,
                Some(last_search_time) => last_search_time < log_reader.load_time_point().unwrap(),
            };
            if search_needed {
                self.execute_search();
            }
        }

        self.log_entries_table.ui(
            ui,
            log_reader,
            viewer_state,
            Some(&self.search_results),
            |ui| {
                if ui
                    .add(
                        Button::image(include_image!("../../assets/icons8-repeat-24-white.png"))
                            .selected(repeat_search),
                    )
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .on_hover_text("Repeat Search on Change")
                    .clicked()
                {
                    repeat_search = !repeat_search;
                };
            },
        );

        self.repeat_search = repeat_search;
    }
}

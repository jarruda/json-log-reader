use std::{fs::File, path::Path};

use grep::searcher::{sinks::Lossy, Searcher};
use grep_regex::RegexMatcherBuilder;

use log::error;

use super::log_file_reader::LineNumber;

pub struct SearchWindow {
    search_text: String,
    is_open: bool,
    search_results: Vec<LineNumber>,
    selected_search_result_row: Option<u64>,
    selection_changed: bool,
    search_options: SearchOptions,
    wants_open_results: bool,
}

pub struct SearchOptions {
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

impl SearchWindow {
    pub fn new() -> Self {
        Self {
            search_text: String::new(),
            is_open: false,
            selected_search_result_row: None,
            selection_changed: false,
            search_results: vec![],
            search_options: SearchOptions {
                case_sensitive: false,
                whole_word: false,
                regex: false,
            },
            wants_open_results: false,
        }
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context, path: &Path) -> &mut Self {
        self.selection_changed = false;
        self.wants_open_results = false;

        let mut trigger_search = false;
        let mut is_open = self.is_open;

        egui::Window::new("Search")
            .open(&mut is_open)
            .show(ctx, |ui| {
                ui.label("Search text:");

                ui.horizontal(|ui| {
                    if ui.text_edit_singleline(&mut self.search_text).lost_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        trigger_search = true;
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
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Search").clicked() {
                        trigger_search = true;
                    }
                    if self.search_result_count() > 0 {
                        if ui.button("Open Results").clicked() {
                            self.wants_open_results = true;
                            self.is_open = false;
                        }
                    }
                    if ui.button("Close").clicked() {
                        self.is_open = false;
                    }
                });

                if !self.search_results.is_empty() {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.add_enabled_ui(self.has_previous_result(), |ui| {
                            if ui.button("Previous Result").clicked() {
                                self.selected_search_result_row =
                                    Some(self.selected_search_result_row.unwrap() - 1);
                                self.selection_changed = true;
                            }
                        });
                        ui.add_enabled_ui(self.has_next_result(), |ui| {
                            if ui.button("Next Result").clicked() {
                                self.selected_search_result_row =
                                    Some(self.selected_search_result_row.unwrap() + 1);
                                self.selection_changed = true;
                            }
                        });
                        ui.label(format!(
                            "{} of {} results",
                            self.selected_search_result_row.unwrap_or(0) + 1,
                            self.search_results.len()
                        ));
                    });
                } else {
                    ui.label("No results.");
                }
            });

        self.is_open = self.is_open && is_open;

        if trigger_search {
            match Self::search(&self.search_options, path, &self.search_text) {
                Some(results) => {
                    self.search_results = results;
                    self.selected_search_result_row = if self.search_results.is_empty() {
                        None
                    } else {
                        Some(0)
                    };
                    self.selection_changed = true;
                }
                None => error!("Search failed: (need to propagate error)"),
            }
        }

        self
    }

    fn has_previous_result(&self) -> bool {
        match self.selected_search_result_row {
            Some(selected_row_idx) => self.search_results.len() > 0 && selected_row_idx > 0,
            None => false,
        }
    }

    fn has_next_result(&self) -> bool {
        match self.selected_search_result_row {
            Some(selected_row_idx) => selected_row_idx < (self.search_results.len() as u64 - 1),
            None => false,
        }
    }

    fn search(options: &SearchOptions, file_path: &Path, search_text: &str) -> Option<Vec<LineNumber>> {
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
            .build(&pattern)
            .ok()?;
        let mut searcher = Searcher::new();

        // Store line numbers of all matches
        let mut matches: Vec<LineNumber> = vec![];

        searcher
            .search_file(
                matcher,
                &File::open(file_path).ok()?,
                Lossy(|line_num, _line| {
                    let zero_based_line_num = line_num - 1;
                    matches.push(zero_based_line_num as LineNumber);
                    Ok(true)
                }),
            )
            .ok()?;

        Some(matches)
    }

    pub fn selection_changed(&self) -> bool {
        self.selection_changed
    }

    pub fn selected_search_result_line(&self) -> Option<LineNumber> {
        let result_line = *self
            .search_results
            .get(self.selected_search_result_row? as usize)?;
        Some(result_line)
    }

    pub fn search_result_count(&self) -> usize {
        self.search_results.len()
    }

    pub fn search_term(&self) -> &str {
        &self.search_text
    }

    pub fn search_results(&self) -> &[LineNumber] {
        &self.search_results
    }

    pub fn wants_open_results(&self) -> bool {
        self.wants_open_results
    }
}

impl Default for SearchWindow {
    fn default() -> Self {
        Self::new()
    }
}

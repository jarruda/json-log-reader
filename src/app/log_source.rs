use json::JsonValue;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub object: JsonValue,
}

pub enum ReadEntryError {
    ReadFailure,
    ParseFailure(String),
}


#[derive(Clone)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
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

pub trait LogSource {
    fn name(&self) -> String;
    
    fn uri(&self) -> String;

    fn entry_count(&mut self) -> usize;

    fn use_entry(
        &mut self,
        entry_index: usize,
        entry_user: &mut dyn FnMut(Result<&LogEntry, ReadEntryError>),
    );

    fn filter_entries(
        &self,
        search_query: String,
        search_options: SearchOptions,
    ) -> Arc<Mutex<dyn LogSource>>;
}

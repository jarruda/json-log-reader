use chrono::{DateTime, FixedOffset};
use json::JsonValue;
use std::cmp::Ordering;
use std::hash::{DefaultHasher, Hasher};
use std::io;
use std::io::Write;
use std::sync::{Arc, Mutex};

// Custom writer that wraps a hasher
struct HasherWriter {
    hasher: DefaultHasher
}

impl Write for HasherWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.hasher.write(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl HasherWriter {
    pub fn new() -> Self {
        HasherWriter {
            hasher: DefaultHasher::new()
        }
    }

    pub fn finish(&mut self) -> u64 {
        self.hasher.finish()
    }
}

#[derive(Clone, Default, Debug)]
pub struct LogEntryId {
    pub sort_key: Option<DateTime<FixedOffset>>,
    pub hash: u64,
}

impl From<&LogEntry> for LogEntryId {
    fn from(value: &LogEntry) -> Self {
        let t = &value.object["t"];

        let mut hasher = HasherWriter::new();
        value.object.write(&mut hasher).expect("Write to hasher failed.");

        LogEntryId {
            sort_key: match t.as_str() {
                None => None,
                Some(t) => DateTime::parse_from_rfc3339(t).ok(),
            },
            hash: hasher.finish(),
        }
    }
}

impl PartialEq<Self> for LogEntryId {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for LogEntryId {}

impl PartialOrd<Self> for LogEntryId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.sort_key.partial_cmp(&other.sort_key)
    }
}

impl Ord for LogEntryId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.sort_key.cmp(&other.sort_key)
    }
}

#[derive(Clone)]
pub struct LogEntry {
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

    fn find_entry_index(&mut self, entry_id: &LogEntryId) -> Option<usize>;

    fn sync(&mut self);
}

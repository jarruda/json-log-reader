use io::Error;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, Weak};
use std::time::SystemTime;
use std::{
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use crossbeam_channel::Receiver;
use grep::searcher::sinks::Lossy;
use grep::searcher::{Searcher, Sink, SinkMatch};
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::app::log_source::{LogEntry, LogEntryId, LogSource, ReadEntryError, SearchOptions};

fn to_io_error(err: notify::Error) -> Error {
    Error::new(io::ErrorKind::Other, err)
}

pub type LineNumber = usize;

type FileOffset = u64;

struct AbsolutePositionSink<F>(pub F)
where
    F: FnMut(u64) -> Result<bool, Error>;

impl<F> Sink for AbsolutePositionSink<F>
where
    F: FnMut(u64) -> Result<bool, Error>,
{
    type Error = Error;

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        self.0(mat.absolute_byte_offset())
    }
}

pub struct FileLogSource {
    arc_weak: Weak<Mutex<Self>>,
    file_path: PathBuf,
    buf_reader: BufReader<File>,
    line_map: Vec<FileOffset>,
    file_size: FileOffset,
    load_time_point: Option<SystemTime>,
    read_line_buffer: Vec<u8>,
    _watcher: Box<dyn Watcher>,
    watcher_recv: Receiver<notify::Result<Event>>,
}

impl FileLogSource {
    pub fn open(path: &Path) -> io::Result<Arc<Mutex<FileLogSource>>> {
        // sync_channel of 0 makes it a "rendezvous" channel where the watching thread hands off to receiver
        let (tx, rx) = crossbeam_channel::bounded(0);

        let mut watcher = RecommendedWatcher::new(tx, Config::default()).map_err(to_io_error)?;
        watcher
            .watch(path, RecursiveMode::NonRecursive)
            .map_err(to_io_error)?;

        let file = File::open(path)?;
        let log_source = Arc::new_cyclic(|weak| {
            Mutex::new(FileLogSource {
                arc_weak: weak.clone(),
                file_path: path.to_path_buf(),
                buf_reader: BufReader::new(file),
                line_map: Vec::new(),
                file_size: 0,
                load_time_point: None,
                read_line_buffer: vec![],
                _watcher: Box::new(watcher),
                watcher_recv: rx,
            })
        });
        log_source.lock().unwrap().load()?;
        Ok(log_source)
    }

    /// Reads the entire file to count the number of lines.
    /// Caches a map of line numbers to file positions.
    /// Returns the number of lines in the file if successful, error otherwise.
    pub fn load(&mut self) -> io::Result<usize> {
        puffin::profile_function!();

        self.buf_reader.rewind()?;
        self.line_map.clear();

        // Build a grep matcher and searcher matching the options
        let newline = "$";
        let matcher = RegexMatcher::new_line_matcher(&newline).unwrap();
        let mut searcher = Searcher::new();

        // Load all newline file positions into line_map
        searcher.search_reader(
            matcher,
            self.buf_reader.get_ref(),
            AbsolutePositionSink(|file_offset| -> Result<bool, Error> {
                self.line_map.push(file_offset as FileOffset);
                Ok(true)
            }),
        )?;

        self.buf_reader.seek(SeekFrom::End(0))?;
        self.file_size = self.buf_reader.stream_position()?;
        self.line_map.push(self.file_size);

        self.load_time_point = Some(SystemTime::now());
        Ok(self.entry_count())
    }

    pub fn has_changed(&mut self) -> bool {
        self.watcher_recv.try_recv().is_ok()
    }

    pub fn load_time_point(&self) -> Option<SystemTime> {
        self.load_time_point
    }

    /// Reads a line from the file parsed as a UTF8 string
    pub fn read_line(&mut self, line_num: LineNumber) -> Option<String> {
        let (file_start_offset, file_end_offset) = self.line_file_offsets(line_num);
        self.buf_reader
            .seek(SeekFrom::Start(file_start_offset))
            .ok()?;

        self.read_line_buffer.clear();
        self.read_line_buffer
            .resize((file_end_offset - file_start_offset) as usize, 0);
        self.buf_reader
            .read_exact(&mut self.read_line_buffer)
            .ok()?;

        Some(String::from_utf8_lossy(&self.read_line_buffer).to_string())
    }

    /// Reads a line from the file parsed as a UTF8 string
    pub fn read_line_from_offset(
        reader: &mut BufReader<File>,
        file_offset: &FileOffset,
        line_buffer: &mut Vec<u8>,
    ) -> Option<String> {
        reader.seek(SeekFrom::Start(*file_offset)).ok()?;

        line_buffer.clear();
        reader.read_until(b'\n', line_buffer).ok()?;

        Some(String::from_utf8_lossy(&line_buffer).to_string())
    }

    /// Parses a JSON object from the given string slice
    /// Format is <json-object>\n
    /// e.g. { "t": "2023-06-25T00:49:20Z", "message": "hello, world" }
    pub fn parse_logline(line: &str) -> Option<LogEntry> {
        let log_entry = json::parse(line).ok()?;

        if log_entry.is_object() {
            Some(LogEntry { object: log_entry })
        } else {
            None
        }
    }

    /// Returns the file offset of the beginning of the given line number
    fn line_start_offset(&self, line_num: LineNumber) -> FileOffset {
        match self.line_map.get(line_num) {
            Some(offset) => *offset,
            None => self.file_size,
        }
    }

    /// Returns the file offset of the end of the given line number
    fn line_end_offset(&self, line_num: LineNumber) -> FileOffset {
        match self.line_map.get(line_num + 1) {
            Some(offset) => *offset,
            None => self.file_size,
        }
    }

    /// Returns the file offsets for the start and end of the given line
    /// If line_num is invalid (> line_count()), returns the end of the file for both offsets.
    fn line_file_offsets(&self, line_num: LineNumber) -> (FileOffset, FileOffset) {
        (
            self.line_start_offset(line_num),
            self.line_end_offset(line_num),
        )
    }
}

impl LogSource for FileLogSource {
    fn name(&self) -> String {
        match self.file_path.file_name() {
            None => "No file name.".into(),
            Some(s) => s.to_string_lossy().into(),
        }
    }

    fn uri(&self) -> String {
        self.file_path.to_string_lossy().into()
    }

    fn entry_count(&mut self) -> usize {
        self.line_map.len() - 1
    }

    fn use_entry(
        &mut self,
        entry_index: usize,
        entry_user: &mut dyn FnMut(Result<&LogEntry, ReadEntryError>),
    ) {
        let line_content = self.read_line(entry_index);
        match line_content {
            None => {
                entry_user(Err(ReadEntryError::ReadFailure));
            }
            Some(line_content) => {
                let entry = Self::parse_logline(&line_content);
                match entry {
                    None => entry_user(Err(ReadEntryError::ParseFailure(line_content))),
                    Some(ref entry) => entry_user(Ok(entry)),
                }
            }
        }
    }

    fn filter_entries(
        &self,
        search_query: String,
        search_options: SearchOptions,
    ) -> Arc<Mutex<dyn LogSource>> {
        FilteredFileLogSource::new(self.file_path.clone(), search_query, search_options)
    }

    fn find_entry_index(&mut self, entry_id: &LogEntryId) -> Option<usize> {
        let search_result = self.line_map.binary_search_by_key(entry_id, |file_offset| {
            let line = Self::read_line_from_offset(
                &mut self.buf_reader,
                file_offset,
                &mut self.read_line_buffer,
            );
            let entry = match line {
                None => None,
                Some(ref line) => Self::parse_logline(line),
            };
            match entry {
                None => Default::default(),
                Some(ref entry) => entry.into(),
            }
        });

        search_result.ok()
    }
}

///

struct FilteredFileLogSource {
    arc_weak: Weak<Mutex<FilteredFileLogSource>>,
    file_log_source: Arc<Mutex<FileLogSource>>,
    file_path: PathBuf,
    search_query: String,
    search_options: SearchOptions,
    search_results: Vec<usize>,
}

impl FilteredFileLogSource {
    fn new(
        file_path: PathBuf,
        search_query: String,
        search_options: SearchOptions,
    ) -> Arc<Mutex<FilteredFileLogSource>> {
        let log_source = Arc::new_cyclic(|weak| {
            Mutex::new(FilteredFileLogSource {
                arc_weak: weak.clone(),
                file_log_source: FileLogSource::open(&file_path).unwrap(),
                file_path,
                search_query,
                search_options,
                search_results: vec![],
            })
        });

        log_source.lock().unwrap().load_results();

        log_source
    }

    fn load_results(&mut self) {
        // If regex is turned off, escape the search text to literals.
        let escaped_search_text = if !self.search_options.regex {
            Some(regex::escape(&self.search_query))
        } else {
            None
        };

        // Take a reference to escaped text (present if regex searching is off), or the search text if it's on.
        let pattern = if let Some(ref escaped_text) = escaped_search_text {
            escaped_text
        } else {
            &self.search_query
        };

        // Build a grep matcher and searcher matching the options
        let matcher = RegexMatcherBuilder::new()
            .case_insensitive(!self.search_options.case_sensitive)
            .word(self.search_options.whole_word)
            .build(&pattern)
            .unwrap();
        let mut searcher = Searcher::new();

        // Store line numbers of all matches
        let mut matches: Vec<usize> = vec![];

        searcher
            .search_file(
                matcher,
                &File::open(&self.file_path).unwrap(),
                Lossy(|line_num, _line| {
                    let zero_based_line_num = line_num - 1;
                    matches.push(zero_based_line_num as usize);
                    Ok(true)
                }),
            )
            .unwrap();

        self.search_results = matches;
    }
}

impl LogSource for FilteredFileLogSource {
    fn name(&self) -> String {
        match self.file_path.file_name() {
            None => "No file name.".into(),
            Some(s) => s.to_string_lossy().into(),
        }
    }

    fn uri(&self) -> String {
        self.file_path.to_string_lossy().into()
    }

    fn entry_count(&mut self) -> usize {
        self.search_results.len()
    }

    fn use_entry(
        &mut self,
        entry_index: usize,
        entry_user: &mut dyn FnMut(Result<&LogEntry, ReadEntryError>),
    ) {
        let source_index = self.search_results.get(entry_index);
        match source_index {
            Some(source_index) => {
                self.file_log_source
                    .lock()
                    .unwrap()
                    .use_entry(*source_index, entry_user);
            }
            None => {
                entry_user(Err(ReadEntryError::ReadFailure));
            }
        }
    }

    fn filter_entries(
        &self,
        search_query: String,
        search_options: SearchOptions,
    ) -> Arc<Mutex<dyn LogSource>> {
        self.file_log_source
            .lock()
            .unwrap()
            .filter_entries(search_query, search_options)
    }

    fn find_entry_index(&mut self, entry_id: &LogEntryId) -> Option<usize> {
        let mut source = self.file_log_source.lock().unwrap();

        let search_result = self
            .search_results
            .binary_search_by_key(entry_id, |line_num| {
                let line = source.read_line(*line_num);
                let entry = match line {
                    None => None,
                    Some(ref line) => FileLogSource::parse_logline(line),
                };
                match entry {
                    None => Default::default(),
                    Some(ref entry) => entry.into(),
                }
            });

        search_result.ok()
    }
}

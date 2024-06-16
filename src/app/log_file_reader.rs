use io::Error;
use std::{
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom},
    path::Path,
};
use std::time::SystemTime;
use crossbeam_channel::Receiver;

use grep::searcher::{Searcher, Sink, SinkMatch};
use grep_regex::RegexMatcher;
use json::JsonValue;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

fn to_io_error(err: notify::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

#[derive(Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub object: JsonValue,
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
        (self.0)(mat.absolute_byte_offset())
    }
}

pub struct LogFileReader {
    buf_reader: BufReader<File>,
    line_map: Vec<FileOffset>,
    file_size: FileOffset,
    load_time_point: Option<SystemTime>,
    _watcher: Box<dyn Watcher>,
    watcher_recv: Receiver<notify::Result<Event>>,
}

impl LogFileReader {
    pub fn open(path: &Path) -> io::Result<LogFileReader> {
        // sync_channel of 0 makes it a "rendezvous" channel where the watching thread hands off to receiver
        let (tx, rx) = crossbeam_channel::bounded(0);

        let mut watcher = RecommendedWatcher::new(tx, Config::default()).map_err(to_io_error)?;
        watcher
            .watch(path, RecursiveMode::NonRecursive)
            .map_err(to_io_error)?;

        let file = File::open(path)?;
        Ok(LogFileReader {
            buf_reader: BufReader::new(file),
            line_map: Vec::new(),
            file_size: 0,
            load_time_point: None,
            _watcher: Box::new(watcher),
            watcher_recv: rx,
        })
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
        Ok(self.line_count())
    }

    pub fn has_changed(&mut self) -> bool {
        self.watcher_recv.try_recv().is_ok()
    }

    pub fn load_time_point(&self) -> Option<SystemTime> {
        self.load_time_point
    }

    /// Returns the total number of lines counted in the file
    /// Only valid after a successful load.
    pub fn line_count(&self) -> usize {
        self.line_map.len() - 1
    }

    /// Reads a line from the file parsed as a UTF8 string
    pub fn read_line(&mut self, line_num: LineNumber) -> Option<String> {
        let (file_start_offset, file_end_offset) = self.line_file_offsets(line_num);
        self.buf_reader
            .seek(SeekFrom::Start(file_start_offset))
            .ok()?;

        // non-ideal, pre-populates vector with 0s
        let mut line_bytes: Vec<u8> = vec![0u8; (file_end_offset - file_start_offset) as usize];
        self.buf_reader.read_exact(&mut line_bytes).ok()?;

        Some(String::from_utf8_lossy(&line_bytes).to_string())
    }

    /// Reads a log entry from the give line from the file.
    /// Equivalent to using `read_line` and `parse_logline` consecutively.
    pub fn read_entry(&mut self, line_num: usize) -> Option<LogEntry> {
        let line_content = self.read_line(line_num)?;
        Self::parse_logline(&line_content)
    }

    /// Parses a JSON object from the given string slice
    /// Format is <json-object>\n
    /// e.g. { "t": "2023-06-25T00:49:20Z", "message": "hello, world" }
    pub fn parse_logline(line: &str) -> Option<LogEntry> {
        let log_entry = json::parse(line).ok()?;

        if log_entry.is_object() {
            Some(LogEntry {
                timestamp: log_entry["t"].as_str()?.to_owned(),
                object: log_entry,
            })
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

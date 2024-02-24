use std::{
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom},
    path::Path,
};

use json::JsonValue;

pub struct LogEntry {
    pub timestamp: String,
    pub object: JsonValue,
}

pub type LineNumber = usize;

type FileOffset = u64;

pub struct LogFileReader {
    buf_reader: BufReader<File>,
    line_map: Vec<FileOffset>,
    file_size: FileOffset,
}

impl LogFileReader {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<LogFileReader> {
        let file = File::open(path)?;
        Ok(LogFileReader::new(file))
    }

    pub fn new(file: File) -> LogFileReader {
        LogFileReader {
            buf_reader: BufReader::new(file),
            line_map: Vec::new(),
            file_size: 0,
        }
    }

    /// Reads the entire file to count the number of lines.
    /// Caches a map of line numbers to file positions.
    /// Returns the number of lines in the file if successful, error otherwise.
    pub fn load(&mut self) -> io::Result<usize> {
        self.buf_reader.rewind()?;
        self.line_map.clear();

        for i in self
            .buf_reader
            .by_ref()
            .bytes()
            .enumerate()
            .filter(|i| i.1.is_ok() && *i.1.as_ref().unwrap() as char == '\n')
        {
            self.line_map.push(i.0 as FileOffset);
        }

        self.buf_reader.seek(SeekFrom::End(0))?;
        self.file_size = self.buf_reader.stream_position()?;
        self.line_map.push(self.file_size);

        Ok(self.line_count())
    }

    /// Returns the total number of lines counted in the file
    /// Only valid after a successful load.
    pub fn line_count(&self) -> usize {
        self.line_map.len()
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

    /// Parses a timestamp and JSON object from the given string slice
    /// Format is <json-object>\n
    /// Time is in field "t"
    /// e.g. 2023-06-25T00:49:20Z { "message": "hello, world" }
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
        if line_num == 0 {
            0
        } else {
            match self.line_map.get(line_num - 1) {
                Some(offset) => offset + 1,
                None => self.file_size,
            }
        }
    }

    /// Returns the file offset of the end of the given line number
    fn line_end_offset(&self, line_num: LineNumber) -> FileOffset {
        match self.line_map.get(line_num) {
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

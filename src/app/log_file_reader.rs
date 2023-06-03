use std::{
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom},
    path::Path,
};

pub struct LogFileReader {
    buf_reader: BufReader<File>,
    line_map: Vec<u64>,
    file_size: u64,
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

    pub fn load(&mut self) -> io::Result<u64> {
        self.buf_reader.rewind()?;
        self.line_map.clear();

        for i in self
            .buf_reader
            .by_ref()
            .bytes()
            .enumerate()
            .filter(|i| i.1.is_ok() && *i.1.as_ref().unwrap() as char == '\n')
        {
            self.line_map.push(i.0 as u64);
        }

        
        self.buf_reader.seek(SeekFrom::End(0))?;
        self.file_size = self.buf_reader.stream_position()?;
        self.line_map.push(self.file_size);
        
        Ok(self.line_count())
    }

    pub fn line_count(&self) -> u64 {
        self.line_map.len() as u64
    }

    pub fn line(&mut self, line_num: u64) -> Option<String> {
        let file_start_offset = self.line_start_offset(line_num);
        let file_end_offset = self.line_end_offset(line_num);
        self.buf_reader
            .seek(SeekFrom::Start(file_start_offset))
            .ok()?;

        // non-ideal, pre-populates vector with 0s
        let mut line_bytes: Vec<u8> = vec![0u8; (file_end_offset - file_start_offset) as usize];
        self.buf_reader.read_exact(&mut line_bytes).ok()?;

        Some(String::from_utf8_lossy(&line_bytes).to_string())
    }

    fn line_start_offset(&self, line_num: u64) -> u64 {
        if line_num == 0 {
            0
        } else {
            self.line_map[(line_num - 1) as usize] + 1
        }
    }

    fn line_end_offset(&self, line_num: u64) -> u64 {
        if line_num == 0 {
            self.line_map[0]
        } else {
            match self.line_map.get(line_num as usize) {
                Some(idx) => *idx,
                None => self.file_size,
            }
        }
    }
}

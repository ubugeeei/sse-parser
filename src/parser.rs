use compact_str::CompactString;
use memchr::{memchr, memchr2};
use smallvec::SmallVec;

use crate::{Event, ParseError};

const UTF8_BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

#[derive(Debug, Default)]
pub struct Parser {
    bom_buffer: SmallVec<[u8; 3]>,
    bom_resolved: bool,
    skip_lf: bool,
    line_buffer: SmallVec<[u8; 256]>,
    data_buffer: SmallVec<[u8; 256]>,
    event_type: CompactString,
    last_event_id: Option<CompactString>,
    retry: Option<u64>,
}

impl Parser {
    #[must_use]
    pub fn new() -> Self {
        Self {
            bom_buffer: SmallVec::new(),
            bom_resolved: false,
            skip_lf: false,
            line_buffer: SmallVec::new(),
            data_buffer: SmallVec::new(),
            event_type: CompactString::default(),
            last_event_id: None,
            retry: None,
        }
    }

    pub fn feed(&mut self, chunk: &[u8]) -> Result<Vec<Event>, ParseError> {
        let mut events = Vec::new();
        self.feed_into(chunk, &mut events)?;
        Ok(events)
    }

    pub fn feed_into(&mut self, chunk: &[u8], out: &mut Vec<Event>) -> Result<(), ParseError> {
        let mut offset = 0;

        while !self.bom_resolved && offset < chunk.len() {
            self.feed_byte(chunk[offset], out)?;
            offset += 1;
        }

        if offset < chunk.len() {
            self.feed_stream_chunk(&chunk[offset..], out)?;
        }

        Ok(())
    }

    pub fn finish(&mut self) -> Result<Vec<Event>, ParseError> {
        let mut events = Vec::new();
        self.finish_into(&mut events)?;
        Ok(events)
    }

    pub fn finish_into(&mut self, out: &mut Vec<Event>) -> Result<(), ParseError> {
        self.flush_bom(out)?;

        if !self.line_buffer.is_empty() {
            self.process_pending_line(out)?;
        }

        self.skip_lf = false;
        self.dispatch(out)
    }

    pub fn reset(&mut self) {
        self.bom_buffer.clear();
        self.bom_resolved = false;
        self.skip_lf = false;
        self.line_buffer.clear();
        self.data_buffer.clear();
        self.event_type.clear();
        self.last_event_id = None;
        self.retry = None;
    }

    #[must_use]
    pub fn last_event_id(&self) -> Option<&str> {
        self.last_event_id.as_deref()
    }

    #[must_use]
    pub fn retry(&self) -> Option<u64> {
        self.retry
    }

    fn feed_byte(&mut self, byte: u8, out: &mut Vec<Event>) -> Result<(), ParseError> {
        if !self.bom_resolved {
            return self.handle_bom(byte, out);
        }

        self.feed_stream_byte(byte, out)
    }

    fn handle_bom(&mut self, byte: u8, out: &mut Vec<Event>) -> Result<(), ParseError> {
        self.bom_buffer.push(byte);

        match self.bom_buffer.as_slice() {
            [0xEF] | [0xEF, 0xBB] => Ok(()),
            buffer if buffer == UTF8_BOM => {
                self.bom_buffer.clear();
                self.bom_resolved = true;
                Ok(())
            }
            buffer if UTF8_BOM.starts_with(buffer) => Ok(()),
            _ => self.flush_bom(out),
        }
    }

    fn flush_bom(&mut self, out: &mut Vec<Event>) -> Result<(), ParseError> {
        if self.bom_resolved {
            return Ok(());
        }

        self.bom_resolved = true;

        if self.bom_buffer.is_empty() {
            return Ok(());
        }

        let mut buffered = SmallVec::new();
        core::mem::swap(&mut buffered, &mut self.bom_buffer);

        for byte in buffered {
            self.feed_stream_byte(byte, out)?;
        }

        Ok(())
    }

    fn feed_stream_byte(&mut self, byte: u8, out: &mut Vec<Event>) -> Result<(), ParseError> {
        if self.skip_lf {
            self.skip_lf = false;

            if byte == b'\n' {
                return Ok(());
            }
        }

        match byte {
            b'\n' => self.process_pending_line(out),
            b'\r' => {
                self.process_pending_line(out)?;
                self.skip_lf = true;
                Ok(())
            }
            _ => {
                self.line_buffer.push(byte);
                Ok(())
            }
        }
    }

    fn feed_stream_chunk(
        &mut self,
        mut chunk: &[u8],
        out: &mut Vec<Event>,
    ) -> Result<(), ParseError> {
        if self.skip_lf {
            self.skip_lf = false;

            if let Some(b'\n') = chunk.first().copied() {
                chunk = &chunk[1..];
            }
        }

        while let Some(line_end) = memchr2(b'\n', b'\r', chunk) {
            if self.line_buffer.is_empty() {
                self.process_line(&chunk[..line_end], out)?;
            } else {
                self.line_buffer.extend_from_slice(&chunk[..line_end]);
                self.process_pending_line(out)?;
            }

            chunk = self.consume_line_ending(chunk, line_end);
        }

        if !chunk.is_empty() {
            self.line_buffer.extend_from_slice(chunk);
        }

        Ok(())
    }

    fn process_pending_line(&mut self, out: &mut Vec<Event>) -> Result<(), ParseError> {
        let mut line = core::mem::take(&mut self.line_buffer);
        let result = self.process_line(&line, out);
        line.clear();
        self.line_buffer = line;
        result
    }

    fn process_line(&mut self, line: &[u8], out: &mut Vec<Event>) -> Result<(), ParseError> {
        if line.is_empty() {
            return self.dispatch(out);
        }

        if line[0] == b':' {
            return Ok(());
        }

        let colon_index = memchr(b':', line);
        let (field, value) = match colon_index {
            Some(index) => {
                let mut value = &line[index + 1..];
                if value.first() == Some(&b' ') {
                    value = &value[1..];
                }
                (&line[..index], value)
            }
            None => (line, b"".as_slice()),
        };

        match field {
            b"data" => {
                self.data_buffer.extend_from_slice(value);
                self.data_buffer.push(b'\n');
            }
            b"event" => {
                self.event_type = CompactString::from(decode_utf8(value, "event")?);
            }
            b"id" => {
                if memchr(0, value).is_none() {
                    self.last_event_id = Some(CompactString::from(decode_utf8(value, "id")?));
                }
            }
            b"retry" => {
                if let Some(retry) = parse_retry(value) {
                    self.retry = Some(retry);
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn dispatch(&mut self, out: &mut Vec<Event>) -> Result<(), ParseError> {
        if self.data_buffer.is_empty() {
            self.event_type.clear();
            return Ok(());
        }

        let data_slice = match self.data_buffer.last() {
            Some(b'\n') => &self.data_buffer[..self.data_buffer.len() - 1],
            _ => self.data_buffer.as_slice(),
        };

        let data = decode_utf8(data_slice, "data")?.to_owned();
        let event = if self.event_type.is_empty() {
            CompactString::from("message")
        } else {
            core::mem::take(&mut self.event_type)
        };

        out.push(Event {
            event,
            data,
            id: self.last_event_id.clone(),
        });

        self.data_buffer.clear();

        Ok(())
    }

    fn consume_line_ending<'a>(&mut self, chunk: &'a [u8], line_end: usize) -> &'a [u8] {
        let next = line_end + 1;

        if chunk[line_end] == b'\r' {
            if let Some(b'\n') = chunk.get(next).copied() {
                return &chunk[next + 1..];
            }

            if next == chunk.len() {
                self.skip_lf = true;
            }
        }

        &chunk[next..]
    }
}

fn decode_utf8<'a>(bytes: &'a [u8], field: &str) -> Result<&'a str, ParseError> {
    core::str::from_utf8(bytes).map_err(|_| ParseError::invalid_utf8(field))
}

fn parse_retry(bytes: &[u8]) -> Option<u64> {
    if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
        return None;
    }

    let mut value = 0_u64;

    for &byte in bytes {
        value = value.checked_mul(10)?;
        value = value.checked_add(u64::from(byte - b'0'))?;
    }

    Some(value)
}

#[cfg(test)]
mod tests {
    use compact_str::CompactString;

    use super::Parser;
    use crate::Event;

    #[test]
    fn parses_basic_events() {
        let mut parser = Parser::new();
        let events = parser.feed(b"event: ping\ndata: hello\n\n").unwrap();

        assert_eq!(
            events,
            vec![Event {
                event: CompactString::from("ping"),
                data: String::from("hello"),
                id: None,
            }]
        );
    }

    #[test]
    fn supports_split_chunks_and_utf8() {
        let mut parser = Parser::new();

        assert!(parser.feed("data: こ".as_bytes()).unwrap().is_empty());

        let events = parser.feed("んにちは\n\n".as_bytes()).unwrap();
        assert_eq!(events[0].data, "こんにちは");
    }

    #[test]
    fn handles_crlf_and_finish() {
        let mut parser = Parser::new();

        assert!(parser.feed(b"data: hello\r\n").unwrap().is_empty());

        let events = parser.finish().unwrap();
        assert_eq!(events[0].data, "hello");
        assert_eq!(events[0].event, "message");
    }

    #[test]
    fn persists_last_event_id_and_retry() {
        let mut parser = Parser::new();

        assert!(parser.feed(b"id: 42\nretry: 1500\n\n").unwrap().is_empty());
        assert_eq!(parser.last_event_id(), Some("42"));
        assert_eq!(parser.retry(), Some(1500));

        let events = parser.feed(b"data: ok\n\n").unwrap();
        assert_eq!(events[0].id.as_deref(), Some("42"));
    }

    #[test]
    fn strips_utf8_bom() {
        let mut parser = Parser::new();
        let input = [
            0xEF, 0xBB, 0xBF, b'd', b'a', b't', b'a', b':', b' ', b'x', b'\n', b'\n',
        ];
        let events = parser.feed(&input).unwrap();

        assert_eq!(events[0].data, "x");
    }

    #[test]
    fn handles_crlf_split_across_chunks() {
        let mut parser = Parser::new();

        assert!(parser.feed(b"data: hello\r").unwrap().is_empty());

        let events = parser.feed(b"\n\r\n").unwrap();
        assert_eq!(events[0].data, "hello");
    }

    #[test]
    fn handles_line_split_across_chunks() {
        let mut parser = Parser::new();

        assert!(parser.feed(b"data: hello").unwrap().is_empty());

        let events = parser.feed(b"\ndata: world\n\n").unwrap();
        assert_eq!(events[0].data, "hello\nworld");
    }
}

//! Minimal, forgiving Server-Sent Events parser (LF, CRLF and CR line ends,
//! multi-line `data:`, comments / keep-alives ignored).

#[derive(Debug, Default, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

#[derive(Default)]
pub struct SseParser {
    buf: Vec<u8>,
    event: Option<String>,
    data: Vec<String>,
}

impl SseParser {
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        self.buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        while let Some(pos) = self.buf.iter().position(|b| *b == b'\n' || *b == b'\r') {
            let mut end = pos + 1;
            if self.buf[pos] == b'\r' {
                match self.buf.get(pos + 1) {
                    Some(b'\n') => end += 1,
                    None => break, // could be CRLF split across chunks; wait
                    _ => {}
                }
            }
            let line = String::from_utf8_lossy(&self.buf[..pos]).into_owned();
            self.buf.drain(..end);
            if let Some(ev) = self.line(&line) {
                out.push(ev);
            }
        }
        out
    }

    /// Stream ended: dispatch a trailing event that lacked its blank line.
    pub fn finish(&mut self) -> Option<SseEvent> {
        if !self.buf.is_empty() {
            let line = String::from_utf8_lossy(&std::mem::take(&mut self.buf)).into_owned();
            let line = line.trim_end_matches(['\r', '\n']).to_string();
            if let Some(ev) = self.line(&line) {
                return Some(ev);
            }
        }
        self.dispatch()
    }

    fn line(&mut self, line: &str) -> Option<SseEvent> {
        if line.is_empty() {
            return self.dispatch();
        }
        if line.starts_with(':') {
            return None;
        }
        let (field, value) = match line.split_once(':') {
            Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
            None => (line, ""),
        };
        match field {
            "data" => self.data.push(value.to_string()),
            "event" => self.event = Some(value.to_string()),
            _ => {}
        }
        None
    }

    fn dispatch(&mut self) -> Option<SseEvent> {
        let event = self.event.take();
        if self.data.is_empty() {
            return None;
        }
        Some(SseEvent { event, data: std::mem::take(&mut self.data).join("\n") })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_crlf_multiline_comments_and_split_chunks() {
        let mut p = SseParser::default();
        let mut evs = p.feed(b": keep-alive\r\n\r\nevent: x\r\ndata: a\r\ndata: b\r");
        evs.extend(p.feed(b"\n\r\ndata: [DONE]\n\n"));
        assert_eq!(evs.len(), 2);
        assert_eq!(evs[0], SseEvent { event: Some("x".into()), data: "a\nb".into() });
        assert_eq!(evs[1].data, "[DONE]");
    }

    #[test]
    fn finish_flushes_unterminated_event() {
        let mut p = SseParser::default();
        assert!(p.feed(b"data: tail").is_empty());
        assert_eq!(p.finish().unwrap().data, "tail");
    }
}

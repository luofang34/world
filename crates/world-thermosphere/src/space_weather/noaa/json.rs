//! Zero-dependency pure-Rust RFC 8259 JSON parser for NOAA SWPC
//! `f107_cm_flux.json` records.

use alloc::string::String;
use alloc::vec::Vec;

use crate::space_weather::calendar::CalendarDate;
use crate::space_weather::error::{SpaceWeatherError, SpaceWeatherErrorKind};
use crate::space_weather::fields::parse_time_tag;
use crate::space_weather::noaa::{NoaaF107Entry, ReportingSchedule};

const MAX_SKIP_DEPTH: usize = 32;

pub(crate) fn parse_entries(text: &str) -> Result<Vec<NoaaF107Entry>, SpaceWeatherError> {
    let mut cursor = Cursor::new(text);
    cursor.expect_byte(b'[')?;
    cursor.skip_ws();
    let mut entries = Vec::new();
    if cursor.peek() == Some(b']') {
        let _ = cursor.advance();
        cursor.finish()?;
        return Ok(entries);
    }
    loop {
        entries.push(cursor.parse_entry()?);
        cursor.skip_ws();
        match cursor.advance() {
            Some(b',') => continue,
            Some(b']') => break,
            _ => {
                return Err(cursor.malformed("expected `,` or `]` in JSON array"));
            }
        }
    }
    cursor.finish()?;
    Ok(entries)
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
    line: usize,
}

impl<'a> Cursor<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            bytes: text.as_bytes(),
            pos: 0,
            line: 1,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let b = self.bytes.get(self.pos).copied()?;
        self.pos = self.pos.wrapping_add(1);
        if b == b'\n' {
            self.line = self.line.wrapping_add(1);
        }
        Some(b)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            let _ = self.advance();
        }
    }

    fn malformed(&self, detail: &'static str) -> SpaceWeatherError {
        SpaceWeatherError::at_line(self.line, SpaceWeatherErrorKind::Malformed(detail))
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), SpaceWeatherError> {
        self.skip_ws();
        match self.advance() {
            Some(b) if b == expected => Ok(()),
            _ => Err(self.malformed("unexpected JSON token")),
        }
    }

    fn finish(&mut self) -> Result<(), SpaceWeatherError> {
        self.skip_ws();
        if self.peek().is_some() {
            return Err(self.malformed("trailing characters after JSON array"));
        }
        Ok(())
    }

    fn consume_literal(&mut self, lit: &[u8]) -> Result<(), SpaceWeatherError> {
        for &expected in lit {
            if self.advance() != Some(expected) {
                return Err(self.malformed("invalid JSON keyword"));
            }
        }
        Ok(())
    }

    fn consume_null(&mut self) -> bool {
        self.skip_ws();
        if self.peek() == Some(b'n') {
            self.consume_literal(b"null").is_ok()
        } else {
            false
        }
    }

    fn parse_entry(&mut self) -> Result<NoaaF107Entry, SpaceWeatherError> {
        self.expect_byte(b'{')?;
        let obj_line = self.line;
        self.skip_ws();
        let mut date: Option<CalendarDate> = None;
        let mut time_seconds: u32 = 0;
        let mut flux: Option<f64> = None;
        let mut frequency: Option<f64> = None;
        let mut schedule = ReportingSchedule::Other;
        let mut ninety_day_mean: Option<f64> = None;

        if self.peek() == Some(b'}') {
            let _ = self.advance();
            return Err(SpaceWeatherError::at_line(
                obj_line,
                SpaceWeatherErrorKind::MissingHeader("time_tag"),
            ));
        }

        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.expect_byte(b':')?;
            self.skip_ws();
            match key.as_str() {
                "time_tag" => {
                    let raw = self.parse_string()?;
                    let (parsed_date, secs) = parse_time_tag(&raw, self.line)?;
                    date = Some(parsed_date);
                    time_seconds = secs;
                }
                "frequency" => {
                    frequency = self.parse_opt_number("frequency")?;
                }
                "flux" => {
                    flux = self.parse_opt_flux("flux")?;
                }
                "reporting_schedule" => {
                    schedule = self.parse_opt_schedule()?;
                }
                "ninety_day_mean" => {
                    ninety_day_mean = self.parse_opt_flux("ninety_day_mean")?;
                }
                _ => self.skip_value(0)?,
            }
            self.skip_ws();
            match self.advance() {
                Some(b',') => continue,
                Some(b'}') => break,
                _ => return Err(self.malformed("expected `,` or `}` in JSON object")),
            }
        }

        if let Some(freq) = frequency {
            if (freq - 2800.0).abs() > 1.0 {
                return Err(SpaceWeatherError::at_line(
                    obj_line,
                    SpaceWeatherErrorKind::InvalidValue {
                        field: "frequency",
                        value: freq,
                    },
                ));
            }
        }
        let date = date.ok_or(SpaceWeatherError::at_line(
            obj_line,
            SpaceWeatherErrorKind::MissingHeader("time_tag"),
        ))?;
        Ok(NoaaF107Entry {
            date,
            time_seconds,
            frequency_mhz: frequency,
            flux,
            schedule,
            ninety_day_mean,
        })
    }

    fn parse_opt_number(&mut self, field: &'static str) -> Result<Option<f64>, SpaceWeatherError> {
        if self.consume_null() {
            return Ok(None);
        }
        self.parse_number(field).map(Some)
    }

    fn parse_opt_flux(&mut self, field: &'static str) -> Result<Option<f64>, SpaceWeatherError> {
        let Some(val) = self.parse_opt_number(field)? else {
            return Ok(None);
        };
        if val == -1.0 || val == -999.0 {
            return Ok(None);
        }
        if val <= 0.0 {
            return Err(SpaceWeatherError::at_line(
                self.line,
                SpaceWeatherErrorKind::InvalidValue { field, value: val },
            ));
        }
        Ok(Some(val))
    }

    fn parse_opt_schedule(&mut self) -> Result<ReportingSchedule, SpaceWeatherError> {
        if self.consume_null() {
            return Ok(ReportingSchedule::Other);
        }
        let s = self.parse_string()?;
        Ok(match s.as_str() {
            "Morning" => ReportingSchedule::Morning,
            "Noon" => ReportingSchedule::Noon,
            "Afternoon" => ReportingSchedule::Afternoon,
            _ => ReportingSchedule::Other,
        })
    }

    fn parse_string(&mut self) -> Result<String, SpaceWeatherError> {
        self.expect_byte(b'"')?;
        let mut out = String::new();
        let mut run_start = self.pos;
        while let Some(b) = self.peek() {
            if b == b'"' {
                self.flush_utf8_run(run_start, &mut out)?;
                let _ = self.advance();
                return Ok(out);
            }
            if b == b'\\' {
                self.flush_utf8_run(run_start, &mut out)?;
                let _ = self.advance();
                self.push_escape(&mut out)?;
                run_start = self.pos;
                continue;
            }
            if b < 0x20 {
                return Err(self.malformed("unescaped control character in JSON string"));
            }
            let _ = self.advance();
        }
        Err(self.malformed("unterminated JSON string"))
    }

    fn flush_utf8_run(&self, start: usize, out: &mut String) -> Result<(), SpaceWeatherError> {
        let slice = self
            .bytes
            .get(start..self.pos)
            .ok_or_else(|| self.malformed("invalid string span"))?;
        let text = core::str::from_utf8(slice)
            .map_err(|_| self.malformed("invalid UTF-8 in JSON string"))?;
        out.push_str(text);
        Ok(())
    }

    fn push_escape(&mut self, out: &mut String) -> Result<(), SpaceWeatherError> {
        match self.advance() {
            Some(b'"') => out.push('"'),
            Some(b'\\') => out.push('\\'),
            Some(b'/') => out.push('/'),
            Some(b'b') => out.push('\u{0008}'),
            Some(b'f') => out.push('\u{000C}'),
            Some(b'n') => out.push('\n'),
            Some(b'r') => out.push('\r'),
            Some(b't') => out.push('\t'),
            Some(b'u') => out.push(self.parse_unicode_escape()?),
            _ => return Err(self.malformed("invalid JSON string escape")),
        }
        Ok(())
    }

    fn parse_unicode_escape(&mut self) -> Result<char, SpaceWeatherError> {
        let first = self.parse_hex4()?;
        if (0xD800..=0xDBFF).contains(&first) {
            if self.advance() != Some(b'\\') || self.advance() != Some(b'u') {
                return Err(self.malformed("unpaired UTF-16 high surrogate"));
            }
            let second = self.parse_hex4()?;
            if !(0xDC00..=0xDFFF).contains(&second) {
                return Err(self.malformed("invalid UTF-16 low surrogate"));
            }
            let code = 0x10000 + ((u32::from(first - 0xD800) << 10) | u32::from(second - 0xDC00));
            return char::from_u32(code).ok_or_else(|| self.malformed("invalid unicode scalar"));
        }
        if (0xDC00..=0xDFFF).contains(&first) {
            return Err(self.malformed("unexpected UTF-16 low surrogate"));
        }
        char::from_u32(u32::from(first)).ok_or_else(|| self.malformed("invalid unicode scalar"))
    }

    fn parse_hex4(&mut self) -> Result<u16, SpaceWeatherError> {
        let mut value = 0u16;
        for _ in 0..4 {
            let b = self
                .advance()
                .ok_or_else(|| self.malformed("truncated \\u escape"))?;
            let nibble = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return Err(self.malformed("invalid hex digit in \\u escape")),
            };
            value = (value << 4) | u16::from(nibble);
        }
        Ok(value)
    }

    fn parse_number(&mut self, field: &'static str) -> Result<f64, SpaceWeatherError> {
        self.skip_ws();
        let line = self.line;
        let start = self.pos;
        if self.peek() == Some(b'-') {
            let _ = self.advance();
        }
        match self.peek() {
            Some(b'0') => {
                let _ = self.advance();
            }
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    let _ = self.advance();
                }
            }
            _ => {
                return Err(SpaceWeatherError::at_line(
                    line,
                    SpaceWeatherErrorKind::BadNumber(field),
                ));
            }
        }
        if self.peek() == Some(b'.') {
            let _ = self.advance();
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(SpaceWeatherError::at_line(
                    line,
                    SpaceWeatherErrorKind::BadNumber(field),
                ));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                let _ = self.advance();
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            let _ = self.advance();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                let _ = self.advance();
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(SpaceWeatherError::at_line(
                    line,
                    SpaceWeatherErrorKind::BadNumber(field),
                ));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                let _ = self.advance();
            }
        }
        let slice = self
            .bytes
            .get(start..self.pos)
            .ok_or_else(|| self.malformed("invalid number span"))?;
        let text = core::str::from_utf8(slice).map_err(|_| {
            SpaceWeatherError::at_line(line, SpaceWeatherErrorKind::BadNumber(field))
        })?;
        let val = text.parse::<f64>().map_err(|_| {
            SpaceWeatherError::at_line(line, SpaceWeatherErrorKind::BadNumber(field))
        })?;
        if !val.is_finite() {
            return Err(SpaceWeatherError::at_line(
                line,
                SpaceWeatherErrorKind::InvalidValue { field, value: val },
            ));
        }
        Ok(val)
    }

    fn skip_value(&mut self, depth: usize) -> Result<(), SpaceWeatherError> {
        if depth > MAX_SKIP_DEPTH {
            return Err(self.malformed("JSON nesting depth exceeded"));
        }
        self.skip_ws();
        match self.peek() {
            Some(b'"') => {
                let _ = self.parse_string()?;
                Ok(())
            }
            Some(b'n') => self.consume_literal(b"null"),
            Some(b't') => self.consume_literal(b"true"),
            Some(b'f') => self.consume_literal(b"false"),
            Some(b'-' | b'0'..=b'9') => {
                let _ = self.parse_number("json")?;
                Ok(())
            }
            Some(b'[') => self.skip_array(depth),
            Some(b'{') => self.skip_object(depth),
            _ => Err(self.malformed("unexpected JSON value")),
        }
    }

    fn skip_array(&mut self, depth: usize) -> Result<(), SpaceWeatherError> {
        let _ = self.advance();
        self.skip_ws();
        if self.peek() == Some(b']') {
            let _ = self.advance();
            return Ok(());
        }
        loop {
            self.skip_value(depth.wrapping_add(1))?;
            self.skip_ws();
            match self.advance() {
                Some(b',') => continue,
                Some(b']') => return Ok(()),
                _ => return Err(self.malformed("expected `,` or `]` while skipping array")),
            }
        }
    }

    fn skip_object(&mut self, depth: usize) -> Result<(), SpaceWeatherError> {
        let _ = self.advance();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            let _ = self.advance();
            return Ok(());
        }
        loop {
            self.skip_ws();
            let _ = self.parse_string()?;
            self.expect_byte(b':')?;
            self.skip_value(depth.wrapping_add(1))?;
            self.skip_ws();
            match self.advance() {
                Some(b',') => continue,
                Some(b'}') => return Ok(()),
                _ => return Err(self.malformed("expected `,` or `}` while skipping object")),
            }
        }
    }
}

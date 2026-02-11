use crate::error::SlkError;

#[derive(Debug, PartialEq, Clone)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            JsonValue::Array(a) => Some(a),
            _ => None,
        }
    }
}

pub fn parse(input: &str) -> Result<JsonValue, SlkError> {
    let mut parser = Parser::new(input);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.pos < parser.input.len() {
        return Err(parser.error("unexpected trailing content"));
    }
    Ok(value)
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Parser {
            input: input.as_bytes(),
            pos: 0,
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, SlkError> {
        self.skip_whitespace();
        match self.peek()? {
            b'"' => self.parse_string().map(JsonValue::String),
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b't' | b'f' => self.parse_bool(),
            b'n' => self.parse_null(),
            b'-' | b'0'..=b'9' => self.parse_number(),
            ch => Err(self.error(&format!("unexpected character: '{}'", ch as char))),
        }
    }

    fn parse_string(&mut self) -> Result<String, SlkError> {
        self.expect(b'"')?;
        let mut s = String::new();
        loop {
            let ch = self.advance()?;
            match ch {
                b'"' => return Ok(s),
                b'\\' => {
                    let escaped = self.advance()?;
                    match escaped {
                        b'"' => s.push('"'),
                        b'\\' => s.push('\\'),
                        b'/' => s.push('/'),
                        b'n' => s.push('\n'),
                        b'r' => s.push('\r'),
                        b't' => s.push('\t'),
                        b'u' => {
                            let cp = self.parse_unicode_escape()?;
                            if (0xD800..=0xDBFF).contains(&cp) {
                                self.expect(b'\\')?;
                                self.expect(b'u')?;
                                let low = self.parse_unicode_escape()?;
                                let combined =
                                    0x10000 + ((cp as u32 - 0xD800) << 10) + (low as u32 - 0xDC00);
                                let c = char::from_u32(combined)
                                    .ok_or_else(|| self.error("invalid surrogate pair"))?;
                                s.push(c);
                            } else {
                                let c = char::from_u32(cp as u32)
                                    .ok_or_else(|| self.error("invalid unicode codepoint"))?;
                                s.push(c);
                            }
                        }
                        _ => return Err(self.error(&format!("invalid escape: \\{}", escaped as char))),
                    }
                }
                _ => s.push(ch as char),
            }
        }
    }

    fn parse_unicode_escape(&mut self) -> Result<u16, SlkError> {
        let mut val: u16 = 0;
        for _ in 0..4 {
            let ch = self.advance()?;
            let digit = match ch {
                b'0'..=b'9' => ch - b'0',
                b'a'..=b'f' => ch - b'a' + 10,
                b'A'..=b'F' => ch - b'A' + 10,
                _ => return Err(self.error("invalid unicode escape hex digit")),
            };
            val = val * 16 + digit as u16;
        }
        Ok(val)
    }

    fn parse_number(&mut self) -> Result<JsonValue, SlkError> {
        let start = self.pos;
        if self.peek_matches(b'-') {
            self.pos += 1;
        }
        self.consume_digits()?;
        if self.peek_matches(b'.') {
            self.pos += 1;
            self.consume_digits()?;
        }
        if self.pos < self.input.len() && (self.input[self.pos] == b'e' || self.input[self.pos] == b'E') {
            self.pos += 1;
            if self.pos < self.input.len() && (self.input[self.pos] == b'+' || self.input[self.pos] == b'-') {
                self.pos += 1;
            }
            self.consume_digits()?;
        }
        let num_str = std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| self.error("invalid UTF-8 in number"))?;
        let n: f64 = num_str
            .parse()
            .map_err(|_| self.error(&format!("invalid number: {}", num_str)))?;
        Ok(JsonValue::Number(n))
    }

    fn consume_digits(&mut self) -> Result<(), SlkError> {
        if self.pos >= self.input.len() || !self.input[self.pos].is_ascii_digit() {
            return Err(self.error("expected digit"));
        }
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        Ok(())
    }

    fn parse_object(&mut self) -> Result<JsonValue, SlkError> {
        self.expect(b'{')?;
        self.skip_whitespace();
        let mut pairs = Vec::new();
        if self.peek_matches(b'}') {
            self.pos += 1;
            return Ok(JsonValue::Object(pairs));
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            let value = self.parse_value()?;
            pairs.push((key, value));
            self.skip_whitespace();
            let ch = self.advance()?;
            match ch {
                b'}' => return Ok(JsonValue::Object(pairs)),
                b',' => continue,
                _ => return Err(self.error("expected ',' or '}' in object")),
            }
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue, SlkError> {
        self.expect(b'[')?;
        self.skip_whitespace();
        let mut items = Vec::new();
        if self.peek_matches(b']') {
            self.pos += 1;
            return Ok(JsonValue::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_whitespace();
            let ch = self.advance()?;
            match ch {
                b']' => return Ok(JsonValue::Array(items)),
                b',' => continue,
                _ => return Err(self.error("expected ',' or ']' in array")),
            }
        }
    }

    fn parse_bool(&mut self) -> Result<JsonValue, SlkError> {
        if self.starts_with(b"true") {
            self.pos += 4;
            Ok(JsonValue::Bool(true))
        } else if self.starts_with(b"false") {
            self.pos += 5;
            Ok(JsonValue::Bool(false))
        } else {
            Err(self.error("expected 'true' or 'false'"))
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, SlkError> {
        if self.starts_with(b"null") {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err(self.error("expected 'null'"))
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && matches!(self.input[self.pos], b' ' | b'\t' | b'\n' | b'\r') {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Result<u8, SlkError> {
        self.input
            .get(self.pos)
            .copied()
            .ok_or_else(|| self.error("unexpected end of input"))
    }

    fn peek_matches(&self, ch: u8) -> bool {
        self.pos < self.input.len() && self.input[self.pos] == ch
    }

    fn advance(&mut self) -> Result<u8, SlkError> {
        let ch = self.peek()?;
        self.pos += 1;
        Ok(ch)
    }

    fn expect(&mut self, expected: u8) -> Result<(), SlkError> {
        let ch = self.advance()?;
        if ch != expected {
            Err(self.error(&format!(
                "expected '{}', found '{}'",
                expected as char, ch as char
            )))
        } else {
            Ok(())
        }
    }

    fn starts_with(&self, prefix: &[u8]) -> bool {
        self.input[self.pos..].starts_with(prefix)
    }

    fn error(&self, msg: &str) -> SlkError {
        SlkError {
            message: format!("JSON parse error at position {}: {}", self.pos, msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_string() {
        assert_eq!(
            parse(r#""hello""#).unwrap(),
            JsonValue::String("hello".to_string())
        );
    }

    #[test]
    fn test_parse_escaped_string() {
        assert_eq!(
            parse(r#""hello \"world\"""#).unwrap(),
            JsonValue::String("hello \"world\"".to_string())
        );
        assert_eq!(
            parse(r#""line\nbreak""#).unwrap(),
            JsonValue::String("line\nbreak".to_string())
        );
        assert_eq!(
            parse(r#""tab\there""#).unwrap(),
            JsonValue::String("tab\there".to_string())
        );
        assert_eq!(
            parse(r#""back\\slash""#).unwrap(),
            JsonValue::String("back\\slash".to_string())
        );
    }

    #[test]
    fn test_parse_unicode_escape() {
        assert_eq!(
            parse(r#""\u0041""#).unwrap(),
            JsonValue::String("A".to_string())
        );
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse("true").unwrap(), JsonValue::Bool(true));
        assert_eq!(parse("false").unwrap(), JsonValue::Bool(false));
    }

    #[test]
    fn test_parse_null() {
        assert_eq!(parse("null").unwrap(), JsonValue::Null);
    }

    #[test]
    fn test_parse_number_integer() {
        assert_eq!(parse("42").unwrap(), JsonValue::Number(42.0));
        assert_eq!(parse("-7").unwrap(), JsonValue::Number(-7.0));
        assert_eq!(parse("0").unwrap(), JsonValue::Number(0.0));
    }

    #[test]
    fn test_parse_number_decimal() {
        assert_eq!(parse("3.14").unwrap(), JsonValue::Number(3.14));
        assert_eq!(parse("-0.5").unwrap(), JsonValue::Number(-0.5));
    }

    #[test]
    fn test_parse_number_exponent() {
        assert_eq!(parse("1e10").unwrap(), JsonValue::Number(1e10));
        assert_eq!(parse("2.5E-3").unwrap(), JsonValue::Number(2.5e-3));
    }

    #[test]
    fn test_parse_empty_object() {
        assert_eq!(parse("{}").unwrap(), JsonValue::Object(vec![]));
    }

    #[test]
    fn test_parse_object_single_key() {
        let val = parse(r#"{"ok":true}"#).unwrap();
        assert_eq!(
            val,
            JsonValue::Object(vec![("ok".to_string(), JsonValue::Bool(true))])
        );
    }

    #[test]
    fn test_parse_empty_array() {
        assert_eq!(parse("[]").unwrap(), JsonValue::Array(vec![]));
    }

    #[test]
    fn test_parse_array() {
        assert_eq!(
            parse("[1, 2, 3]").unwrap(),
            JsonValue::Array(vec![
                JsonValue::Number(1.0),
                JsonValue::Number(2.0),
                JsonValue::Number(3.0),
            ])
        );
    }

    #[test]
    fn test_parse_nested_slack_response() {
        let input = r#"{
            "ok": true,
            "messages": [
                {
                    "user": "U123",
                    "text": "hello",
                    "ts": "1770689887.565249"
                },
                {
                    "user": "U456",
                    "text": "world"
                }
            ],
            "has_more": false
        }"#;
        let val = parse(input).unwrap();

        assert_eq!(val.get("ok").unwrap().as_bool(), Some(true));

        let messages = val.get("messages").unwrap().as_array().unwrap();
        assert_eq!(messages.len(), 2);

        assert_eq!(messages[0].get("user").unwrap().as_str(), Some("U123"));
        assert_eq!(messages[0].get("text").unwrap().as_str(), Some("hello"));
        assert_eq!(messages[1].get("user").unwrap().as_str(), Some("U456"));
        assert_eq!(messages[1].get("text").unwrap().as_str(), Some("world"));

        assert_eq!(val.get("has_more").unwrap().as_bool(), Some(false));
    }

    #[test]
    fn test_parse_null_in_object() {
        let val = parse(r#"{"value": null}"#).unwrap();
        assert_eq!(val.get("value"), Some(&JsonValue::Null));
    }

    #[test]
    fn test_parse_error_unclosed_string() {
        assert!(parse(r#""unclosed"#).is_err());
    }

    #[test]
    fn test_parse_error_unexpected_char() {
        assert!(parse("@invalid").is_err());
    }

    #[test]
    fn test_parse_error_trailing_content() {
        assert!(parse("true false").is_err());
    }

    #[test]
    fn test_parse_whitespace_handling() {
        let val = parse("  { \"a\" : 1 }  ").unwrap();
        assert_eq!(
            val,
            JsonValue::Object(vec![("a".to_string(), JsonValue::Number(1.0))])
        );
    }

    #[test]
    fn test_get_returns_none_for_missing_key() {
        let val = parse(r#"{"a": 1}"#).unwrap();
        assert_eq!(val.get("b"), None);
    }

    #[test]
    fn test_get_returns_none_for_non_object() {
        let val = parse("42").unwrap();
        assert_eq!(val.get("key"), None);
    }
}

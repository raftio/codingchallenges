use std::env;
use std::fs;
use std::io::Read;
use std::process;

// ── Tokens ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Colon,
    Comma,
    String(String),
    Number(String),
    True,
    False,
    Null,
}

// ── Lexer ─────────────────────────────────────────────────────────────────────

struct Lexer<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Lexer { input: input.as_bytes(), pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn next_byte(&mut self) -> Option<u8> {
        let c = self.input.get(self.pos).copied();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.next_byte();
        }
    }

    fn read_string(&mut self) -> Result<Token, String> {
        self.next_byte(); // consume opening "
        let mut s = String::new();
        loop {
            match self.next_byte() {
                None => return Err("Unterminated string literal".into()),
                Some(b'"') => break,
                Some(b'\\') => match self.next_byte() {
                    None => return Err("Unterminated escape sequence".into()),
                    Some(b'"') => s.push('"'),
                    Some(b'\\') => s.push('\\'),
                    Some(b'/') => s.push('/'),
                    Some(b'b') => s.push('\x08'),
                    Some(b'f') => s.push('\x0C'),
                    Some(b'n') => s.push('\n'),
                    Some(b'r') => s.push('\r'),
                    Some(b't') => s.push('\t'),
                    Some(b'u') => {
                        let mut hex = String::new();
                        for _ in 0..4 {
                            match self.next_byte() {
                                Some(c) if (c as char).is_ascii_hexdigit() => {
                                    hex.push(c as char)
                                }
                                _ => return Err("Invalid \\uXXXX escape".into()),
                            }
                        }
                        let cp = u32::from_str_radix(&hex, 16).unwrap();
                        // Handle surrogate pairs
                        if (0xD800..=0xDBFF).contains(&cp) {
                            // High surrogate – expect \uLow next
                            if self.next_byte() != Some(b'\\') || self.next_byte() != Some(b'u') {
                                return Err("Expected low surrogate after high surrogate".into());
                            }
                            let mut hex2 = String::new();
                            for _ in 0..4 {
                                match self.next_byte() {
                                    Some(c) if (c as char).is_ascii_hexdigit() => {
                                        hex2.push(c as char)
                                    }
                                    _ => return Err("Invalid \\uXXXX escape".into()),
                                }
                            }
                            let low = u32::from_str_radix(&hex2, 16).unwrap();
                            if !(0xDC00..=0xDFFF).contains(&low) {
                                return Err("Invalid surrogate pair".into());
                            }
                            let code_point =
                                0x10000 + ((cp - 0xD800) << 10) + (low - 0xDC00);
                            match char::from_u32(code_point) {
                                Some(c) => s.push(c),
                                None => return Err("Invalid unicode code point".into()),
                            }
                        } else if (0xDC00..=0xDFFF).contains(&cp) {
                            return Err("Unexpected low surrogate".into());
                        } else {
                            match char::from_u32(cp) {
                                Some(c) => s.push(c),
                                None => return Err(format!("Invalid unicode code point U+{cp:04X}")),
                            }
                        }
                    }
                    Some(c) => return Err(format!("Invalid escape '\\{}'", c as char)),
                },
                Some(c) => {
                    if c < 0x20 {
                        return Err(format!(
                            "Unescaped control character 0x{c:02X} in string"
                        ));
                    }
                    if c < 0x80 {
                        s.push(c as char);
                    } else {
                        // Multi-byte UTF-8
                        let extra = if c & 0xE0 == 0xC0 {
                            1
                        } else if c & 0xF0 == 0xE0 {
                            2
                        } else if c & 0xF8 == 0xF0 {
                            3
                        } else {
                            return Err("Invalid UTF-8 byte in string".into());
                        };
                        let mut bytes = vec![c];
                        for _ in 0..extra {
                            match self.next_byte() {
                                Some(b) => bytes.push(b),
                                None => return Err("Truncated UTF-8 sequence".into()),
                            }
                        }
                        match std::str::from_utf8(&bytes) {
                            Ok(st) => s.push_str(st),
                            Err(_) => return Err("Invalid UTF-8 sequence".into()),
                        }
                    }
                }
            }
        }
        Ok(Token::String(s))
    }

    fn read_number(&mut self) -> Result<Token, String> {
        let start = self.pos;

        // Optional leading minus
        if self.peek() == Some(b'-') {
            self.next_byte();
        }

        // Integer part
        match self.peek() {
            Some(b'0') => {
                self.next_byte();
                if matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                    return Err("Leading zeros are not allowed in JSON numbers".into());
                }
            }
            Some(c) if c.is_ascii_digit() => {
                while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                    self.next_byte();
                }
            }
            _ => return Err("Invalid number: missing digits".into()),
        }

        // Optional fractional part
        if self.peek() == Some(b'.') {
            self.next_byte();
            if !matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                return Err("Expected digit after decimal point".into());
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.next_byte();
            }
        }

        // Optional exponent
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.next_byte();
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.next_byte();
            }
            if !matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                return Err("Expected digit in exponent".into());
            }
            while matches!(self.peek(), Some(c) if c.is_ascii_digit()) {
                self.next_byte();
            }
        }

        let s = std::str::from_utf8(&self.input[start..self.pos]).unwrap().to_string();
        Ok(Token::Number(s))
    }

    fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                None => break,
                Some(b'{') => { self.next_byte(); tokens.push(Token::LBrace); }
                Some(b'}') => { self.next_byte(); tokens.push(Token::RBrace); }
                Some(b'[') => { self.next_byte(); tokens.push(Token::LBracket); }
                Some(b']') => { self.next_byte(); tokens.push(Token::RBracket); }
                Some(b':') => { self.next_byte(); tokens.push(Token::Colon); }
                Some(b',') => { self.next_byte(); tokens.push(Token::Comma); }
                Some(b'"') => tokens.push(self.read_string()?),
                Some(b'-') | Some(b'0'..=b'9') => tokens.push(self.read_number()?),
                Some(b't') => {
                    if self.input[self.pos..].starts_with(b"true") {
                        self.pos += 4;
                        tokens.push(Token::True);
                    } else {
                        return Err(format!("Invalid token at position {}", self.pos));
                    }
                }
                Some(b'f') => {
                    if self.input[self.pos..].starts_with(b"false") {
                        self.pos += 5;
                        tokens.push(Token::False);
                    } else {
                        return Err(format!("Invalid token at position {}", self.pos));
                    }
                }
                Some(b'n') => {
                    if self.input[self.pos..].starts_with(b"null") {
                        self.pos += 4;
                        tokens.push(Token::Null);
                    } else {
                        return Err(format!("Invalid token at position {}", self.pos));
                    }
                }
                Some(c) => {
                    return Err(format!("Unexpected character '{}' at position {}", c as char, self.pos))
                }
            }
        }
        Ok(tokens)
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next_token(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn parse_value(&mut self) -> Result<(), String> {
        match self.peek() {
            Some(Token::LBrace) => self.parse_object(),
            Some(Token::LBracket) => self.parse_array(),
            Some(Token::String(_) | Token::Number(_) | Token::True | Token::False | Token::Null) => {
                self.next_token();
                Ok(())
            }
            Some(t) => Err(format!("Unexpected token {:?}", t)),
            None => Err("Unexpected end of input, expected a value".into()),
        }
    }

    fn parse_object(&mut self) -> Result<(), String> {
        self.next_token(); // consume '{'

        if self.peek() == Some(&Token::RBrace) {
            self.next_token();
            return Ok(());
        }

        loop {
            // Key must be a string
            match self.next_token() {
                Some(Token::String(_)) => {}
                Some(t) => return Err(format!("Expected string key in object, got {:?}", t)),
                None => return Err("Expected string key, got end of input".into()),
            }

            // Colon separator
            match self.next_token() {
                Some(Token::Colon) => {}
                Some(t) => return Err(format!("Expected ':', got {:?}", t)),
                None => return Err("Expected ':', got end of input".into()),
            }

            // Value
            self.parse_value()?;

            match self.peek() {
                Some(Token::Comma) => {
                    self.next_token();
                    // Trailing comma check
                    if self.peek() == Some(&Token::RBrace) {
                        return Err("Trailing comma in object".into());
                    }
                }
                Some(Token::RBrace) => {
                    self.next_token();
                    break;
                }
                Some(t) => return Err(format!("Expected ',' or '}}', got {:?}", t)),
                None => return Err("Expected ',' or '}}', got end of input".into()),
            }
        }
        Ok(())
    }

    fn parse_array(&mut self) -> Result<(), String> {
        self.next_token(); // consume '['

        if self.peek() == Some(&Token::RBracket) {
            self.next_token();
            return Ok(());
        }

        loop {
            self.parse_value()?;

            match self.peek() {
                Some(Token::Comma) => {
                    self.next_token();
                    // Trailing comma check
                    if self.peek() == Some(&Token::RBracket) {
                        return Err("Trailing comma in array".into());
                    }
                }
                Some(Token::RBracket) => {
                    self.next_token();
                    break;
                }
                Some(t) => return Err(format!("Expected ',' or ']', got {:?}", t)),
                None => return Err("Expected ',' or ']', got end of input".into()),
            }
        }
        Ok(())
    }

    fn parse(&mut self) -> Result<(), String> {
        if self.tokens.is_empty() {
            return Err("Empty input".into());
        }
        self.parse_value()?;
        if self.pos < self.tokens.len() {
            return Err(format!(
                "Unexpected token after JSON value: {:?}",
                self.tokens[self.pos]
            ));
        }
        Ok(())
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn parse_json(input: &str) -> Result<(), String> {
    let mut lexer = Lexer::new(input);
    let tokens = lexer.tokenize()?;
    Parser::new(tokens).parse()
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let input = if args.len() > 1 {
        match fs::read_to_string(&args[1]) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading '{}': {}", args[1], e);
                process::exit(1);
            }
        }
    } else {
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).unwrap();
        buf
    };

    match parse_json(&input) {
        Ok(_) => {
            println!("Valid JSON");
            process::exit(0);
        }
        Err(e) => {
            eprintln!("Invalid JSON: {}", e);
            process::exit(1);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::parse_json;

    fn valid(s: &str) {
        assert!(parse_json(s).is_ok(), "Expected valid, got error for: {s}");
    }

    fn invalid(s: &str) {
        assert!(parse_json(s).is_err(), "Expected invalid, but parsed ok: {s}");
    }

    // Step 1 – empty object
    #[test]
    fn test_empty_object() { valid("{}"); }
    #[test]
    fn test_truly_empty() { invalid(""); }
    #[test]
    fn test_bare_brace() { invalid("{"); }

    // Step 2 – string keys and values
    #[test]
    fn test_string_kv() { valid(r#"{"key": "value"}"#); }
    #[test]
    fn test_multiple_string_kv() {
        valid(r#"{"key1": "value1", "key2": "value2"}"#);
    }
    #[test]
    fn test_missing_value() { invalid(r#"{"key": }"#); }
    #[test]
    fn test_missing_colon() { invalid(r#"{"key" "value"}"#); }
    #[test]
    fn test_trailing_comma_object() { invalid(r#"{"key": "value",}"#); }

    // Step 3 – all scalar types
    #[test]
    fn test_scalars() {
        valid(r#"{"key1": true, "key2": false, "key3": null, "key4": "value", "key5": 101}"#);
    }
    #[test]
    fn test_number_negative() { valid(r#"{"n": -42}"#); }
    #[test]
    fn test_number_float() { valid(r#"{"n": 3.14}"#); }
    #[test]
    fn test_number_exp() { valid(r#"{"n": 1e10}"#); }
    #[test]
    fn test_number_leading_zero() { invalid(r#"{"n": 01}"#); }
    #[test]
    fn test_bool_typo() { invalid(r#"{"k": True}"#); }

    // Step 4 – nested objects and arrays
    #[test]
    fn test_nested_object() {
        valid(r#"{"key": "value", "key-n": 101, "key-o": {}, "key-l": []}"#);
    }
    #[test]
    fn test_array_values() { valid(r#"{"arr": [1, 2, 3]}"#); }
    #[test]
    fn test_deeply_nested() {
        valid(r#"{"a": {"b": {"c": [1, [2, 3]]}}}"#);
    }
    #[test]
    fn test_trailing_comma_array() { invalid(r#"{"arr": [1, 2,]}"#); }

    // String edge cases
    #[test]
    fn test_string_escape_sequences() {
        valid(r#"{"s": "line1\nline2\ttab\r\n"}"#);
    }
    #[test]
    fn test_string_unicode_escape() {
        valid(r#"{"s": "\u0041"}"#); // 'A'
    }
    #[test]
    fn test_string_invalid_escape() {
        invalid(r#"{"s": "\q"}"#);
    }
    #[test]
    fn test_unquoted_key() { invalid(r#"{key: "value"}"#); }

    // Top-level non-object values (valid JSON)
    #[test]
    fn test_top_level_array() { valid("[1, 2, 3]"); }
    #[test]
    fn test_top_level_string() { valid(r#""hello""#); }
    #[test]
    fn test_top_level_number() { valid("42"); }
    #[test]
    fn test_top_level_null() { valid("null"); }
}

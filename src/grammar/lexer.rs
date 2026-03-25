/// Lexer for the EBNF grammar language.
///
/// Tokenizes grammar text into a stream of tokens for the parser.

// ── Token ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Token {
  Ident(String),
  StringLit(String),
  RegexLit(String),
  Tilde,
  At,
  Dot,
  Pipe,
  LParen,
  RParen,
  LBracket,
  RBracket,
  LBrace,
  RBrace,
  Comma,
  Question,
  Star,
  Plus,
  Underscore,
  Equals,
  Semicolon,
  /// Capture slot reference: $N (where N is a digit)
  CaptureSlot(u8),
}

// ── Lexer ─────────────────────────────────────────────────────────────────

pub(crate) struct Lexer {
  chars: Vec<char>,
  pos: usize,
}

impl Lexer {
  pub(crate) fn new(input: &str) -> Self {
    Lexer {
      chars: input.chars().collect(),
      pos: 0,
    }
  }

  fn peek_char(&self) -> Option<char> {
    self.chars.get(self.pos).copied()
  }

  fn next_char(&mut self) -> Option<char> {
    let ch = self.chars.get(self.pos).copied();
    if ch.is_some() {
      self.pos += 1;
    }
    ch
  }

  fn skip_whitespace_and_comments(&mut self) {
    loop {
      // Skip whitespace
      while let Some(ch) = self.peek_char() {
        if ch.is_whitespace() {
          self.next_char();
        } else {
          break;
        }
      }
      // Skip line comments
      if self.pos + 1 < self.chars.len()
        && self.chars[self.pos] == '/'
        && self.chars[self.pos + 1] == '/'
      {
        while let Some(ch) = self.next_char() {
          if ch == '\n' {
            break;
          }
        }
      } else {
        break;
      }
    }
  }

  fn read_string_lit(&mut self) -> Result<String, String> {
    // Opening quote already consumed
    let mut s = String::new();
    loop {
      match self.next_char() {
        Some('"') => return Ok(s),
        Some('\\') => match self.next_char() {
          Some(c) => s.push(c),
          None => return Err("Unterminated escape in string".into()),
        },
        Some(c) => s.push(c),
        None => return Err("Unterminated string literal".into()),
      }
    }
  }

  fn is_ident_char(ch: char) -> bool {
    !ch.is_whitespace()
      && !matches!(
        ch,
        '.' | '@' | '|' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | '?' | '*' | '+' | '=' | ';' | '"' | '/'
          | '~' | '$'
      )
  }

  fn read_regex_lit(&mut self) -> Result<String, String> {
    // Opening '/' already consumed
    let mut s = String::new();
    loop {
      match self.next_char() {
        Some('/') => return Ok(s),
        Some('\\') => match self.next_char() {
          Some(c) => {
            s.push('\\');
            s.push(c);
          }
          None => return Err("Unterminated escape in regex".into()),
        },
        Some(c) => s.push(c),
        None => return Err("Unterminated regex literal".into()),
      }
    }
  }

  fn read_capture_digit(&mut self) -> Result<u8, String> {
    match self.peek_char() {
      Some(c) if c.is_ascii_digit() => {
        self.next_char();
        Ok(c as u8 - b'0')
      }
      other => Err(format!("Expected digit after '$', got {:?}", other)),
    }
  }

  fn read_ident(&mut self, first: char) -> String {
    let mut s = String::new();
    s.push(first);
    while let Some(ch) = self.peek_char() {
      if Self::is_ident_char(ch) {
        s.push(ch);
        self.next_char();
      } else {
        break;
      }
    }
    s
  }

  pub(crate) fn tokenize(&mut self) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    loop {
      self.skip_whitespace_and_comments();
      let ch = match self.peek_char() {
        Some(c) => c,
        None => break,
      };
      self.next_char();
      let tok = match ch {
        '@' => Token::At,
        '.' => Token::Dot,
        '|' => Token::Pipe,
        '(' => Token::LParen,
        ')' => Token::RParen,
        '[' => Token::LBracket,
        ']' => Token::RBracket,
        '{' => Token::LBrace,
        '}' => Token::RBrace,
        ',' => Token::Comma,
        '?' => Token::Question,
        '*' => Token::Star,
        '+' => Token::Plus,
        '=' => Token::Equals,
        ';' => Token::Semicolon,
        '~' => Token::Tilde,
        '/' => Token::RegexLit(self.read_regex_lit()?),
        '"' => Token::StringLit(self.read_string_lit()?),
        '_' => {
          // Check if underscore is followed by capture slot ($N)
          if self.peek_char() == Some('$') {
            self.next_char(); // consume $
            let n = self.read_capture_digit()?;
            Token::Ident(format!("_${}", n)) // sentinel: handled in parse_atom
          } else if let Some(next) = self.peek_char() {
            if Self::is_ident_char(next) && next != '_' {
              Token::Ident(self.read_ident('_'))
            } else {
              Token::Underscore
            }
          } else {
            Token::Underscore
          }
        }
        '$' => {
          let n = self.read_capture_digit()?;
          Token::CaptureSlot(n)
        }
        c if Self::is_ident_char(c) => Token::Ident(self.read_ident(c)),
        c => return Err(format!("Unexpected character: '{}'", c)),
      };
      tokens.push(tok);
    }
    Ok(tokens)
  }
}

/// Heuristic: rule names are ASCII identifiers (letters, digits, underscores).
/// POS tags contain Japanese characters.
pub(crate) fn is_rule_name(s: &str) -> bool {
  s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

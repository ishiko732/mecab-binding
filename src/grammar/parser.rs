/// Recursive-descent parser for the EBNF grammar language.
///
/// Grammar syntax:
///   grammar     = rule* ;
///   rule        = metadata? IDENT "=" expr ";" ;
///   metadata    = "[" level* ("," STRING)? "]" ;
///   expr        = seq_expr ("|" seq_expr)* ;
///   seq_expr    = quantified+ ;
///   quantified  = atom ("?" | "*" | "+")? ;
///   atom        = token_matcher | "(" expr ")" | "_" | IDENT ;
///   token_matcher = pos_path? string_lit? base_form? ;
///   pos_path    = IDENT ("." IDENT)* ;
///   string_lit  = '"' ... '"' ;
///   base_form   = "@" string_lit ;
use super::syntax::*;

// ── Lexer ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
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
}

struct Lexer {
  chars: Vec<char>,
  pos: usize,
}

impl Lexer {
  fn new(input: &str) -> Self {
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
          | '~'
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

  fn tokenize(&mut self) -> Result<Vec<Token>, String> {
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
          // Check if underscore is followed by ident chars (then it's part of ident)
          if let Some(next) = self.peek_char() {
            if Self::is_ident_char(next) && next != '_' {
              Token::Ident(self.read_ident('_'))
            } else {
              Token::Underscore
            }
          } else {
            Token::Underscore
          }
        }
        c if Self::is_ident_char(c) => Token::Ident(self.read_ident(c)),
        c => return Err(format!("Unexpected character: '{}'", c)),
      };
      tokens.push(tok);
    }
    Ok(tokens)
  }
}

// ── Parser ─────────────────────────────────────────────────────────────────

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

  fn next(&mut self) -> Option<&Token> {
    let tok = self.tokens.get(self.pos);
    if tok.is_some() {
      self.pos += 1;
    }
    tok
  }

  fn expect(&mut self, expected: &Token) -> Result<(), String> {
    match self.next() {
      Some(tok) if tok == expected => Ok(()),
      Some(tok) => Err(format!("Expected {:?}, got {:?}", expected, tok)),
      None => Err(format!("Expected {:?}, got end of input", expected)),
    }
  }

  /// Parse a string value that may be exact ("text"), suffix (~"text"), or regex (/pattern/).
  fn parse_string_value(&mut self) -> Result<StringMatcher, String> {
    match self.peek() {
      Some(Token::Tilde) => {
        self.next(); // consume ~
        match self.next() {
          Some(Token::StringLit(s)) => Ok(StringMatcher::Suffix(s.clone())),
          other => Err(format!("Expected string after '~', got {:?}", other)),
        }
      }
      Some(Token::RegexLit(_)) => {
        if let Some(Token::RegexLit(s)) = self.next() {
          let re = regex::Regex::new(&s.clone())
            .map_err(|e| format!("Invalid regex '{}': {}", s, e))?;
          Ok(StringMatcher::Regex(re))
        } else {
          unreachable!()
        }
      }
      Some(Token::StringLit(_)) => {
        if let Some(Token::StringLit(s)) = self.next() {
          Ok(StringMatcher::Exact(s.clone()))
        } else {
          unreachable!()
        }
      }
      other => Err(format!("Expected string value, got {:?}", other)),
    }
  }

  fn parse_grammar(&mut self) -> Result<Grammar, String> {
    let mut rules = Vec::new();
    while self.peek().is_some() {
      rules.push(self.parse_rule()?);
    }
    Ok(Grammar { rules })
  }

  fn parse_rule(&mut self) -> Result<Rule, String> {
    // Optional metadata: [N5 N4, "description"]
    let metadata = if self.peek() == Some(&Token::LBracket) {
      Some(self.parse_metadata()?)
    } else {
      None
    };

    // Rule name
    let name = match self.next() {
      Some(Token::Ident(s)) => s.clone(),
      other => return Err(format!("Expected rule name, got {:?}", other)),
    };

    self.expect(&Token::Equals)?;
    let pattern = self.parse_expr()?;
    self.expect(&Token::Semicolon)?;

    Ok(Rule {
      name,
      pattern,
      metadata,
    })
  }

  fn parse_metadata(&mut self) -> Result<RuleMetadata, String> {
    self.expect(&Token::LBracket)?;

    let mut levels = Vec::new();
    let mut description = None;

    // Read levels (identifiers) until we hit comma, "]", or string
    loop {
      match self.peek() {
        Some(Token::Ident(s)) => {
          levels.push(s.clone());
          self.next();
        }
        Some(Token::Comma) => {
          self.next();
          // Read description string
          match self.next() {
            Some(Token::StringLit(s)) => description = Some(s.clone()),
            other => return Err(format!("Expected description string, got {:?}", other)),
          }
          break;
        }
        Some(Token::RBracket) => break,
        Some(Token::StringLit(s)) => {
          // String without comma means description only, no levels
          description = Some(s.clone());
          self.next();
          break;
        }
        other => return Err(format!("Unexpected token in metadata: {:?}", other)),
      }
    }

    self.expect(&Token::RBracket)?;

    Ok(RuleMetadata {
      levels,
      description,
      connection: None,
      examples: Vec::new(),
    })
  }

  fn parse_expr(&mut self) -> Result<PatternExpr, String> {
    let first = self.parse_seq_expr()?;
    let mut alternatives = vec![first];

    while self.peek() == Some(&Token::Pipe) {
      self.next();
      alternatives.push(self.parse_seq_expr()?);
    }

    if alternatives.len() == 1 {
      Ok(alternatives.pop().unwrap())
    } else {
      Ok(PatternExpr::Alternative(alternatives))
    }
  }

  fn parse_seq_expr(&mut self) -> Result<PatternExpr, String> {
    let mut items = Vec::new();

    while let Some(tok) = self.peek() {
      // Stop at tokens that end a sequence context
      if matches!(tok, Token::Pipe | Token::RParen | Token::Semicolon) {
        break;
      }
      items.push(self.parse_quantified()?);
    }

    if items.is_empty() {
      return Err("Expected at least one pattern in sequence".into());
    }
    if items.len() == 1 {
      Ok(items.pop().unwrap())
    } else {
      Ok(PatternExpr::Sequence(items))
    }
  }

  fn parse_quantified(&mut self) -> Result<PatternExpr, String> {
    let atom = self.parse_atom()?;

    match self.peek() {
      Some(Token::Question) => {
        self.next();
        Ok(PatternExpr::Optional(Box::new(atom)))
      }
      Some(Token::Star) => {
        self.next();
        Ok(PatternExpr::ZeroOrMore(Box::new(atom)))
      }
      Some(Token::Plus) => {
        self.next();
        Ok(PatternExpr::OneOrMore(Box::new(atom)))
      }
      _ => Ok(atom),
    }
  }

  fn parse_atom(&mut self) -> Result<PatternExpr, String> {
    match self.peek() {
      Some(Token::LParen) => {
        self.next();
        let expr = self.parse_expr()?;
        self.expect(&Token::RParen)?;
        Ok(expr)
      }
      Some(Token::Underscore) => {
        self.next();
        Ok(PatternExpr::Wildcard)
      }
      Some(Token::StringLit(_)) | Some(Token::Tilde) | Some(Token::RegexLit(_)) | Some(Token::At) => {
        // Surface-only or base-form-only token matcher
        Ok(PatternExpr::Token(self.parse_token_predicate(None)?))
      }
      Some(Token::Ident(_)) => {
        // Could be: POS path (token matcher), or rule reference.
        // Peek ahead to decide: if followed by "=", it's a new rule (shouldn't be here).
        // If followed by dot, string, or @, it's a token matcher.
        // If followed by quantifier, pipe, rparen, semicolon, or another atom-start, it's ambiguous.
        // Strategy: parse as POS path first, then check for string/@.
        // If the ident looks like a rule reference (ASCII-only, appears as a rule name),
        // we'll resolve that at match time via RuleRef.
        let ident = match self.next() {
          Some(Token::Ident(s)) => s.clone(),
          _ => unreachable!(),
        };

        // Check if this is a POS path (may have dots)
        let mut pos_parts = vec![ident.clone()];
        while self.peek() == Some(&Token::Dot) {
          self.next();
          match self.next() {
            Some(Token::Ident(s)) => pos_parts.push(s.clone()),
            other => return Err(format!("Expected identifier after '.', got {:?}", other)),
          }
        }

        // Parse optional conjugation form: [活用形] or [~"suffix"] or [/regex/]
        let mut conjugation_form = None;
        if self.peek() == Some(&Token::LBracket) {
          self.next(); // consume [
          match self.peek() {
            Some(Token::Tilde) | Some(Token::RegexLit(_)) => {
              conjugation_form = Some(self.parse_string_value()?);
              self.expect(&Token::RBracket)?;
            }
            Some(Token::Ident(_)) => {
              if let Some(Token::Ident(cf)) = self.next() {
                conjugation_form = Some(StringMatcher::Exact(cf.clone()));
              }
              self.expect(&Token::RBracket)?;
            }
            other => return Err(format!("Expected conjugation form, got {:?}", other)),
          }
        }

        // Parse optional conjugation type: {活用型} or {~"suffix"} or {/regex/}
        let mut conjugation_type = None;
        if self.peek() == Some(&Token::LBrace) {
          self.next(); // consume {
          match self.peek() {
            Some(Token::Tilde) | Some(Token::RegexLit(_)) | Some(Token::StringLit(_)) => {
              conjugation_type = Some(self.parse_string_value()?);
              self.expect(&Token::RBrace)?;
            }
            Some(Token::Ident(_)) => {
              if let Some(Token::Ident(ct)) = self.next() {
                conjugation_type = Some(StringMatcher::Exact(ct.clone()));
              }
              self.expect(&Token::RBrace)?;
            }
            other => return Err(format!("Expected conjugation type, got {:?}", other)),
          }
        }

        // Check for surface or base_form constraints.
        // When conjugation_form or conjugation_type is set, the following string
        // literal starts a new atom in the sequence
        // (e.g. 形容詞[ガル接続] "さ" = two separate tokens).
        let mut surface = None;
        let mut base_form = None;

        if conjugation_form.is_none() && conjugation_type.is_none() {
          if matches!(
            self.peek(),
            Some(Token::StringLit(_)) | Some(Token::Tilde) | Some(Token::RegexLit(_))
          ) {
            surface = Some(self.parse_string_value()?);
          }

          if self.peek() == Some(&Token::At) {
            self.next();
            base_form = Some(self.parse_string_value()?);
          }
        }

        // If it's a bare identifier with no dots, no surface, no base_form,
        // no conjugation_form/type, and looks like it could be a rule reference
        // (ASCII letters + underscores), treat it as a rule ref.
        // Otherwise treat as POS token matcher.
        if pos_parts.len() == 1
          && surface.is_none()
          && base_form.is_none()
          && conjugation_form.is_none()
          && conjugation_type.is_none()
          && is_rule_name(&pos_parts[0])
        {
          Ok(PatternExpr::RuleRef(pos_parts.pop().unwrap()))
        } else {
          Ok(PatternExpr::Token(TokenPredicate {
            pos: pos_parts,
            surface,
            base_form,
            conjugation_form,
            conjugation_type,
          }))
        }
      }
      other => Err(format!("Unexpected token: {:?}", other)),
    }
  }

  fn parse_token_predicate(&mut self, pos: Option<Vec<String>>) -> Result<TokenPredicate, String> {
    let pos = pos.unwrap_or_default();
    let mut surface = None;
    let mut base_form = None;

    if matches!(
      self.peek(),
      Some(Token::StringLit(_)) | Some(Token::Tilde) | Some(Token::RegexLit(_))
    ) {
      surface = Some(self.parse_string_value()?);
    }

    if self.peek() == Some(&Token::At) {
      self.next();
      base_form = Some(self.parse_string_value()?);
    }

    Ok(TokenPredicate {
      pos,
      surface,
      base_form,
      conjugation_form: None,
      conjugation_type: None,
    })
  }
}

/// Heuristic: rule names are ASCII identifiers (letters, digits, underscores).
/// POS tags contain Japanese characters.
fn is_rule_name(s: &str) -> bool {
  s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse a grammar text string into a Grammar AST.
pub fn parse_grammar(input: &str) -> Result<Grammar, String> {
  let mut lexer = Lexer::new(input);
  let tokens = lexer.tokenize()?;
  let mut parser = Parser::new(tokens);
  parser.parse_grammar()
}

/// Parse a single CSV field, handling double-quote escaping.
/// Returns (field_value, rest_of_line).
fn parse_csv_field(input: &str) -> (String, &str) {
  let input = input.trim_start();
  if input.starts_with('"') {
    // Quoted field: find matching close quote (doubled quotes are escapes)
    let mut result = String::new();
    let mut chars = input[1..].chars().peekable();
    let mut byte_pos = 1; // start after opening quote
    loop {
      match chars.next() {
        Some('"') => {
          byte_pos += '"'.len_utf8();
          if chars.peek() == Some(&'"') {
            // Escaped quote
            result.push('"');
            chars.next();
            byte_pos += '"'.len_utf8();
          } else {
            // End of quoted field
            break;
          }
        }
        Some(c) => {
          result.push(c);
          byte_pos += c.len_utf8();
        }
        None => break,
      }
    }
    // Skip comma after closing quote
    let rest = &input[byte_pos..];
    let rest = if rest.starts_with(',') {
      &rest[1..]
    } else {
      rest
    };
    (result, rest)
  } else {
    // Unquoted field: read until comma or end
    match input.find(',') {
      Some(pos) => (input[..pos].to_string(), &input[pos + 1..]),
      None => (input.to_string(), ""),
    }
  }
}

/// Parse examples string: "ja:text1;zh:翻译|ja:text2"
fn parse_examples_str(s: &str) -> Vec<Example> {
  if s.is_empty() {
    return Vec::new();
  }
  s.split('|')
    .map(|example_group| {
      let parts: Vec<&str> = example_group.split(';').collect();
      let mut sentence = String::new();
      let mut translations = Vec::new();
      for part in parts {
        if let Some(colon_pos) = part.find(':') {
          let lang = &part[..colon_pos];
          let text = &part[colon_pos + 1..];
          if lang == "ja" && sentence.is_empty() {
            sentence = text.to_string();
          } else {
            translations.push((lang.to_string(), text.to_string()));
          }
        } else if sentence.is_empty() {
          sentence = part.to_string();
        }
      }
      Example {
        sentence,
        translations,
      }
    })
    .collect()
}

/// Parse CSV format grammar rules (from grammars.gz).
///
/// CSV format (UTF-8, with header):
/// rule_name,levels,name,description,connection,pattern,examples
pub fn parse_csv_grammar(csv_text: &str) -> Result<Grammar, String> {
  let mut rules = Vec::new();

  for (line_num, line) in csv_text.lines().enumerate() {
    // Skip header and empty lines
    if line_num == 0 || line.trim().is_empty() {
      continue;
    }

    // Parse 7 CSV fields
    let (rule_name, rest) = parse_csv_field(line);
    let (levels_str, rest) = parse_csv_field(rest);
    let (name, rest) = parse_csv_field(rest);
    let (description, rest) = parse_csv_field(rest);
    let (connection, rest) = parse_csv_field(rest);
    let (pattern_str, rest) = parse_csv_field(rest);
    let (examples_str, _) = parse_csv_field(rest);

    let levels: Vec<String> = if levels_str.is_empty() {
      Vec::new()
    } else {
      levels_str
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
    };

    let desc = if name.is_empty() && description.is_empty() {
      None
    } else if description.is_empty() {
      Some(name.clone())
    } else {
      Some(format!("{}：{}", name, description))
    };

    let examples = parse_examples_str(&examples_str);

    let metadata = Some(RuleMetadata {
      levels,
      description: desc,
      connection: if connection.is_empty() {
        None
      } else {
        Some(connection)
      },
      examples,
    });

    // Parse the EBNF pattern if present, otherwise create a dummy pattern
    let pattern = if pattern_str.trim().is_empty() {
      // No pattern: create a rule that never matches (empty sequence)
      PatternExpr::Sequence(Vec::new())
    } else {
      // Parse the EBNF pattern expression
      let pattern_grammar = format!("{} = {} ;", rule_name, pattern_str);
      match parse_grammar(&pattern_grammar) {
        Ok(g) => {
          if let Some(r) = g.rules.into_iter().next() {
            r.pattern
          } else {
            PatternExpr::Sequence(Vec::new())
          }
        }
        Err(e) => {
          // Log parse error but continue with other rules
          eprintln!(
            "Warning: failed to parse pattern for rule '{}': {}",
            rule_name, e
          );
          PatternExpr::Sequence(Vec::new())
        }
      }
    };

    rules.push(Rule {
      name: rule_name,
      pattern,
      metadata,
    });
  }

  Ok(Grammar { rules })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_simple_rule() {
    let grammar = parse_grammar(r#"te_form = 動詞 助詞.接続助詞"て" ;"#).unwrap();
    assert_eq!(grammar.rules.len(), 1);
    assert_eq!(grammar.rules[0].name, "te_form");
  }

  #[test]
  fn test_metadata() {
    let grammar = parse_grammar(
      r#"
            [N5 N4, "concession"]
            concession = "いくら" _* 動詞 ;
        "#,
    )
    .unwrap();
    assert_eq!(grammar.rules.len(), 1);
    let meta = grammar.rules[0].metadata.as_ref().unwrap();
    assert_eq!(meta.levels, vec!["N5", "N4"]);
    assert_eq!(meta.description.as_deref(), Some("concession"));
  }

  #[test]
  fn test_alternatives() {
    let grammar = parse_grammar(r#"pos = 名詞 | 動詞 | 形容詞 ;"#).unwrap();
    match &grammar.rules[0].pattern {
      PatternExpr::Alternative(alts) => assert_eq!(alts.len(), 3),
      _ => panic!("Expected Alternative"),
    }
  }

  #[test]
  fn test_quantifiers() {
    let grammar = parse_grammar(r#"rep = 名詞+ 助詞? _* ;"#).unwrap();
    match &grammar.rules[0].pattern {
      PatternExpr::Sequence(items) => {
        assert!(matches!(items[0], PatternExpr::OneOrMore(_)));
        assert!(matches!(items[1], PatternExpr::Optional(_)));
        assert!(matches!(items[2], PatternExpr::ZeroOrMore(_)));
      }
      _ => panic!("Expected Sequence"),
    }
  }

  #[test]
  fn test_grouping() {
    let grammar = parse_grammar(r#"chain = 動詞 (助詞.接続助詞"て" 動詞)+ ;"#).unwrap();
    match &grammar.rules[0].pattern {
      PatternExpr::Sequence(items) => {
        assert!(matches!(items[0], PatternExpr::Token(_)));
        assert!(matches!(items[1], PatternExpr::OneOrMore(_)));
      }
      _ => panic!("Expected Sequence"),
    }
  }

  #[test]
  fn test_comments() {
    let grammar = parse_grammar(
      r#"
            // This is a comment
            rule1 = 名詞 ; // inline comment
            rule2 = 動詞 ;
        "#,
    )
    .unwrap();
    assert_eq!(grammar.rules.len(), 2);
  }

  #[test]
  fn test_base_form() {
    let grammar = parse_grammar(r#"suru = 動詞@"する" ;"#).unwrap();
    match &grammar.rules[0].pattern {
      PatternExpr::Token(pred) => {
        assert_eq!(pred.pos, vec!["動詞"]);
        assert!(matches!(&pred.base_form, Some(StringMatcher::Exact(s)) if s == "する"));
      }
      _ => panic!("Expected Token"),
    }
  }

  #[test]
  fn test_surface_only() {
    let grammar = parse_grammar(r#"ikura = "いくら" ;"#).unwrap();
    match &grammar.rules[0].pattern {
      PatternExpr::Token(pred) => {
        assert!(pred.pos.is_empty());
        assert!(matches!(&pred.surface, Some(StringMatcher::Exact(s)) if s == "いくら"));
      }
      _ => panic!("Expected Token"),
    }
  }

  #[test]
  fn test_rule_ref() {
    let grammar = parse_grammar(
      r#"
            noun_part = 名詞+ ;
            sentence = noun_part 動詞 ;
        "#,
    )
    .unwrap();
    assert_eq!(grammar.rules.len(), 2);
    match &grammar.rules[1].pattern {
      PatternExpr::Sequence(items) => {
        assert!(matches!(&items[0], PatternExpr::RuleRef(name) if name == "noun_part"));
      }
      _ => panic!("Expected Sequence"),
    }
  }

  #[test]
  fn test_multiple_rules_with_metadata() {
    let grammar = parse_grammar(
      r#"
            [N5, "te-form"]
            te_form = 動詞 助詞.接続助詞"て" ;

            [N3 N2, "causative passive"]
            causative_passive = 動詞 助動詞 ;
        "#,
    )
    .unwrap();
    assert_eq!(grammar.rules.len(), 2);
    let m0 = grammar.rules[0].metadata.as_ref().unwrap();
    assert_eq!(m0.levels, vec!["N5"]);
    let m1 = grammar.rules[1].metadata.as_ref().unwrap();
    assert_eq!(m1.levels, vec!["N3", "N2"]);
  }

  #[test]
  fn test_parse_csv_grammar_basic() {
    let csv = "rule_name,levels,name,description,connection,pattern,examples\n\
                   te_form,N5,て形,動作の接続,動詞て形,\"動詞 助詞.接続助詞\"\"て\"\"\",ja:食べて寝る|ja:走って帰る\n";
    let grammar = parse_csv_grammar(csv).unwrap();
    assert_eq!(grammar.rules.len(), 1);
    assert_eq!(grammar.rules[0].name, "te_form");
    let meta = grammar.rules[0].metadata.as_ref().unwrap();
    assert_eq!(meta.levels, vec!["N5"]);
    assert_eq!(meta.connection.as_deref(), Some("動詞て形"));
    assert_eq!(meta.examples.len(), 2);
    assert_eq!(meta.examples[0].sentence, "食べて寝る");
    assert_eq!(meta.examples[1].sentence, "走って帰る");
  }

  #[test]
  fn test_parse_csv_grammar_with_translations() {
    let csv = "rule_name,levels,name,description,connection,pattern,examples\n\
                   test,N4,テスト,テスト説明,,\"\",ja:食べる;zh:吃;en:eat\n";
    let grammar = parse_csv_grammar(csv).unwrap();
    let meta = grammar.rules[0].metadata.as_ref().unwrap();
    assert_eq!(meta.examples.len(), 1);
    assert_eq!(meta.examples[0].sentence, "食べる");
    assert_eq!(meta.examples[0].translations.len(), 2);
    assert_eq!(meta.examples[0].translations[0].0, "zh");
    assert_eq!(meta.examples[0].translations[0].1, "吃");
    assert_eq!(meta.examples[0].translations[1].0, "en");
    assert_eq!(meta.examples[0].translations[1].1, "eat");
  }

  #[test]
  fn test_parse_csv_grammar_empty_pattern() {
    let csv = "rule_name,levels,name,description,connection,pattern,examples\n\
                   no_pattern,N3,テスト,説明,接続,,ja:例文\n";
    let grammar = parse_csv_grammar(csv).unwrap();
    assert_eq!(grammar.rules.len(), 1);
    assert_eq!(grammar.rules[0].name, "no_pattern");
  }

  #[test]
  fn test_parse_csv_grammar_multiple_levels() {
    let csv = "rule_name,levels,name,description,connection,pattern,examples\n\
                   multi,N5 N4 N3,テスト,説明,,,\n";
    let grammar = parse_csv_grammar(csv).unwrap();
    let meta = grammar.rules[0].metadata.as_ref().unwrap();
    assert_eq!(meta.levels, vec!["N5", "N4", "N3"]);
  }

  #[test]
  fn test_suffix_surface() {
    let grammar = parse_grammar(r#"rule = 動詞~"る" ;"#).unwrap();
    match &grammar.rules[0].pattern {
      PatternExpr::Token(pred) => {
        assert_eq!(pred.pos, vec!["動詞"]);
        assert!(matches!(&pred.surface, Some(StringMatcher::Suffix(s)) if s == "る"));
      }
      _ => panic!("Expected Token"),
    }
  }

  #[test]
  fn test_suffix_base_form() {
    let grammar = parse_grammar(r#"rule = 動詞@~"上がる" ;"#).unwrap();
    match &grammar.rules[0].pattern {
      PatternExpr::Token(pred) => {
        assert_eq!(pred.pos, vec!["動詞"]);
        assert!(matches!(&pred.base_form, Some(StringMatcher::Suffix(s)) if s == "上がる"));
      }
      _ => panic!("Expected Token"),
    }
  }

  #[test]
  fn test_regex_base_form() {
    let grammar = parse_grammar(r#"rule = 動詞@/す[るれ]/ ;"#).unwrap();
    match &grammar.rules[0].pattern {
      PatternExpr::Token(pred) => {
        assert_eq!(pred.pos, vec!["動詞"]);
        assert!(matches!(&pred.base_form, Some(StringMatcher::Regex(_))));
      }
      _ => panic!("Expected Token"),
    }
  }

  #[test]
  fn test_suffix_conjugation_form() {
    let grammar = parse_grammar(r#"rule = 動詞[~"接続"] ;"#).unwrap();
    match &grammar.rules[0].pattern {
      PatternExpr::Token(pred) => {
        assert!(matches!(&pred.conjugation_form, Some(StringMatcher::Suffix(s)) if s == "接続"));
      }
      _ => panic!("Expected Token"),
    }
  }

  #[test]
  fn test_regex_surface() {
    let grammar = parse_grammar(r#"rule = /^食べ/ ;"#).unwrap();
    match &grammar.rules[0].pattern {
      PatternExpr::Token(pred) => {
        assert!(pred.pos.is_empty());
        assert!(matches!(&pred.surface, Some(StringMatcher::Regex(_))));
      }
      _ => panic!("Expected Token"),
    }
  }
}

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
use super::lexer::{is_rule_name, Lexer, Token};
use super::syntax::*;

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
          let re = fancy_regex::Regex::new(&s.clone())
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

    let uses_captures = pattern_uses_captures(&pattern);
    Ok(Rule {
      name,
      pattern,
      metadata,
      uses_captures,
      max_bunsetsu_span: 0,
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
        // Check for _@=$N (wildcard with back-reference)
        if self.peek() == Some(&Token::At)
          && self.pos + 1 < self.tokens.len()
          && self.tokens[self.pos + 1] == Token::Equals
        {
          self.next(); // consume @
          self.next(); // consume =
          match self.next() {
            Some(Token::CaptureSlot(n)) => {
              let n = *n;
              return Ok(PatternExpr::Token(Box::new(TokenPredicate {
                pos: vec![],
                surface: None,
                base_form: None,
                conjugation_form: None,
                conjugation_type: None,
                capture: None,
                base_form_ref: Some(n),
              })));
            }
            other => {
              return Err(format!(
                "Expected capture slot after '_@=', got {:?}",
                other
              ))
            }
          }
        }
        Ok(PatternExpr::Wildcard)
      }
      Some(Token::StringLit(_))
      | Some(Token::Tilde)
      | Some(Token::RegexLit(_))
      | Some(Token::At) => {
        // Surface-only or base-form-only token matcher, or @=$N back-reference
        // Check for @=$N (base_form back-reference on bare token)
        if self.peek() == Some(&Token::At) {
          // Peek further to see if it's @= (back-ref) or @"..." (base_form)
          if self.pos + 1 < self.tokens.len() && self.tokens[self.pos + 1] == Token::Equals {
            // @=$N on bare token (no POS)
            self.next(); // consume @
            self.next(); // consume =
            match self.next() {
              Some(Token::CaptureSlot(n)) => {
                let n = *n;
                return Ok(PatternExpr::Token(Box::new(TokenPredicate {
                  pos: vec![],
                  surface: None,
                  base_form: None,
                  conjugation_form: None,
                  conjugation_type: None,
                  capture: None,
                  base_form_ref: Some(n),
                })));
              }
              other => return Err(format!("Expected capture slot after '@=', got {:?}", other)),
            }
          }
        }
        Ok(PatternExpr::Token(Box::new(
          self.parse_token_predicate(None)?,
        )))
      }
      Some(Token::Ident(_)) => {
        // Could be: POS path (token matcher), rule reference, or _$N wildcard capture.
        let ident = match self.next() {
          Some(Token::Ident(s)) => s.clone(),
          _ => unreachable!(),
        };

        // Handle _$N wildcard capture sentinel (produced by lexer)
        if let Some(suffix) = ident.strip_prefix("_$") {
          if let Ok(n) = suffix.parse::<u8>() {
            return Ok(PatternExpr::WildcardCapture(n));
          }
        }

        // Check if this is a POS path (may have dots)
        let mut pos_parts = vec![ident.clone()];
        while self.peek() == Some(&Token::Dot) {
          self.next();
          match self.next() {
            Some(Token::Ident(s)) => pos_parts.push(s.clone()),
            other => return Err(format!("Expected identifier after '.', got {:?}", other)),
          }
        }

        // Parse optional capture slot: POS$N
        let mut capture = None;
        if let Some(Token::CaptureSlot(n)) = self.peek() {
          capture = Some(*n);
          self.next();
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
        // When conjugation_form or conjugation_type is set, a plain string literal
        // starts a new atom in the sequence
        // (e.g. 形容詞[ガル接続] "さ" = two separate tokens).
        // However, @"..." (base_form) is still allowed on the same atom
        // (e.g. 形容詞[連用テ接続]@"ない" = single token with conjugation + base match).
        let mut surface = None;
        let mut base_form = None;
        let mut base_form_ref = None;

        if conjugation_form.is_none()
          && conjugation_type.is_none()
          && matches!(
            self.peek(),
            Some(Token::StringLit(_)) | Some(Token::Tilde) | Some(Token::RegexLit(_))
          )
        {
          surface = Some(self.parse_string_value()?);
        }

        // Base form: @"..." (static) or @=$N (back-reference)
        if self.peek() == Some(&Token::At) {
          self.next();
          if self.peek() == Some(&Token::Equals) {
            // @=$N back-reference
            self.next(); // consume =
            match self.next() {
              Some(Token::CaptureSlot(n)) => {
                base_form_ref = Some(*n);
              }
              other => return Err(format!("Expected capture slot after '@=', got {:?}", other)),
            }
          } else {
            base_form = Some(self.parse_string_value()?);
          }
        }

        // If it's a bare identifier with no dots, no surface, no base_form,
        // no conjugation_form/type, no capture, no base_form_ref, and looks like
        // a rule reference (ASCII letters + underscores), treat it as a rule ref.
        if pos_parts.len() == 1
          && surface.is_none()
          && base_form.is_none()
          && conjugation_form.is_none()
          && conjugation_type.is_none()
          && capture.is_none()
          && base_form_ref.is_none()
          && is_rule_name(&pos_parts[0])
        {
          Ok(PatternExpr::RuleRef(pos_parts.pop().unwrap()))
        } else {
          Ok(PatternExpr::Token(Box::new(TokenPredicate {
            pos: pos_parts,
            surface,
            base_form,
            conjugation_form,
            conjugation_type,
            capture,
            base_form_ref,
          })))
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
      capture: None,
      base_form_ref: None,
    })
  }
}

/// Parse a grammar text string into a Grammar AST.
pub fn parse_grammar(input: &str) -> Result<Grammar, String> {
  let mut lexer = Lexer::new(input);
  let tokens = lexer.tokenize()?;
  let mut parser = Parser::new(tokens);
  parser.parse_grammar()
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
  fn test_conjugation_form_with_base_form() {
    // Conjugation form + base form on the same atom should work
    let grammar = parse_grammar(r#"rule = 形容詞[連用テ接続]@"ない" ;"#).unwrap();
    match &grammar.rules[0].pattern {
      PatternExpr::Token(pred) => {
        assert_eq!(pred.pos, vec!["形容詞"]);
        assert!(
          matches!(&pred.conjugation_form, Some(StringMatcher::Exact(s)) if s == "連用テ接続")
        );
        assert!(matches!(&pred.base_form, Some(StringMatcher::Exact(s)) if s == "ない"));
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

/// AST types for the EBNF grammar pattern matcher.
///
/// The grammar operates on a stream of MecabNode tokens. Each terminal
/// matches a single token by its POS hierarchy, surface form, or base form.
use fancy_regex::Regex;
use std::collections::HashSet;

/// String matching strategy for token fields.
#[derive(Debug, Clone)]
pub enum StringMatcher {
  /// Exact equality: `"text"` or `@"text"` or `[form]`
  Exact(String),
  /// Suffix match: `~"suffix"` or `@~"suffix"` or `[~"suffix"]`
  Suffix(String),
  /// Regex match: `/pattern/` or `@/pattern/` or `[/pattern/]`
  Regex(Regex),
}

impl StringMatcher {
  /// Test whether the given string matches this matcher.
  pub fn matches(&self, value: &str) -> bool {
    match self {
      StringMatcher::Exact(s) => value == s,
      StringMatcher::Suffix(s) => value.ends_with(s.as_str()),
      StringMatcher::Regex(r) => r.is_match(value).unwrap_or(false),
    }
  }
}

/// Constraint on a single MeCab token.
#[derive(Debug, Clone)]
pub struct TokenPredicate {
  /// POS hierarchy: e.g. ["動詞"] or ["助詞", "接続助詞"]
  pub pos: Vec<String>,
  /// Surface form constraint
  pub surface: Option<StringMatcher>,
  /// Base form constraint (原形, feature index 6)
  pub base_form: Option<StringMatcher>,
  /// Conjugation form constraint (活用形, feature index 5)
  pub conjugation_form: Option<StringMatcher>,
  /// Conjugation type constraint (活用型, feature index 4)
  pub conjugation_type: Option<StringMatcher>,
  /// Capture slot: store this token's base_form into capture $N
  pub capture: Option<u8>,
  /// Back-reference: require base_form == captured value in slot $N
  pub base_form_ref: Option<u8>,
}

/// A node in the pattern AST.
#[derive(Debug, Clone)]
pub enum PatternExpr {
  /// Match a single token against a predicate
  Token(Box<TokenPredicate>),
  /// Wildcard: match any single token
  Wildcard,
  /// Wildcard with capture: match any token and capture its base_form into slot $N
  WildcardCapture(u8),
  /// Sequence of patterns (A B C)
  Sequence(Vec<PatternExpr>),
  /// Alternative (A | B)
  Alternative(Vec<PatternExpr>),
  /// Optional (A?)
  Optional(Box<PatternExpr>),
  /// Zero or more (A*)
  ZeroOrMore(Box<PatternExpr>),
  /// One or more (A+)
  OneOrMore(Box<PatternExpr>),
  /// Named rule reference
  RuleRef(String),
}

/// A single example sentence with optional translations.
#[derive(Debug, Clone)]
pub struct Example {
  /// Original sentence (typically Japanese)
  pub sentence: String,
  /// Translations: [(lang_code, text), ...] e.g. [("zh", "无论怎么吵也没关系")]
  pub translations: Vec<(String, String)>,
}

/// Metadata attached to a grammar rule.
#[derive(Debug, Clone)]
pub struct RuleMetadata {
  /// JLPT levels this rule belongs to, e.g. ["N5", "N4"]
  pub levels: Vec<String>,
  /// Human-readable description
  pub description: Option<String>,
  /// Connection pattern (接続方式), e.g. "動詞て形＋も"
  pub connection: Option<String>,
  /// Example sentences with translations
  pub examples: Vec<Example>,
}

/// A named grammar rule.
#[derive(Debug, Clone)]
pub struct Rule {
  pub name: String,
  pub pattern: PatternExpr,
  pub metadata: Option<RuleMetadata>,
  /// Whether this rule uses capture back-references ($N / @=$N).
  pub uses_captures: bool,
}

/// Recursively check whether a pattern expression uses captures or back-references.
pub fn pattern_uses_captures(expr: &PatternExpr) -> bool {
  match expr {
    PatternExpr::Token(pred) => pred.capture.is_some() || pred.base_form_ref.is_some(),
    PatternExpr::WildcardCapture(_) => true,
    PatternExpr::Wildcard => false,
    PatternExpr::Sequence(items) => items.iter().any(pattern_uses_captures),
    PatternExpr::Alternative(alts) => alts.iter().any(pattern_uses_captures),
    PatternExpr::Optional(inner)
    | PatternExpr::ZeroOrMore(inner)
    | PatternExpr::OneOrMore(inner) => pattern_uses_captures(inner),
    PatternExpr::RuleRef(_) => false,
  }
}

/// A complete grammar (set of rules).
#[derive(Debug, Clone)]
pub struct Grammar {
  pub rules: Vec<Rule>,
}

impl Grammar {
  pub fn new() -> Self {
    Grammar { rules: Vec::new() }
  }

  /// Merge another grammar's rules into this one, skipping duplicate names.
  pub fn merge(&mut self, other: Grammar) {
    for rule in other.rules {
      if !self.rules.iter().any(|r| r.name == rule.name) {
        self.rules.push(rule);
      }
    }
  }

  /// Find a rule by name.
  pub fn find_rule(&self, name: &str) -> Option<&Rule> {
    self.rules.iter().find(|r| r.name == name)
  }

  /// Get all rule names.
  pub fn rule_names(&self) -> Vec<String> {
    self.rules.iter().map(|r| r.name.clone()).collect()
  }

  /// Check whether a rule (transitively through RuleRefs) uses captures.
  pub fn rule_uses_captures(&self, rule_name: &str) -> bool {
    let mut visited = HashSet::new();
    self.rule_uses_captures_inner(rule_name, &mut visited)
  }

  fn rule_uses_captures_inner(&self, rule_name: &str, visited: &mut HashSet<String>) -> bool {
    if !visited.insert(rule_name.to_string()) {
      return false; // cycle guard
    }
    if let Some(rule) = self.find_rule(rule_name) {
      self.expr_uses_captures(&rule.pattern, visited)
    } else {
      false
    }
  }

  fn expr_uses_captures(&self, expr: &PatternExpr, visited: &mut HashSet<String>) -> bool {
    match expr {
      PatternExpr::Token(pred) => pred.capture.is_some() || pred.base_form_ref.is_some(),
      PatternExpr::WildcardCapture(_) => true,
      PatternExpr::Wildcard => false,
      PatternExpr::Sequence(items) => items.iter().any(|e| self.expr_uses_captures(e, visited)),
      PatternExpr::Alternative(alts) => alts.iter().any(|e| self.expr_uses_captures(e, visited)),
      PatternExpr::Optional(inner)
      | PatternExpr::ZeroOrMore(inner)
      | PatternExpr::OneOrMore(inner) => self.expr_uses_captures(inner, visited),
      PatternExpr::RuleRef(name) => self.rule_uses_captures_inner(name, visited),
    }
  }
}

/// AST types for the EBNF grammar pattern matcher.
///
/// The grammar operates on a stream of MecabNode tokens. Each terminal
/// matches a single token by its POS hierarchy, surface form, or base form.

/// Constraint on a single MeCab token.
#[derive(Debug, Clone)]
pub struct TokenPredicate {
  /// POS hierarchy: e.g. ["動詞"] or ["助詞", "接続助詞"]
  pub pos: Vec<String>,
  /// Surface form constraint (exact match)
  pub surface: Option<String>,
  /// Base form constraint (原形, feature index 6)
  pub base_form: Option<String>,
  /// Conjugation form constraint (活用形, feature index 5)
  pub conjugation_form: Option<String>,
}

/// A node in the pattern AST.
#[derive(Debug, Clone)]
pub enum PatternExpr {
  /// Match a single token against a predicate
  Token(TokenPredicate),
  /// Wildcard: match any single token
  Wildcard,
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
}

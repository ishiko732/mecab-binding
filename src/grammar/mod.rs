mod matcher;
mod parser;
mod syntax;

use crate::node::MecabNode;
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// A translation of an example sentence.
#[napi(object)]
#[derive(Clone)]
pub struct ExampleTranslation {
  /// Language code: "zh", "en", "ko", etc.
  pub lang: String,
  /// Translated text
  pub text: String,
}

/// An example sentence with optional translations.
#[napi(object)]
#[derive(Clone)]
pub struct GrammarExample {
  /// Original sentence (typically Japanese)
  pub sentence: String,
  /// Translations in other languages
  pub translations: Vec<ExampleTranslation>,
}

/// A match found by the grammar matcher.
#[napi(object)]
#[derive(Clone)]
pub struct GrammarMatch {
  /// Name of the rule that matched
  pub rule: String,
  /// Start index in the node array (inclusive)
  pub start: u32,
  /// End index in the node array (exclusive)
  pub end: u32,
  /// The matched nodes
  pub nodes: Vec<MecabNode>,
  /// Indices of nodes matched by fixed (non-wildcard) pattern elements.
  /// Only these nodes should be highlighted as part of the grammar pattern.
  pub fixed_indices: Vec<u32>,
  /// JLPT levels from rule metadata
  pub levels: Vec<String>,
  /// Description from rule metadata
  pub description: Option<String>,
  /// Connection pattern (接続方式)
  pub connection: Option<String>,
  /// Example sentences
  pub examples: Vec<GrammarExample>,
}

/// EBNF-style grammar pattern matcher for MeCab token streams.
///
/// Parse grammar rules once, then match against multiple inputs.
#[napi]
pub struct GrammarMatcher {
  grammar: syntax::Grammar,
}

#[napi]
impl GrammarMatcher {
  /// Create a matcher from inline grammar text.
  #[napi(constructor)]
  pub fn new(grammar_text: String) -> Result<Self> {
    let grammar = parser::parse_grammar(&grammar_text)
      .map_err(|e| Error::from_reason(format!("Grammar parse error: {}", e)))?;
    Ok(GrammarMatcher { grammar })
  }

  /// Create a matcher from a .grammar file path.
  #[napi(factory)]
  pub fn from_file(path: String) -> Result<Self> {
    let text = std::fs::read_to_string(&path)
      .map_err(|e| Error::from_reason(format!("Failed to read grammar file '{}': {}", path, e)))?;
    let grammar = parser::parse_grammar(&text)
      .map_err(|e| Error::from_reason(format!("Grammar parse error in '{}': {}", path, e)))?;
    Ok(GrammarMatcher { grammar })
  }

  /// Create a matcher from gzip-compressed CSV data.
  #[napi(factory)]
  pub fn from_gz(data: &[u8]) -> Result<Self> {
    use flate2::read::GzDecoder;
    use std::io::Read;

    let mut decoder = GzDecoder::new(data);
    let mut csv_text = String::new();
    decoder
      .read_to_string(&mut csv_text)
      .map_err(|e| Error::from_reason(format!("Failed to decompress gz data: {}", e)))?;

    let grammar = parser::parse_csv_grammar(&csv_text)
      .map_err(|e| Error::from_reason(format!("CSV grammar parse error: {}", e)))?;
    Ok(GrammarMatcher { grammar })
  }

  /// Find all matches of a specific rule in the given nodes.
  #[napi]
  pub fn find(&self, rule_name: String, nodes: Vec<MecabNode>) -> Result<Vec<GrammarMatch>> {
    let results = matcher::find_matches(&self.grammar, &rule_name, &nodes);
    Ok(
      results
        .into_iter()
        .map(|m| to_grammar_match(m, &nodes))
        .collect(),
    )
  }

  /// Find all matches of ALL rules in the given nodes.
  #[napi]
  pub fn find_all(&self, nodes: Vec<MecabNode>) -> Result<Vec<GrammarMatch>> {
    let results = matcher::find_all_matches(&self.grammar, &nodes);
    Ok(
      results
        .into_iter()
        .map(|m| to_grammar_match(m, &nodes))
        .collect(),
    )
  }

  /// Test if a specific rule matches anywhere in the nodes.
  #[napi]
  pub fn test(&self, rule_name: String, nodes: Vec<MecabNode>) -> Result<bool> {
    let results = matcher::find_matches(&self.grammar, &rule_name, &nodes);
    Ok(!results.is_empty())
  }

  /// List all rule names in this grammar.
  #[napi]
  pub fn rule_names(&self) -> Vec<String> {
    self.grammar.rule_names()
  }

  /// Clone this matcher.
  #[napi]
  pub fn clone_matcher(&self) -> GrammarMatcher {
    GrammarMatcher {
      grammar: self.grammar.clone(),
    }
  }

  /// Merge another grammar text into this matcher.
  #[napi]
  pub fn merge(&mut self, grammar_text: String) -> Result<()> {
    let other = parser::parse_grammar(&grammar_text)
      .map_err(|e| Error::from_reason(format!("Grammar parse error: {}", e)))?;
    self.grammar.merge(other);
    Ok(())
  }
}

fn to_grammar_match(m: matcher::MatchResult, nodes: &[MecabNode]) -> GrammarMatch {
  GrammarMatch {
    rule: m.rule_name,
    start: m.start as u32,
    end: m.end as u32,
    nodes: nodes[m.start..m.end].to_vec(),
    fixed_indices: m.fixed_indices.iter().map(|&i| i as u32).collect(),
    levels: m.levels,
    description: m.description,
    connection: m.connection,
    examples: m
      .examples
      .into_iter()
      .map(|ex| GrammarExample {
        sentence: ex.sentence,
        translations: ex
          .translations
          .into_iter()
          .map(|(lang, text)| ExampleTranslation { lang, text })
          .collect(),
      })
      .collect(),
  }
}

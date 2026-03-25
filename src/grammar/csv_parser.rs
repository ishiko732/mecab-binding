/// CSV format parser for grammar rules (from grammars.gz).
///
/// Parses CSV with fields: rule_name,levels,name,description,connection,pattern,examples
use serde::Deserialize;

use super::syntax::*;

/// A single row in the grammar CSV file.
#[derive(Deserialize)]
struct GrammarItem {
  rule_name: String,
  #[serde(default)]
  levels: String,
  #[serde(default)]
  name: String,
  #[serde(default)]
  description: String,
  #[serde(default)]
  connection: String,
  #[serde(default)]
  pattern: String,
  #[serde(default)]
  examples: String,
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

  let mut rdr = csv::ReaderBuilder::new()
    .has_headers(true)
    .flexible(true)
    .from_reader(csv_text.as_bytes());

  for result in rdr.deserialize::<GrammarItem>() {
    let row = result.map_err(|e| format!("CSV parse error: {}", e))?;

    if row.rule_name.is_empty() {
      continue;
    }

    let levels: Vec<String> = if row.levels.is_empty() {
      Vec::new()
    } else {
      row.levels.split_whitespace().map(|s| s.to_string()).collect()
    };

    let desc = if row.name.is_empty() && row.description.is_empty() {
      None
    } else if row.description.is_empty() {
      Some(row.name.clone())
    } else {
      Some(format!("{}：{}", row.name, row.description))
    };

    let examples = parse_examples_str(&row.examples);

    let metadata = Some(RuleMetadata {
      levels,
      description: desc,
      connection: if row.connection.is_empty() {
        None
      } else {
        Some(row.connection)
      },
      examples,
    });

    // Parse the EBNF pattern if present, otherwise create a dummy pattern
    let pattern = if row.pattern.trim().is_empty() {
      PatternExpr::Sequence(Vec::new())
    } else {
      let pattern_grammar = format!("{} = {} ;", row.rule_name, row.pattern);
      match super::parser::parse_grammar(&pattern_grammar) {
        Ok(g) => {
          if let Some(r) = g.rules.into_iter().next() {
            r.pattern
          } else {
            PatternExpr::Sequence(Vec::new())
          }
        }
        Err(e) => {
          eprintln!(
            "Warning: failed to parse pattern for rule '{}': {}",
            row.rule_name, e
          );
          PatternExpr::Sequence(Vec::new())
        }
      }
    };

    let uses_captures = pattern_uses_captures(&pattern);
    rules.push(Rule {
      name: row.rule_name,
      pattern,
      metadata,
      uses_captures,
      max_bunsetsu_span: 0,
    });
  }

  Ok(Grammar { rules })
}
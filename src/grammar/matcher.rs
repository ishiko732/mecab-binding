/// Pattern matching engine that matches grammar rules against MecabNode sequences.
///
/// Uses backtracking with memoization. Wildcard `_*` is non-greedy by default.
use std::collections::HashMap;

use crate::node::MecabNode;

use super::syntax::*;
use super::token::*;

/// Try to match `pattern` starting at `nodes[pos]`.
/// Returns a sorted Vec of possible end positions (ascending = non-greedy first).
fn match_at(
  grammar: &Grammar,
  pattern: &PatternExpr,
  nodes: &[MecabNode],
  pos: usize,
  memo: &mut HashMap<(usize, usize), Vec<usize>>,
) -> Vec<usize> {
  // Use pattern address as key (patterns are in a stable allocation)
  let key = (pos, pattern as *const PatternExpr as usize);
  if let Some(cached) = memo.get(&key) {
    return cached.clone();
  }

  let result = match pattern {
    PatternExpr::Token(pred) => {
      if pos < nodes.len() && token_matches(&nodes[pos], pred) {
        vec![pos + 1]
      } else {
        vec![]
      }
    }

    PatternExpr::Wildcard | PatternExpr::WildcardCapture(_) => {
      if pos < nodes.len() {
        vec![pos + 1]
      } else {
        vec![]
      }
    }

    PatternExpr::Sequence(items) => {
      let mut current_positions = vec![pos];
      for item in items {
        let mut next_positions = Vec::new();
        for &p in &current_positions {
          let ends = match_at(grammar, item, nodes, p, memo);
          for end in ends {
            if !next_positions.contains(&end) {
              next_positions.push(end);
            }
          }
        }
        current_positions = next_positions;
        if current_positions.is_empty() {
          break;
        }
      }
      current_positions.sort_unstable();
      current_positions.dedup();
      current_positions
    }

    PatternExpr::Alternative(alts) => {
      let mut results = Vec::new();
      for alt in alts {
        let ends = match_at(grammar, alt, nodes, pos, memo);
        for end in ends {
          if !results.contains(&end) {
            results.push(end);
          }
        }
      }
      results.sort_unstable();
      results
    }

    PatternExpr::Optional(inner) => {
      // Non-greedy: try zero first, then one
      let mut results = vec![pos];
      let ends = match_at(grammar, inner, nodes, pos, memo);
      for end in ends {
        if !results.contains(&end) {
          results.push(end);
        }
      }
      results
    }

    PatternExpr::ZeroOrMore(inner) => {
      // Non-greedy: collect all reachable positions iteratively
      let mut results = vec![pos];
      let mut frontier = vec![pos];
      let mut visited = vec![pos];

      while !frontier.is_empty() {
        let mut next_frontier = Vec::new();
        for &p in &frontier {
          let ends = match_at(grammar, inner, nodes, p, memo);
          for end in ends {
            if end > p && !visited.contains(&end) {
              visited.push(end);
              results.push(end);
              next_frontier.push(end);
            }
          }
        }
        frontier = next_frontier;
      }
      results.sort_unstable();
      results
    }

    PatternExpr::OneOrMore(inner) => {
      // Must match at least once
      let first_ends = match_at(grammar, inner, nodes, pos, memo);
      let mut results = Vec::new();
      let mut frontier = first_ends.clone();
      let mut visited: Vec<usize> = first_ends.clone();

      // Add initial matches
      results.extend(&first_ends);

      // Then try additional matches
      while !frontier.is_empty() {
        let mut next_frontier = Vec::new();
        for &p in &frontier {
          let ends = match_at(grammar, inner, nodes, p, memo);
          for end in ends {
            if end > p && !visited.contains(&end) {
              visited.push(end);
              results.push(end);
              next_frontier.push(end);
            }
          }
        }
        frontier = next_frontier;
      }
      results.sort_unstable();
      results.dedup();
      results
    }

    PatternExpr::RuleRef(name) => {
      if let Some(rule) = grammar.find_rule(name) {
        // Clone the pattern to avoid borrow issues
        let pattern = rule.pattern.clone();
        match_at(grammar, &pattern, nodes, pos, memo)
      } else {
        vec![] // Unknown rule reference
      }
    }
  };

  memo.insert(key, result.clone());
  result
}

// ── Capture-aware matching (no memoization) ──────────────────────────────

/// Capture-aware match_at. Returns Vec of (end_pos, captures) pairs.
/// Does NOT use memoization because capture state invalidates cache keys.
fn match_at_captures(
  grammar: &Grammar,
  pattern: &PatternExpr,
  nodes: &[MecabNode],
  pos: usize,
  captures: &CaptureMap,
) -> Vec<(usize, CaptureMap)> {
  match pattern {
    PatternExpr::Token(pred) => {
      if pos < nodes.len() {
        if let Some(new_caps) = token_matches_with_captures(&nodes[pos], pred, captures) {
          vec![(pos + 1, new_caps)]
        } else {
          vec![]
        }
      } else {
        vec![]
      }
    }

    PatternExpr::Wildcard => {
      if pos < nodes.len() {
        vec![(pos + 1, captures.clone())]
      } else {
        vec![]
      }
    }

    PatternExpr::WildcardCapture(slot) => {
      if pos < nodes.len() {
        let mut new_caps = captures.clone();
        new_caps.insert(*slot, base_form_from_parts(&parse_feature(&nodes[pos].feature)).to_string());
        vec![(pos + 1, new_caps)]
      } else {
        vec![]
      }
    }

    PatternExpr::Sequence(items) => {
      let mut current: Vec<(usize, CaptureMap)> = vec![(pos, captures.clone())];
      for item in items {
        let mut next = Vec::new();
        for (p, caps) in &current {
          let results = match_at_captures(grammar, item, nodes, *p, caps);
          for (end, new_caps) in results {
            // Dedup by (position, captures) — not position alone, because
            // different capture states may affect subsequent back-references.
            if !next.iter().any(|(e, c)| *e == end && *c == new_caps) {
              next.push((end, new_caps));
            }
          }
        }
        current = next;
        if current.is_empty() {
          break;
        }
      }
      current.sort_by_key(|(e, _)| *e);
      current
    }

    PatternExpr::Alternative(alts) => {
      let mut results = Vec::new();
      for alt in alts {
        let ends = match_at_captures(grammar, alt, nodes, pos, captures);
        for (end, caps) in ends {
          if !results.iter().any(|(e, c)| *e == end && *c == caps) {
            results.push((end, caps));
          }
        }
      }
      results.sort_by_key(|(e, _)| *e);
      results
    }

    PatternExpr::Optional(inner) => {
      let mut results = vec![(pos, captures.clone())];
      let ends = match_at_captures(grammar, inner, nodes, pos, captures);
      for (end, caps) in ends {
        if !results.iter().any(|(e, c)| *e == end && *c == caps) {
          results.push((end, caps));
        }
      }
      results
    }

    PatternExpr::ZeroOrMore(inner) => {
      let mut results = vec![(pos, captures.clone())];
      let mut frontier = vec![(pos, captures.clone())];
      let mut visited = vec![pos];

      while !frontier.is_empty() {
        let mut next_frontier = Vec::new();
        for (p, caps) in &frontier {
          let ends = match_at_captures(grammar, inner, nodes, *p, caps);
          for (end, new_caps) in ends {
            if end > *p && !visited.contains(&end) {
              visited.push(end);
              results.push((end, new_caps.clone()));
              next_frontier.push((end, new_caps));
            }
          }
        }
        frontier = next_frontier;
      }
      results.sort_by_key(|(e, _)| *e);
      results
    }

    PatternExpr::OneOrMore(inner) => {
      let first = match_at_captures(grammar, inner, nodes, pos, captures);
      let mut results = Vec::new();
      let mut frontier = first.clone();
      let mut visited: Vec<usize> = first.iter().map(|(e, _)| *e).collect();

      results.extend(first);

      while !frontier.is_empty() {
        let mut next_frontier = Vec::new();
        for (p, caps) in &frontier {
          let ends = match_at_captures(grammar, inner, nodes, *p, caps);
          for (end, new_caps) in ends {
            if end > *p && !visited.contains(&end) {
              visited.push(end);
              results.push((end, new_caps.clone()));
              next_frontier.push((end, new_caps));
            }
          }
        }
        frontier = next_frontier;
      }
      results.sort_by_key(|(e, _)| *e);
      results
    }

    PatternExpr::RuleRef(name) => {
      if let Some(rule) = grammar.find_rule(name) {
        let p = rule.pattern.clone();
        match_at_captures(grammar, &p, nodes, pos, captures)
      } else {
        vec![]
      }
    }
  }
}

/// Trace which node indices are matched by fixed (non-wildcard) patterns.
/// Returns Some(fixed_indices) if the pattern matches at `pos` ending at `end`,
/// or None if no match is possible.
fn trace_fixed(
  grammar: &Grammar,
  pattern: &PatternExpr,
  nodes: &[MecabNode],
  pos: usize,
  end: usize,
  memo: &mut HashMap<(usize, usize), Vec<usize>>,
) -> Option<Vec<usize>> {
  match pattern {
    PatternExpr::Token(pred) => {
      if pos < nodes.len() && pos + 1 == end && token_matches(&nodes[pos], pred) {
        Some(vec![pos])
      } else {
        None
      }
    }

    PatternExpr::Wildcard | PatternExpr::WildcardCapture(_) => {
      if pos < nodes.len() && pos + 1 == end {
        Some(vec![]) // Wildcard: NOT fixed
      } else {
        None
      }
    }

    PatternExpr::Sequence(items) => {
      // Try to find a valid assignment of positions for each item
      trace_sequence(grammar, items, nodes, pos, end, memo)
    }

    PatternExpr::Alternative(alts) => {
      for alt in alts {
        if let Some(fixed) = trace_fixed(grammar, alt, nodes, pos, end, memo) {
          return Some(fixed);
        }
      }
      None
    }

    PatternExpr::Optional(inner) => {
      if pos == end {
        Some(vec![])
      } else {
        trace_fixed(grammar, inner, nodes, pos, end, memo)
      }
    }

    PatternExpr::ZeroOrMore(inner) => {
      if pos == end {
        return Some(vec![]);
      }
      // Try splitting into one match of inner + recursive rest
      let ends = match_at(grammar, inner, nodes, pos, memo);
      for &mid in &ends {
        if mid > pos && mid <= end {
          if let Some(mut first) = trace_fixed(grammar, inner, nodes, pos, mid, memo) {
            if mid == end {
              return Some(first);
            }
            if let Some(rest) = trace_fixed(grammar, pattern, nodes, mid, end, memo) {
              first.extend(rest);
              return Some(first);
            }
          }
        }
      }
      None
    }

    PatternExpr::OneOrMore(inner) => {
      let ends = match_at(grammar, inner, nodes, pos, memo);
      for &mid in &ends {
        if mid > pos && mid <= end {
          if let Some(mut first) = trace_fixed(grammar, inner, nodes, pos, mid, memo) {
            if mid == end {
              return Some(first);
            }
            // Rest is zero-or-more of inner
            let zero_or_more = PatternExpr::ZeroOrMore(Box::new(inner.as_ref().clone()));
            if let Some(rest) = trace_fixed(grammar, &zero_or_more, nodes, mid, end, memo) {
              first.extend(rest);
              return Some(first);
            }
          }
        }
      }
      None
    }

    PatternExpr::RuleRef(name) => {
      if let Some(rule) = grammar.find_rule(name) {
        let p = rule.pattern.clone();
        trace_fixed(grammar, &p, nodes, pos, end, memo)
      } else {
        None
      }
    }
  }
}

/// Trace fixed indices for a sequence of patterns.
fn trace_sequence(
  grammar: &Grammar,
  items: &[PatternExpr],
  nodes: &[MecabNode],
  pos: usize,
  end: usize,
  memo: &mut HashMap<(usize, usize), Vec<usize>>,
) -> Option<Vec<usize>> {
  if items.is_empty() {
    return if pos == end { Some(vec![]) } else { None };
  }

  let first = &items[0];
  let rest = &items[1..];
  let ends = match_at(grammar, first, nodes, pos, memo);

  for &mid in &ends {
    if mid > end {
      continue;
    }
    if let Some(mut first_fixed) = trace_fixed(grammar, first, nodes, pos, mid, memo) {
      if rest.is_empty() && mid == end {
        return Some(first_fixed);
      }
      if let Some(rest_fixed) = trace_sequence(grammar, rest, nodes, mid, end, memo) {
        first_fixed.extend(rest_fixed);
        return Some(first_fixed);
      }
    }
  }
  None
}

/// A single match found in the token stream.
#[derive(Debug, Clone)]
pub struct MatchResult {
  pub rule_name: String,
  pub start: usize,
  pub end: usize,
  /// Indices of nodes matched by fixed (non-wildcard) pattern elements.
  pub fixed_indices: Vec<usize>,
  pub levels: Vec<String>,
  pub description: Option<String>,
  pub connection: Option<String>,
  pub examples: Vec<super::syntax::Example>,
}

/// Find all non-overlapping matches of a specific rule in the token stream.
pub fn find_matches(grammar: &Grammar, rule_name: &str, nodes: &[MecabNode]) -> Vec<MatchResult> {
  let rule = match grammar.find_rule(rule_name) {
    Some(r) => r,
    None => return vec![],
  };

  let (levels, description, connection, examples) = match &rule.metadata {
    Some(m) => (
      m.levels.clone(),
      m.description.clone(),
      m.connection.clone(),
      m.examples.clone(),
    ),
    None => (vec![], None, None, Vec::new()),
  };

  let pattern = rule.pattern.clone();
  // Use grammar-aware check that resolves RuleRef targets transitively
  let uses_captures = grammar.rule_uses_captures(rule_name);
  let mut matches = Vec::new();
  let mut skip_until = 0;

  for start in 0..nodes.len() {
    if start < skip_until {
      continue;
    }

    if uses_captures {
      // Capture-aware path: no memoization
      let empty_caps = CaptureMap::new();
      let results = match_at_captures(grammar, &pattern, nodes, start, &empty_caps);

      // Take the longest match
      if let Some((end, _caps)) = results.last() {
        let end = *end;
        if end > start {
          // trace_fixed still works (it ignores capture semantics; back-ref tokens
          // are still "fixed" since the capture match already confirmed them)
          let mut memo = HashMap::new();
          let fixed_indices =
            trace_fixed(grammar, &pattern, nodes, start, end, &mut memo).unwrap_or_default();
          matches.push(MatchResult {
            rule_name: rule_name.to_string(),
            start,
            end,
            fixed_indices,
            levels: levels.clone(),
            description: description.clone(),
            connection: connection.clone(),
            examples: examples.clone(),
          });
          skip_until = end;
        }
      }
    } else {
      // Fast path with memoization (unchanged)
      let mut memo = HashMap::new();
      let ends = match_at(grammar, &pattern, nodes, start, &mut memo);

      // Take the longest match (last end position)
      if let Some(&end) = ends.last() {
        if end > start {
          let fixed_indices =
            trace_fixed(grammar, &pattern, nodes, start, end, &mut memo).unwrap_or_default();
          matches.push(MatchResult {
            rule_name: rule_name.to_string(),
            start,
            end,
            fixed_indices,
            levels: levels.clone(),
            description: description.clone(),
            connection: connection.clone(),
            examples: examples.clone(),
          });
          skip_until = end;
        }
      }
    }
  }

  matches
}

/// Find all non-overlapping matches of ALL rules in the token stream.
pub fn find_all_matches(grammar: &Grammar, nodes: &[MecabNode]) -> Vec<MatchResult> {
  let mut all_matches = Vec::new();
  for rule in &grammar.rules {
    let matches = find_matches(grammar, &rule.name, nodes);
    all_matches.extend(matches);
  }
  // Sort by start position, then by longest span
  all_matches.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
  all_matches
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::grammar::parser::parse_grammar;

  fn make_node(surface: &str, feature: &str) -> MecabNode {
    MecabNode {
      surface: surface.to_string(),
      feature: feature.to_string(),
      id: 0,
      length: 0,
      rlength: 0,
      rc_attr: 0,
      lc_attr: 0,
      posid: 0,
      char_type: 0,
      stat: 0,
      isbest: true,
      alpha: 0.0,
      beta: 0.0,
      prob: 0.0,
      wcost: 0,
      cost: 0,
    }
  }

  #[test]
  fn test_simple_pos_match() {
    let grammar = parse_grammar(r#"verbs = 動詞 ;"#).unwrap();
    let nodes = vec![
      make_node("食べ", "動詞,自立,*,*,一段,連用形,食べる,タベ,タベ"),
      make_node("た", "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ"),
    ];
    let matches = find_matches(&grammar, "verbs", &nodes);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].start, 0);
    assert_eq!(matches[0].end, 1);
  }

  #[test]
  fn test_sequence_match() {
    let grammar = parse_grammar(r#"te_form = 動詞 助詞.接続助詞"て" ;"#).unwrap();
    let nodes = vec![
      make_node("食べ", "動詞,自立,*,*,一段,連用形,食べる,タベ,タベ"),
      make_node("て", "助詞,接続助詞,*,*,*,*,て,テ,テ"),
      make_node("いる", "動詞,非自立,*,*,一段,基本形,いる,イル,イル"),
    ];
    let matches = find_matches(&grammar, "te_form", &nodes);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].start, 0);
    assert_eq!(matches[0].end, 2);
  }

  #[test]
  fn test_wildcard_non_greedy() {
    let grammar = parse_grammar(r#"pattern = "いくら" _* 助詞.係助詞"も" ;"#).unwrap();
    let nodes = vec![
      make_node("いくら", "副詞,一般,*,*,*,*,いくら,イクラ,イクラ"),
      make_node(
        "騒い",
        "動詞,自立,*,*,五段・ガ行,連用タ接続,騒ぐ,サワイ,サワイ",
      ),
      make_node("で", "助詞,接続助詞,*,*,*,*,で,デ,デ"),
      make_node("も", "助詞,係助詞,*,*,*,*,も,モ,モ"),
    ];
    let matches = find_matches(&grammar, "pattern", &nodes);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].start, 0);
    assert_eq!(matches[0].end, 4);
    // Only いくら(0) and も(3) are fixed; 騒い(1) and で(2) are wildcard
    assert!(matches[0].fixed_indices.contains(&0)); // いくら
    assert!(matches[0].fixed_indices.contains(&3)); // も
    assert!(!matches[0].fixed_indices.contains(&1)); // 騒い = wildcard
    assert!(!matches[0].fixed_indices.contains(&2)); // で = wildcard
  }

  #[test]
  fn test_fixed_indices_sequence() {
    let grammar = parse_grammar(r#"te_form = 動詞 助詞.接続助詞"て" ;"#).unwrap();
    let nodes = vec![
      make_node("食べ", "動詞,自立,*,*,一段,連用形,食べる,タベ,タベ"),
      make_node("て", "助詞,接続助詞,*,*,*,*,て,テ,テ"),
    ];
    let matches = find_matches(&grammar, "te_form", &nodes);
    assert_eq!(matches.len(), 1);
    // All nodes are fixed (no wildcards)
    assert_eq!(matches[0].fixed_indices, vec![0, 1]);
  }

  #[test]
  fn test_no_match() {
    let grammar = parse_grammar(r#"adj = 形容詞 ;"#).unwrap();
    let nodes = vec![make_node(
      "食べ",
      "動詞,自立,*,*,一段,連用形,食べる,タベ,タベ",
    )];
    let matches = find_matches(&grammar, "adj", &nodes);
    assert!(matches.is_empty());
  }

  #[test]
  fn test_base_form_match() {
    let grammar = parse_grammar(r#"suru = 動詞@"する" ;"#).unwrap();
    let nodes = vec![
      make_node("し", "動詞,自立,*,*,サ変・スル,連用形,する,シ,シ"),
      make_node("食べ", "動詞,自立,*,*,一段,連用形,食べる,タベ,タベ"),
    ];
    let matches = find_matches(&grammar, "suru", &nodes);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].start, 0);
  }

  #[test]
  fn test_one_or_more() {
    let grammar = parse_grammar(r#"compound = 名詞+ ;"#).unwrap();
    let nodes = vec![
      make_node(
        "東京",
        "名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トーキョー",
      ),
      make_node("大学", "名詞,一般,*,*,*,*,大学,ダイガク,ダイガク"),
      make_node("の", "助詞,連体化,*,*,*,*,の,ノ,ノ"),
    ];
    let matches = find_matches(&grammar, "compound", &nodes);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].start, 0);
    assert_eq!(matches[0].end, 2);
  }

  #[test]
  fn test_metadata_in_match() {
    let grammar = parse_grammar(
      r#"
            [N5 N4, "concession"]
            concession = "いくら" _* 助詞.係助詞"も" ;
        "#,
    )
    .unwrap();
    let nodes = vec![
      make_node("いくら", "副詞,一般,*,*,*,*,いくら,イクラ,イクラ"),
      make_node("も", "助詞,係助詞,*,*,*,*,も,モ,モ"),
    ];
    let matches = find_matches(&grammar, "concession", &nodes);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].levels, vec!["N5", "N4"]);
    assert_eq!(matches[0].description.as_deref(), Some("concession"));
  }

  #[test]
  fn test_conjugation_form_match() {
    let grammar = parse_grammar(r#"nominalization = 形容詞[ガル接続] "さ" ;"#).unwrap();
    let nodes = vec![
      make_node(
        "大き",
        "形容詞,自立,*,*,形容詞・イ段,ガル接続,大きい,オオキ,オーキ",
      ),
      make_node("さ", "名詞,接尾,特殊,*,*,*,さ,サ,サ"),
    ];
    let matches = find_matches(&grammar, "nominalization", &nodes);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].fixed_indices, vec![0, 1]);
  }

  #[test]
  fn test_conjugation_form_no_match() {
    // Wrong conjugation form should not match
    let grammar = parse_grammar(r#"nominalization = 形容詞[ガル接続] "さ" ;"#).unwrap();
    let nodes = vec![
      make_node(
        "大き",
        "形容詞,自立,*,*,形容詞・イ段,連用テ接続,大きい,オオキ,オーキ",
      ),
      make_node("さ", "名詞,接尾,特殊,*,*,*,さ,サ,サ"),
    ];
    let matches = find_matches(&grammar, "nominalization", &nodes);
    assert!(matches.is_empty());
  }

  #[test]
  fn test_sugiru_hiragana() {
    // すぎる in hiragana should match via base_form 過ぎる
    let grammar = parse_grammar(r#"sugiru = 動詞 動詞@"過ぎる" ;"#).unwrap();
    let nodes = vec![
      make_node("食べ", "動詞,自立,*,*,一段,連用形,食べる,タベ,タベ"),
      make_node("すぎる", "動詞,自立,*,*,上一段,基本形,過ぎる,スギル,スギル"),
    ];
    let matches = find_matches(&grammar, "sugiru", &nodes);
    assert_eq!(matches.len(), 1);
  }

  #[test]
  fn test_sugiru_kanji() {
    // 過ぎる in kanji should also match via base_form
    let grammar = parse_grammar(r#"sugiru = 動詞 動詞@"過ぎる" ;"#).unwrap();
    let nodes = vec![
      make_node("食べ", "動詞,自立,*,*,一段,連用形,食べる,タベ,タベ"),
      make_node("過ぎる", "動詞,自立,*,*,上一段,基本形,過ぎる,スギル,スギル"),
    ];
    let matches = find_matches(&grammar, "sugiru", &nodes);
    assert_eq!(matches.len(), 1);
  }

  #[test]
  fn test_find_all_matches() {
    let grammar = parse_grammar(
      r#"
            nouns = 名詞+ ;
            particles = 助詞 ;
        "#,
    )
    .unwrap();
    let nodes = vec![
      make_node(
        "東京",
        "名詞,固有名詞,地域,一般,*,*,東京,トウキョウ,トーキョー",
      ),
      make_node("の", "助詞,連体化,*,*,*,*,の,ノ,ノ"),
      make_node("大学", "名詞,一般,*,*,*,*,大学,ダイガク,ダイガク"),
    ];
    let matches = find_all_matches(&grammar, &nodes);
    assert!(matches.len() >= 2);
  }

  #[test]
  fn test_suffix_base_form_match() {
    // ~"上がる" should match any verb whose base form ends with 上がる
    let grammar = parse_grammar(r#"agaru = 動詞@~"上がる" ;"#).unwrap();
    let nodes = vec![
      make_node(
        "盛り上がっ",
        "動詞,自立,*,*,五段・ラ行,連用タ接続,盛り上がる,モリアガッ,モリアガッ",
      ),
      make_node("た", "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ"),
    ];
    let matches = find_matches(&grammar, "agaru", &nodes);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].start, 0);
  }

  #[test]
  fn test_suffix_base_form_no_match() {
    let grammar = parse_grammar(r#"agaru = 動詞@~"上がる" ;"#).unwrap();
    let nodes = vec![make_node(
      "食べ",
      "動詞,自立,*,*,一段,連用形,食べる,タベ,タベ",
    )];
    let matches = find_matches(&grammar, "agaru", &nodes);
    assert!(matches.is_empty());
  }

  #[test]
  fn test_regex_base_form_match() {
    // /す[るれ]/ should match する and すれ
    let grammar = parse_grammar(r#"su = 動詞@/す[るれ]/ ;"#).unwrap();
    let nodes = vec![
      make_node("し", "動詞,自立,*,*,サ変・スル,連用形,する,シ,シ"),
      make_node("すれ", "動詞,自立,*,*,サ変・スル,仮定形,すれ,スレ,スレ"),
    ];
    // "する" matches /す[るれ]/ and "すれ" matches /す[るれ]/
    let matches = find_matches(&grammar, "su", &nodes);
    assert_eq!(matches.len(), 2);
  }

  #[test]
  fn test_suffix_surface_match() {
    let grammar = parse_grammar(r#"rule = ~"ない" ;"#).unwrap();
    let nodes = vec![
      make_node("少ない", "形容詞,自立,*,*,形容詞・アウオ段,基本形,少ない,スクナイ,スクナイ"),
      make_node("多い", "形容詞,自立,*,*,形容詞・アウオ段,基本形,多い,オオイ,オーイ"),
    ];
    let matches = find_matches(&grammar, "rule", &nodes);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].start, 0);
  }

  // ── Capture / back-reference tests ─────────────────────────────────────

  #[test]
  fn test_capture_same_verb() {
    // 泣きに泣いた: 泣き(base=泣く) に 泣い(base=泣く) た → should match
    let grammar = parse_grammar(r#"n0_ni = 動詞$1 助詞"に" 動詞@=$1 助動詞* ;"#).unwrap();
    assert!(grammar.rules[0].uses_captures);
    let nodes = vec![
      make_node("泣き", "動詞,自立,*,*,五段・カ行イ音便,連用形,泣く,ナキ,ナキ"),
      make_node("に", "助詞,格助詞,一般,*,*,*,に,ニ,ニ"),
      make_node("泣い", "動詞,自立,*,*,五段・カ行イ音便,連用タ接続,泣く,ナイ,ナイ"),
      make_node("た", "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ"),
    ];
    let matches = find_matches(&grammar, "n0_ni", &nodes);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].start, 0);
    assert_eq!(matches[0].end, 4);
  }

  #[test]
  fn test_capture_different_verb_no_match() {
    // 会場に来た: 会場(名詞) に 来(base=来る) た → should NOT match
    // Even if we test with a verb first: 行きに来た → base 行く vs 来る
    let grammar = parse_grammar(r#"n0_ni = 動詞$1 助詞"に" 動詞@=$1 助動詞* ;"#).unwrap();
    let nodes = vec![
      make_node("行き", "動詞,自立,*,*,五段・カ行イ音便,連用形,行く,イキ,イキ"),
      make_node("に", "助詞,格助詞,一般,*,*,*,に,ニ,ニ"),
      make_node("来", "動詞,自立,*,*,カ変・来ル,連用形,来る,キ,キ"),
      make_node("た", "助動詞,*,*,*,特殊・タ,基本形,た,タ,タ"),
    ];
    let matches = find_matches(&grammar, "n0_ni", &nodes);
    assert!(matches.is_empty());
  }

  #[test]
  fn test_wildcard_capture() {
    // _$1 captures base_form from any token, @=$1 checks it
    let grammar =
      parse_grammar(r#"repeat = _$1 助詞"ば" _@=$1 ;"#).unwrap();
    assert!(grammar.rules[0].uses_captures);
    let nodes = vec![
      make_node("考えれ", "動詞,自立,*,*,一段,仮定形,考える,カンガエレ,カンガエレ"),
      make_node("ば", "助詞,接続助詞,*,*,*,*,ば,バ,バ"),
      make_node("考える", "動詞,自立,*,*,一段,基本形,考える,カンガエル,カンガエル"),
    ];
    let matches = find_matches(&grammar, "repeat", &nodes);
    assert_eq!(matches.len(), 1);
  }

  #[test]
  fn test_wildcard_capture_mismatch() {
    let grammar =
      parse_grammar(r#"repeat = _$1 助詞"ば" _@=$1 ;"#).unwrap();
    let nodes = vec![
      make_node("考えれ", "動詞,自立,*,*,一段,仮定形,考える,カンガエレ,カンガエレ"),
      make_node("ば", "助詞,接続助詞,*,*,*,*,ば,バ,バ"),
      make_node("食べる", "動詞,自立,*,*,一段,基本形,食べる,タベル,タベル"),
    ];
    let matches = find_matches(&grammar, "repeat", &nodes);
    assert!(matches.is_empty());
  }

  #[test]
  fn test_capture_in_alternative() {
    // Capture inside an alternative branch
    let grammar = parse_grammar(
      r#"rule = (名詞$1 | 動詞$1) 助詞"に" 動詞@=$1 ;"#,
    )
    .unwrap();
    assert!(grammar.rules[0].uses_captures);

    // Verb branch: same base_form should match
    let nodes = vec![
      make_node("泣き", "動詞,自立,*,*,五段・カ行イ音便,連用形,泣く,ナキ,ナキ"),
      make_node("に", "助詞,格助詞,一般,*,*,*,に,ニ,ニ"),
      make_node("泣い", "動詞,自立,*,*,五段・カ行イ音便,連用タ接続,泣く,ナイ,ナイ"),
    ];
    let matches = find_matches(&grammar, "rule", &nodes);
    assert_eq!(matches.len(), 1);
  }

  #[test]
  fn test_non_capture_rule_unchanged() {
    // Rules without captures should still work fine via the fast path
    let grammar = parse_grammar(r#"simple = 動詞 助詞"に" ;"#).unwrap();
    assert!(!grammar.rules[0].uses_captures);
    let nodes = vec![
      make_node("行き", "動詞,自立,*,*,五段・カ行イ音便,連用形,行く,イキ,イキ"),
      make_node("に", "助詞,格助詞,一般,*,*,*,に,ニ,ニ"),
    ];
    let matches = find_matches(&grammar, "simple", &nodes);
    assert_eq!(matches.len(), 1);
  }
}

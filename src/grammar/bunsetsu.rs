/// Bunsetsu (文節) segmentation for MeCab token streams.
///
/// A bunsetsu is a minimal syntactic phrase unit in Japanese, consisting of
/// one content word (自立語) followed by zero or more function words (付属語).
///
/// Bunsetsu boundaries help the grammar matcher understand phrase structure
/// without requiring a full dependency parser. Key use cases:
///
/// - Prevent matches from spanning unrelated phrases
/// - Identify the predicate bunsetsu (sentence-final verb phrase)
/// - Distinguish particle roles based on phrase position
use crate::node::MecabNode;

/// A bunsetsu (phrase chunk) spanning a range of token indices.
#[derive(Debug, Clone)]
pub struct Bunsetsu {
  /// Start index (inclusive) in the token array.
  pub start: usize,
  /// End index (exclusive) in the token array.
  pub end: usize,
  /// Index of the head (content word) token within the bunsetsu.
  pub head: usize,
}

/// Check if a token is a content word (自立語) that starts a new bunsetsu.
fn is_content_word(feature: &str) -> bool {
  let parts: Vec<&str> = feature.split(',').collect();
  if parts.is_empty() {
    return false;
  }
  let pos = parts[0];
  let sub = if parts.len() > 1 { parts[1] } else { "*" };

  match pos {
    // Independent nouns always start a new bunsetsu
    "名詞" => match sub {
      "非自立" | "接尾" => false, // dependent nouns/suffixes attach to previous
      _ => true,
    },
    // Independent verbs/adjectives start a new bunsetsu
    "動詞" => sub != "非自立" && sub != "接尾",
    "形容詞" => sub != "非自立",
    // Adverbs, prenominal adjectives, conjunctions, interjections
    "副詞" | "連体詞" | "接続詞" | "感動詞" | "フィラー" => true,
    // Prefixes start a new bunsetsu
    "接頭詞" => true,
    // Particles, auxiliaries, symbols are function words
    _ => false,
  }
}

/// Segment a token stream into bunsetsu (phrase chunks).
///
/// Each bunsetsu starts at a content word and extends until the next content word.
/// Punctuation (記号) is excluded from bunsetsu spans.
pub fn segment_bunsetsu(nodes: &[MecabNode]) -> Vec<Bunsetsu> {
  let mut result = Vec::new();
  let mut current_start: Option<usize> = None;
  let mut current_head: usize = 0;

  for (i, node) in nodes.iter().enumerate() {
    let parts: Vec<&str> = node.feature.split(',').collect();
    let pos = parts.first().copied().unwrap_or("*");

    // Skip punctuation - it doesn't belong to any bunsetsu
    if pos == "記号" {
      // Close current bunsetsu before punctuation
      if let Some(start) = current_start {
        result.push(Bunsetsu {
          start,
          end: i,
          head: current_head,
        });
        current_start = None;
      }
      continue;
    }

    if is_content_word(&node.feature) {
      // Close previous bunsetsu
      if let Some(start) = current_start {
        result.push(Bunsetsu {
          start,
          end: i,
          head: current_head,
        });
      }
      // Start new bunsetsu
      current_start = Some(i);
      current_head = i;
    } else if current_start.is_none() {
      // Function word at the start (rare) - start a bunsetsu anyway
      current_start = Some(i);
      current_head = i;
    }
  }

  // Close final bunsetsu
  if let Some(start) = current_start {
    result.push(Bunsetsu {
      start,
      end: nodes.len(),
      head: current_head,
    });
  }

  result
}

/// Find which bunsetsu a token index belongs to.
/// Returns None if the index is punctuation or out of range.
pub fn bunsetsu_of(chunks: &[Bunsetsu], token_idx: usize) -> Option<usize> {
  chunks
    .iter()
    .position(|b| token_idx >= b.start && token_idx < b.end)
}

/// Check if two token indices are in the same bunsetsu.
pub fn same_bunsetsu(chunks: &[Bunsetsu], a: usize, b: usize) -> bool {
  match (bunsetsu_of(chunks, a), bunsetsu_of(chunks, b)) {
    (Some(x), Some(y)) => x == y,
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
  fn test_basic_bunsetsu() {
    // 私は学生です。
    let nodes = vec![
      make_node("私", "名詞,代名詞,一般,*,*,*,私,ワタシ,ワタシ"),
      make_node("は", "助詞,係助詞,*,*,*,*,は,ハ,ワ"),
      make_node("学生", "名詞,一般,*,*,*,*,学生,ガクセイ,ガクセイ"),
      make_node("です", "助動詞,*,*,*,特殊・デス,基本形,です,デス,デス"),
      make_node("。", "記号,句点,*,*,*,*,。,。,。"),
    ];
    let chunks = segment_bunsetsu(&nodes);
    assert_eq!(chunks.len(), 2); // 私は | 学生です
    assert_eq!(chunks[0].start, 0);
    assert_eq!(chunks[0].end, 2);
    assert_eq!(chunks[1].start, 2);
    assert_eq!(chunks[1].end, 4);
  }

  #[test]
  fn test_same_bunsetsu() {
    let nodes = vec![
      make_node("食べ", "動詞,自立,*,*,一段,連用形,食べる,タベ,タベ"),
      make_node("て", "助詞,接続助詞,*,*,*,*,て,テ,テ"),
      make_node("いる", "動詞,非自立,*,*,一段,基本形,いる,イル,イル"),
    ];
    let chunks = segment_bunsetsu(&nodes);
    // 食べている is one bunsetsu (非自立 doesn't start a new one)
    assert_eq!(chunks.len(), 1);
    assert!(same_bunsetsu(&chunks, 0, 2));
  }
}

/// Token-level matching utilities for MeCab feature strings.
///
/// Provides functions to parse MeCab feature fields and match individual
/// tokens against grammar predicates.
use std::collections::HashMap;

use crate::node::MecabNode;

use super::syntax::*;

/// MeCab ipadic feature field indices.
/// Format: 品詞,品詞細分類1,品詞細分類2,品詞細分類3,活用型,活用形,原形,読み,発音
pub(crate) const FEATURE_CONJUGATION_TYPE: usize = 4; // 活用型
pub(crate) const FEATURE_CONJUGATION_FORM: usize = 5; // 活用形
pub(crate) const FEATURE_BASE_FORM: usize = 6; // 原形

/// Parse the comma-separated feature string from MeCab.
/// Feature format: 品詞,品詞細分類1,品詞細分類2,品詞細分類3,活用型,活用形,原形,読み,発音
pub(crate) fn parse_feature(feature: &str) -> Vec<&str> {
  feature.split(',').collect()
}

/// Check if a single MecabNode matches a TokenPredicate (using pre-parsed features).
pub(crate) fn token_matches_parts(node: &MecabNode, pred: &TokenPredicate, parts: &[&str]) -> bool {
  // Check POS hierarchy
  for (i, expected) in pred.pos.iter().enumerate() {
    if i >= parts.len() || parts[i] != expected.as_str() {
      return false;
    }
  }

  // Check surface form
  if let Some(ref m) = pred.surface {
    if !m.matches(&node.surface) {
      return false;
    }
  }

  // Check base form (原形)
  if let Some(ref m) = pred.base_form {
    if parts.len() <= FEATURE_BASE_FORM || !m.matches(parts[FEATURE_BASE_FORM]) {
      return false;
    }
  }

  // Check conjugation form (活用形)
  if let Some(ref m) = pred.conjugation_form {
    if parts.len() <= FEATURE_CONJUGATION_FORM || !m.matches(parts[FEATURE_CONJUGATION_FORM]) {
      return false;
    }
  }

  // Check conjugation type (活用型)
  if let Some(ref m) = pred.conjugation_type {
    if parts.len() <= FEATURE_CONJUGATION_TYPE || !m.matches(parts[FEATURE_CONJUGATION_TYPE]) {
      return false;
    }
  }

  true
}

/// Check if a single MecabNode matches a TokenPredicate.
pub(crate) fn token_matches(node: &MecabNode, pred: &TokenPredicate) -> bool {
  let parts = parse_feature(&node.feature);
  token_matches_parts(node, pred, &parts)
}

/// Extract the base_form from pre-parsed feature parts.
pub(crate) fn base_form_from_parts<'a>(parts: &[&'a str]) -> &'a str {
  if parts.len() > FEATURE_BASE_FORM {
    parts[FEATURE_BASE_FORM]
  } else {
    ""
  }
}

/// Capture map: slot number → captured base_form string.
pub(crate) type CaptureMap = HashMap<u8, String>;

/// Check if a token matches a predicate with capture support.
/// Parses features once and reuses for both static matching and capture logic.
/// Returns Some(updated_captures) on success, None on failure.
pub(crate) fn token_matches_with_captures(
  node: &MecabNode,
  pred: &TokenPredicate,
  captures: &CaptureMap,
) -> Option<CaptureMap> {
  let parts = parse_feature(&node.feature);

  // Check all static constraints first
  if !token_matches_parts(node, pred, &parts) {
    return None;
  }

  let base = base_form_from_parts(&parts);

  // Check back-reference constraint
  if let Some(slot) = pred.base_form_ref {
    match captures.get(&slot) {
      Some(captured) if captured == base => {}
      _ => return None,
    }
  }

  // Apply capture if requested
  let mut new_captures = captures.clone();
  if let Some(slot) = pred.capture {
    new_captures.insert(slot, base.to_string());
  }

  Some(new_captures)
}

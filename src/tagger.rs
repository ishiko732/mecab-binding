use crate::dict_info::{dict_info_to_vec, DictionaryInfo};
use crate::ffi;
use crate::node::{nodes_to_vec, MecabNode};
use crate::pack;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::ffi::{CStr, CString};
use std::path::PathBuf;

#[napi]
pub struct Tagger {
  inner: *mut ffi::mecab_t,
  /// Hold sentence CString alive for nbest iteration
  _nbest_sentence: Option<CString>,
  /// Temporary directory for fromBuffer; cleaned up on Drop
  _temp_dir: Option<PathBuf>,
}

// Tagger is NOT thread-safe per MeCab docs
// But napi-rs requires Send for classes. We ensure single-threaded usage
// by only accessing from JS main thread.
unsafe impl Send for Tagger {}

impl Drop for Tagger {
  fn drop(&mut self) {
    if !self.inner.is_null() {
      unsafe { ffi::mecab_destroy(self.inner) };
    }
    if let Some(ref dir) = self._temp_dir {
      let _ = std::fs::remove_dir_all(dir);
    }
  }
}

fn get_mecab_error(mecab: *mut ffi::mecab_t) -> String {
  let err = unsafe { ffi::mecab_strerror(mecab) };
  if err.is_null() {
    "Unknown MeCab error".to_string()
  } else {
    unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() }
  }
}

#[napi]
impl Tagger {
  #[napi(constructor)]
  pub fn new(args: String) -> Result<Self> {
    let c_args = CString::new(args).map_err(|e| Error::from_reason(e.to_string()))?;
    let ptr = unsafe { ffi::mecab_new2(c_args.as_ptr()) };
    if ptr.is_null() {
      // Get error from global error
      let err = unsafe { ffi::mecab_strerror(std::ptr::null_mut()) };
      let msg = if err.is_null() {
        "Failed to create MeCab tagger".to_string()
      } else {
        unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() }
      };
      return Err(Error::from_reason(msg));
    }
    Ok(Tagger {
      inner: ptr,
      _nbest_sentence: None,
      _temp_dir: None,
    })
  }

  /// Create a Tagger from a raw pointer (used by Model)
  pub(crate) fn from_raw(ptr: *mut ffi::mecab_t) -> Self {
    Tagger {
      inner: ptr,
      _nbest_sentence: None,
      _temp_dir: None,
    }
  }

  /// Create a Tagger from a gzip-compressed MCBD dictionary buffer.
  #[napi(factory)]
  pub fn from_buffer(data: &[u8]) -> Result<Self> {
    let files = pack::parse_mcbd(data)
      .map_err(|e| Error::from_reason(format!("Failed to parse MCBD: {}", e)))?;

    let dict_dir = PathBuf::from("/mecab-dict");
    std::fs::create_dir_all(&dict_dir).map_err(|e| {
      Error::from_reason(format!(
        "Failed to create dict dir {}: {}",
        dict_dir.display(),
        e
      ))
    })?;

    // Write dict files and an empty mecabrc to avoid requiring /dev/null
    for file in &files {
      let file_path = dict_dir.join(&file.name);
      std::fs::write(&file_path, &file.data).map_err(|e| {
        Error::from_reason(format!("Failed to write {}: {}", file_path.display(), e))
      })?;
    }
    let mecabrc = dict_dir.join("mecabrc");
    std::fs::write(&mecabrc, b"").map_err(|e| {
      Error::from_reason(format!("Failed to write mecabrc: {}", e))
    })?;

    let args = format!("-d {} -r {}", dict_dir.display(), mecabrc.display());
    let c_args = CString::new(args).map_err(|e| Error::from_reason(e.to_string()))?;
    let ptr = unsafe { ffi::mecab_new2(c_args.as_ptr()) };
    if ptr.is_null() {
      let _ = std::fs::remove_dir_all(&dict_dir);
      let err = unsafe { ffi::mecab_strerror(std::ptr::null_mut()) };
      let msg = if err.is_null() {
        "Failed to create MeCab tagger from buffer".to_string()
      } else {
        unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() }
      };
      return Err(Error::from_reason(msg));
    }
    Ok(Tagger {
      inner: ptr,
      _nbest_sentence: None,
      _temp_dir: Some(dict_dir),
    })
  }

  #[napi]
  pub fn parse(&self, input: String) -> Result<String> {
    let c_input = CString::new(input).map_err(|e| Error::from_reason(e.to_string()))?;
    let result = unsafe { ffi::mecab_sparse_tostr(self.inner, c_input.as_ptr()) };
    if result.is_null() {
      return Err(Error::from_reason(get_mecab_error(self.inner)));
    }
    let output = unsafe { CStr::from_ptr(result).to_string_lossy().into_owned() };
    Ok(output)
  }

  #[napi]
  pub fn parse_to_nodes(&self, input: String) -> Result<Vec<MecabNode>> {
    let c_input = CString::new(input).map_err(|e| Error::from_reason(e.to_string()))?;
    let node_ptr = unsafe { ffi::mecab_sparse_tonode(self.inner, c_input.as_ptr()) };
    if node_ptr.is_null() {
      return Err(Error::from_reason(get_mecab_error(self.inner)));
    }
    Ok(nodes_to_vec(node_ptr))
  }

  #[napi]
  pub fn parse_nbest(&self, n: u32, input: String) -> Result<String> {
    let c_input = CString::new(input).map_err(|e| Error::from_reason(e.to_string()))?;
    let result = unsafe { ffi::mecab_nbest_sparse_tostr(self.inner, n as usize, c_input.as_ptr()) };
    if result.is_null() {
      return Err(Error::from_reason(get_mecab_error(self.inner)));
    }
    let output = unsafe { CStr::from_ptr(result).to_string_lossy().into_owned() };
    Ok(output)
  }

  #[napi]
  pub fn parse_nbest_init(&mut self, input: String) -> Result<()> {
    let c_input = CString::new(input).map_err(|e| Error::from_reason(e.to_string()))?;
    let ret = unsafe { ffi::mecab_nbest_init(self.inner, c_input.as_ptr()) };
    if ret == 0 {
      return Err(Error::from_reason(get_mecab_error(self.inner)));
    }
    // Keep CString alive for subsequent nextNbest calls
    self._nbest_sentence = Some(c_input);
    Ok(())
  }

  #[napi]
  pub fn next_nbest(&self) -> Option<String> {
    let result = unsafe { ffi::mecab_nbest_next_tostr(self.inner) };
    if result.is_null() {
      None
    } else {
      Some(unsafe { CStr::from_ptr(result).to_string_lossy().into_owned() })
    }
  }

  #[napi]
  pub fn next_nbest_nodes(&self) -> Option<Vec<MecabNode>> {
    let node_ptr = unsafe { ffi::mecab_nbest_next_tonode(self.inner) };
    if node_ptr.is_null() {
      None
    } else {
      Some(nodes_to_vec(node_ptr))
    }
  }

  #[napi]
  pub fn dictionary_info(&self) -> Vec<DictionaryInfo> {
    let info_ptr = unsafe { ffi::mecab_dictionary_info(self.inner) };
    dict_info_to_vec(info_ptr)
  }

  #[napi(getter)]
  pub fn get_partial(&self) -> bool {
    unsafe { ffi::mecab_get_partial(self.inner) != 0 }
  }

  #[napi(setter)]
  pub fn set_partial(&self, partial: bool) {
    unsafe { ffi::mecab_set_partial(self.inner, if partial { 1 } else { 0 }) };
  }

  #[napi(getter)]
  pub fn get_theta(&self) -> f64 {
    unsafe { ffi::mecab_get_theta(self.inner) as f64 }
  }

  #[napi(setter)]
  pub fn set_theta(&self, theta: f64) {
    unsafe { ffi::mecab_set_theta(self.inner, theta as f32) };
  }

  /// Parse a lattice (used internally by Model workflow)
  pub(crate) fn parse_lattice(&self, lattice: *mut ffi::mecab_lattice_t) -> Result<()> {
    let ret = unsafe { ffi::mecab_parse_lattice(self.inner, lattice) };
    if ret == 0 {
      return Err(Error::from_reason(get_mecab_error(self.inner)));
    }
    Ok(())
  }
}

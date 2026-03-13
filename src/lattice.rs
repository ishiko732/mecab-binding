use crate::ffi;
use crate::node::{all_nodes_to_vec, MecabNode};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::ffi::{CStr, CString};

#[napi]
pub struct Lattice {
    inner: *mut ffi::mecab_lattice_t,
    /// Hold the sentence CString to keep the pointer alive
    _sentence: Option<CString>,
}

unsafe impl Send for Lattice {}

impl Drop for Lattice {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { ffi::mecab_lattice_destroy(self.inner) };
        }
    }
}

#[napi]
impl Lattice {
    #[napi(constructor)]
    pub fn new() -> Result<Self> {
        let ptr = unsafe { ffi::mecab_lattice_new() };
        if ptr.is_null() {
            return Err(Error::from_reason("Failed to create lattice"));
        }
        Ok(Lattice {
            inner: ptr,
            _sentence: None,
        })
    }

    pub(crate) fn from_raw(ptr: *mut ffi::mecab_lattice_t) -> Self {
        Lattice {
            inner: ptr,
            _sentence: None,
        }
    }

    pub(crate) fn as_ptr(&self) -> *mut ffi::mecab_lattice_t {
        self.inner
    }

    #[napi]
    pub fn set_sentence(&mut self, sentence: String) -> Result<()> {
        let c_sentence =
            CString::new(sentence).map_err(|e| Error::from_reason(e.to_string()))?;
        unsafe { ffi::mecab_lattice_set_sentence(self.inner, c_sentence.as_ptr()) };
        // Keep CString alive so the pointer remains valid
        self._sentence = Some(c_sentence);
        Ok(())
    }

    #[napi]
    pub fn get_bos_node(&self) -> Vec<MecabNode> {
        let node_ptr = unsafe { ffi::mecab_lattice_get_bos_node(self.inner) };
        if node_ptr.is_null() {
            Vec::new()
        } else {
            all_nodes_to_vec(node_ptr)
        }
    }

    #[napi]
    pub fn to_string_result(&self) -> Option<String> {
        let result = unsafe { ffi::mecab_lattice_tostr(self.inner) };
        if result.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(result).to_string_lossy().into_owned() })
        }
    }

    #[napi]
    pub fn clear(&mut self) {
        unsafe { ffi::mecab_lattice_clear(self.inner) };
        self._sentence = None;
    }

    #[napi]
    pub fn next(&self) -> bool {
        unsafe { ffi::mecab_lattice_next(self.inner) != 0 }
    }

    #[napi]
    pub fn set_request_type(&self, request_type: i32) {
        unsafe { ffi::mecab_lattice_set_request_type(self.inner, request_type) };
    }

    #[napi]
    pub fn get_request_type(&self) -> i32 {
        unsafe { ffi::mecab_lattice_get_request_type(self.inner) }
    }
}

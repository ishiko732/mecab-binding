use crate::dict_info::{dict_info_to_vec, DictionaryInfo};
use crate::ffi;
use crate::lattice::Lattice;
use crate::tagger::Tagger;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::ffi::{CStr, CString};

#[napi]
pub struct Model {
    inner: *mut ffi::mecab_model_t,
}

unsafe impl Send for Model {}

impl Drop for Model {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { ffi::mecab_model_destroy(self.inner) };
        }
    }
}

#[napi]
impl Model {
    #[napi(constructor)]
    pub fn new(args: String) -> Result<Self> {
        let c_args = CString::new(args).map_err(|e| Error::from_reason(e.to_string()))?;
        let ptr = unsafe { ffi::mecab_model_new2(c_args.as_ptr()) };
        if ptr.is_null() {
            let err = unsafe { ffi::mecab_strerror(std::ptr::null_mut()) };
            let msg = if err.is_null() {
                "Failed to create MeCab model".to_string()
            } else {
                unsafe { CStr::from_ptr(err).to_string_lossy().into_owned() }
            };
            return Err(Error::from_reason(msg));
        }
        Ok(Model { inner: ptr })
    }

    #[napi]
    pub fn create_tagger(&self) -> Result<Tagger> {
        let ptr = unsafe { ffi::mecab_model_new_tagger(self.inner) };
        if ptr.is_null() {
            return Err(Error::from_reason("Failed to create tagger from model"));
        }
        Ok(Tagger::from_raw(ptr))
    }

    #[napi]
    pub fn create_lattice(&self) -> Result<Lattice> {
        let ptr = unsafe { ffi::mecab_model_new_lattice(self.inner) };
        if ptr.is_null() {
            return Err(Error::from_reason("Failed to create lattice from model"));
        }
        Ok(Lattice::from_raw(ptr))
    }

    #[napi]
    pub fn dictionary_info(&self) -> Vec<DictionaryInfo> {
        let info_ptr = unsafe { ffi::mecab_model_dictionary_info(self.inner) };
        dict_info_to_vec(info_ptr)
    }
}

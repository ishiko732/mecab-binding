#![deny(clippy::all)]
#![allow(clippy::needless_return)]
#![allow(non_snake_case, non_camel_case_types, dead_code)]

mod dict_info;
#[allow(non_snake_case, dead_code)]
mod ffi;
mod node;
mod pack;
mod tagger;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::ffi::{CStr, CString};

#[napi]
pub fn mecab_version() -> String {
    let version = unsafe { ffi::mecab_version() };
    unsafe { CStr::from_ptr(version).to_string_lossy().into_owned() }
}

#[napi(object)]
pub struct DictIndexOptions {
    pub input_dir: String,
    pub output_dir: String,
    pub from_charset: String,
    pub to_charset: String,
}

#[napi]
pub fn dict_index(options: DictIndexOptions) -> Result<()> {
    // Build argv for mecab_dict_index
    let program = CString::new("mecab-dict-index").unwrap();
    let d_flag = CString::new("-d").unwrap();
    let d_val =
        CString::new(options.input_dir.as_str()).map_err(|e| Error::from_reason(e.to_string()))?;
    let o_flag = CString::new("-o").unwrap();
    let o_val = CString::new(options.output_dir.as_str())
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let f_flag = CString::new("-f").unwrap();
    let f_val = CString::new(options.from_charset.as_str())
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let t_flag = CString::new("-t").unwrap();
    let t_val =
        CString::new(options.to_charset.as_str()).map_err(|e| Error::from_reason(e.to_string()))?;

    let mut argv: Vec<*mut i8> = vec![
        program.as_ptr() as *mut i8,
        d_flag.as_ptr() as *mut i8,
        d_val.as_ptr() as *mut i8,
        o_flag.as_ptr() as *mut i8,
        o_val.as_ptr() as *mut i8,
        f_flag.as_ptr() as *mut i8,
        f_val.as_ptr() as *mut i8,
        t_flag.as_ptr() as *mut i8,
        t_val.as_ptr() as *mut i8,
    ];

    // Create output dir if not exists
    std::fs::create_dir_all(&options.output_dir)
        .map_err(|e| Error::from_reason(format!("Failed to create output dir: {}", e)))?;

    let ret = unsafe { ffi::mecab_dict_index(argv.len() as i32, argv.as_mut_ptr()) };
    if ret != 0 {
        return Err(Error::from_reason(format!(
            "mecab_dict_index failed with code {}",
            ret
        )));
    }
    Ok(())
}

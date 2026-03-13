use crate::ffi;
use napi_derive::napi;
use std::ffi::CStr;

#[napi(object)]
#[derive(Clone)]
pub struct DictionaryInfo {
    pub filename: String,
    pub charset: String,
    pub size: u32,
    /// 0 = SYS_DIC, 1 = USR_DIC, 2 = UNK_DIC
    pub r#type: i32,
    pub lsize: u32,
    pub rsize: u32,
    pub version: u32,
}

pub fn dict_info_to_vec(info_ptr: *const ffi::mecab_dictionary_info_t) -> Vec<DictionaryInfo> {
    let mut result = Vec::new();
    let mut current = info_ptr;
    while !current.is_null() {
        let info = unsafe { &*current };
        let filename = unsafe {
            CStr::from_ptr(info.filename)
                .to_string_lossy()
                .into_owned()
        };
        let charset = unsafe {
            CStr::from_ptr(info.charset)
                .to_string_lossy()
                .into_owned()
        };
        result.push(DictionaryInfo {
            filename,
            charset,
            size: info.size,
            r#type: info.type_,
            lsize: info.lsize,
            rsize: info.rsize,
            version: info.version as u32,
        });
        current = info.next;
    }
    result
}

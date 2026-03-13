use crate::ffi;
use napi_derive::napi;
use std::ffi::CStr;

#[napi(object)]
#[derive(Clone)]
pub struct MecabNode {
    pub surface: String,
    pub feature: String,
    pub id: u32,
    pub length: u32,
    pub rlength: u32,
    pub rc_attr: u32,
    pub lc_attr: u32,
    pub posid: u32,
    pub char_type: u32,
    pub stat: u32,
    pub isbest: bool,
    pub alpha: f64,
    pub beta: f64,
    pub prob: f64,
    pub wcost: i32,
    pub cost: i64,
}

/// Convert a C node linked-list to a Vec<MecabNode>, skipping BOS/EOS nodes.
pub fn nodes_to_vec(node_ptr: *const ffi::mecab_node_t) -> Vec<MecabNode> {
    let mut nodes = Vec::new();
    let mut current = node_ptr;
    while !current.is_null() {
        let node = unsafe { &*current };
        // Skip BOS (2), EOS (3), EON (4)
        if node.stat < 2 {
            let surface = unsafe {
                let bytes =
                    std::slice::from_raw_parts(node.surface as *const u8, node.length as usize);
                String::from_utf8_lossy(bytes).into_owned()
            };
            let feature = unsafe {
                CStr::from_ptr(node.feature)
                    .to_string_lossy()
                    .into_owned()
            };
            nodes.push(MecabNode {
                surface,
                feature,
                id: node.id,
                length: node.length as u32,
                rlength: node.rlength as u32,
                rc_attr: node.rcAttr as u32,
                lc_attr: node.lcAttr as u32,
                posid: node.posid as u32,
                char_type: node.char_type as u32,
                stat: node.stat as u32,
                isbest: node.isbest != 0,
                alpha: node.alpha as f64,
                beta: node.beta as f64,
                prob: node.prob as f64,
                wcost: node.wcost as i32,
                cost: node.cost as i64,
            });
        }
        current = node.next;
    }
    nodes
}

/// Convert a C node linked-list to a Vec<MecabNode>, including BOS/EOS.
pub fn all_nodes_to_vec(node_ptr: *const ffi::mecab_node_t) -> Vec<MecabNode> {
    let mut nodes = Vec::new();
    let mut current = node_ptr;
    while !current.is_null() {
        let node = unsafe { &*current };
        let surface = if node.length > 0 {
            unsafe {
                let bytes =
                    std::slice::from_raw_parts(node.surface as *const u8, node.length as usize);
                String::from_utf8_lossy(bytes).into_owned()
            }
        } else {
            String::new()
        };
        let feature = unsafe {
            CStr::from_ptr(node.feature)
                .to_string_lossy()
                .into_owned()
        };
        nodes.push(MecabNode {
            surface,
            feature,
            id: node.id,
            length: node.length as u32,
            rlength: node.rlength as u32,
            rc_attr: node.rcAttr as u32,
            lc_attr: node.lcAttr as u32,
            posid: node.posid as u32,
            char_type: node.char_type as u32,
            stat: node.stat as u32,
            isbest: node.isbest != 0,
            alpha: node.alpha as f64,
            beta: node.beta as f64,
            prob: node.prob as f64,
            wcost: node.wcost as i32,
            cost: node.cost as i64,
        });
        current = node.next;
    }
    nodes
}

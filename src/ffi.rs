use std::os::raw::{c_char, c_float, c_int, c_long, c_short, c_uchar, c_uint, c_ushort};

#[repr(C)]
pub struct mecab_node_t {
    pub prev: *mut mecab_node_t,
    pub next: *mut mecab_node_t,
    pub enext: *mut mecab_node_t,
    pub bnext: *mut mecab_node_t,
    pub rpath: *mut mecab_path_t,
    pub lpath: *mut mecab_path_t,
    /// Not null-terminated! Use `length` to extract.
    pub surface: *const c_char,
    pub feature: *const c_char,
    pub id: c_uint,
    pub length: c_ushort,
    pub rlength: c_ushort,
    pub rcAttr: c_ushort,
    pub lcAttr: c_ushort,
    pub posid: c_ushort,
    pub char_type: c_uchar,
    pub stat: c_uchar,
    pub isbest: c_uchar,
    pub alpha: c_float,
    pub beta: c_float,
    pub prob: c_float,
    pub wcost: c_short,
    pub cost: c_long,
}

#[repr(C)]
pub struct mecab_path_t {
    pub rnode: *mut mecab_node_t,
    pub rnext: *mut mecab_path_t,
    pub lnode: *mut mecab_node_t,
    pub lnext: *mut mecab_path_t,
    pub cost: c_int,
    pub prob: c_float,
}

#[repr(C)]
pub struct mecab_dictionary_info_t {
    pub filename: *const c_char,
    pub charset: *const c_char,
    pub size: c_uint,
    pub type_: c_int,
    pub lsize: c_uint,
    pub rsize: c_uint,
    pub version: c_ushort,
    pub next: *mut mecab_dictionary_info_t,
}

// Opaque types
pub enum mecab_t {}
pub enum mecab_model_t {}
pub enum mecab_lattice_t {}

extern "C" {
    // Tagger API
    pub fn mecab_new2(arg: *const c_char) -> *mut mecab_t;
    pub fn mecab_destroy(mecab: *mut mecab_t);
    pub fn mecab_version() -> *const c_char;
    pub fn mecab_strerror(mecab: *mut mecab_t) -> *const c_char;

    pub fn mecab_sparse_tostr(mecab: *mut mecab_t, str: *const c_char) -> *const c_char;
    pub fn mecab_sparse_tonode(mecab: *mut mecab_t, str: *const c_char) -> *const mecab_node_t;
    pub fn mecab_nbest_sparse_tostr(
        mecab: *mut mecab_t,
        n: usize,
        str: *const c_char,
    ) -> *const c_char;
    pub fn mecab_nbest_init(mecab: *mut mecab_t, str: *const c_char) -> c_int;
    pub fn mecab_nbest_next_tostr(mecab: *mut mecab_t) -> *const c_char;
    pub fn mecab_nbest_next_tonode(mecab: *mut mecab_t) -> *const mecab_node_t;

    pub fn mecab_get_partial(mecab: *mut mecab_t) -> c_int;
    pub fn mecab_set_partial(mecab: *mut mecab_t, partial: c_int);
    pub fn mecab_get_theta(mecab: *mut mecab_t) -> c_float;
    pub fn mecab_set_theta(mecab: *mut mecab_t, theta: c_float);

    pub fn mecab_dictionary_info(mecab: *mut mecab_t) -> *const mecab_dictionary_info_t;
    pub fn mecab_parse_lattice(mecab: *mut mecab_t, lattice: *mut mecab_lattice_t) -> c_int;

    // Model API
    pub fn mecab_model_new2(arg: *const c_char) -> *mut mecab_model_t;
    pub fn mecab_model_destroy(model: *mut mecab_model_t);
    pub fn mecab_model_new_tagger(model: *mut mecab_model_t) -> *mut mecab_t;
    pub fn mecab_model_new_lattice(model: *mut mecab_model_t) -> *mut mecab_lattice_t;
    pub fn mecab_model_dictionary_info(
        model: *mut mecab_model_t,
    ) -> *const mecab_dictionary_info_t;

    // Lattice API
    pub fn mecab_lattice_new() -> *mut mecab_lattice_t;
    pub fn mecab_lattice_destroy(lattice: *mut mecab_lattice_t);
    pub fn mecab_lattice_clear(lattice: *mut mecab_lattice_t);
    pub fn mecab_lattice_set_sentence(lattice: *mut mecab_lattice_t, sentence: *const c_char);
    pub fn mecab_lattice_get_bos_node(lattice: *mut mecab_lattice_t) -> *mut mecab_node_t;
    pub fn mecab_lattice_tostr(lattice: *mut mecab_lattice_t) -> *const c_char;
    pub fn mecab_lattice_next(lattice: *mut mecab_lattice_t) -> c_int;
    pub fn mecab_lattice_set_request_type(lattice: *mut mecab_lattice_t, request_type: c_int);
    pub fn mecab_lattice_get_request_type(lattice: *mut mecab_lattice_t) -> c_int;
    pub fn mecab_lattice_strerror(lattice: *mut mecab_lattice_t) -> *const c_char;

    // Dict index
    pub fn mecab_dict_index(argc: c_int, argv: *mut *mut c_char) -> c_int;
}

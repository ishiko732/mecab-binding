use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::io::{Read, Write};

/// MCBD magic bytes
const MCBD_MAGIC: &[u8; 4] = b"MCBD";
/// MCBD format version
const MCBD_VERSION: u32 = 1;
/// Fixed list of dictionary files to pack
const DICT_FILES: &[&str] = &["char.bin", "dicrc", "matrix.bin", "sys.dic", "unk.dic"];

/// Pack a MeCab dictionary directory into a single gzip-compressed `.data` file (MCBD format).
#[napi]
pub fn pack_dict(dict_dir: String, output_path: String) -> Result<()> {
    // Build MCBD payload in memory
    let mut payload: Vec<u8> = Vec::new();
    payload.extend_from_slice(MCBD_MAGIC);
    payload.extend_from_slice(&MCBD_VERSION.to_le_bytes());
    payload.extend_from_slice(&(DICT_FILES.len() as u32).to_le_bytes());

    let dict_path = std::path::Path::new(&dict_dir);
    for name in DICT_FILES {
        let file_path = dict_path.join(name);
        let data = std::fs::read(&file_path).map_err(|e| {
            Error::from_reason(format!("Failed to read {}: {}", file_path.display(), e))
        })?;
        let name_bytes = name.as_bytes();
        payload.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
        payload.extend_from_slice(name_bytes);
        payload.extend_from_slice(&(data.len() as u32).to_le_bytes());
        payload.extend_from_slice(&data);
    }

    // Gzip compress and write to output
    if let Some(parent) = std::path::Path::new(&output_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            Error::from_reason(format!("Failed to create output directory: {}", e))
        })?;
    }
    let output_file = std::fs::File::create(&output_path)
        .map_err(|e| Error::from_reason(format!("Failed to create {}: {}", output_path, e)))?;
    let mut encoder = GzEncoder::new(output_file, Compression::default());
    encoder
        .write_all(&payload)
        .map_err(|e| Error::from_reason(format!("Failed to write compressed data: {}", e)))?;
    encoder
        .finish()
        .map_err(|e| Error::from_reason(format!("Failed to finish gzip: {}", e)))?;

    Ok(())
}

/// Parsed file entry from MCBD format.
pub struct McbdFile {
    pub name: String,
    pub data: Vec<u8>,
}

/// Decompress and parse MCBD buffer. Returns list of (name, data) pairs.
pub fn parse_mcbd(compressed: &[u8]) -> std::result::Result<Vec<McbdFile>, String> {
    // Gzip decompress
    let mut decoder = GzDecoder::new(compressed);
    let mut payload = Vec::new();
    decoder
        .read_to_end(&mut payload)
        .map_err(|e| format!("Failed to decompress gzip: {}", e))?;

    // Read helper
    let read_u32 = |off: &mut usize| -> std::result::Result<u32, String> {
        if *off + 4 > payload.len() {
            return Err("Unexpected end of MCBD data".to_string());
        }
        let val = u32::from_le_bytes(payload[*off..*off + 4].try_into().unwrap());
        *off += 4;
        Ok(val)
    };

    // Magic
    if payload.len() < 4 || &payload[0..4] != MCBD_MAGIC {
        return Err("Invalid MCBD magic".to_string());
    }
    let mut offset = 4;

    // Version
    let version = read_u32(&mut offset)?;
    if version != MCBD_VERSION {
        return Err(format!("Unsupported MCBD version: {}", version));
    }

    // NumFiles
    let num_files = read_u32(&mut offset)? as usize;
    let mut files = Vec::with_capacity(num_files);

    for _ in 0..num_files {
        let name_len = read_u32(&mut offset)? as usize;
        if offset + name_len > payload.len() {
            return Err("Unexpected end of MCBD data (name)".to_string());
        }
        let name = std::str::from_utf8(&payload[offset..offset + name_len])
            .map_err(|e| format!("Invalid UTF-8 in file name: {}", e))?
            .to_string();
        offset += name_len;

        let data_len = read_u32(&mut offset)? as usize;
        if offset + data_len > payload.len() {
            return Err("Unexpected end of MCBD data (file data)".to_string());
        }
        let data = payload[offset..offset + data_len].to_vec();
        offset += data_len;

        files.push(McbdFile { name, data });
    }

    Ok(files)
}

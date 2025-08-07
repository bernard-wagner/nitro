#![cfg_attr(target_arch = "wasm32", no_std)]
extern crate alloc;

use alloc::vec::Vec;
use core::mem::MaybeUninit;

use crate::{BrotliStatus, Dictionary};
use rust_brotli::{
    enc::{BrotliEncoderParams, StandardAlloc},
    CustomRead, CustomWrite,
};

/// Computes an upper bound on compressed size for a given input size and level.
pub fn compression_bound(len: usize, level: u32) -> usize {
    let base_bound = rust_brotli::enc::BrotliEncoderMaxCompressedSize(len);
    if level <= 2 {
        base_bound.max(len + (len >> 10) * 8 + 64)
    } else {
        base_bound
    }
}

/// Compress a slice into a new `Vec<u8>`.
pub fn compress(
    input: &[u8],
    level: u32,
    window_size: u32,
    dictionary: Dictionary,
) -> Result<Vec<u8>, BrotliStatus> {
    compress_into(input, Vec::new(), level, window_size, dictionary)
}

/// Compress into a provided `Vec<u8>`, appending the result.
pub fn compress_into(
    input: &[u8],
    mut output: Vec<u8>,
    level: u32,
    window_size: u32,
    dictionary: Dictionary,
) -> Result<Vec<u8>, BrotliStatus> {
    output.reserve_exact(compression_bound(input.len(), level));

    let space = output.spare_capacity_mut();
    let count = compress_fixed(input, space, level, window_size, dictionary)?.len();
    unsafe {
        output.set_len(output.len() + count);
    }
    Ok(output)
}

/// Compress into a fixed-size uninitialized buffer.
pub fn compress_fixed<'a>(
    input: &'a [u8],
    output: &'a mut [MaybeUninit<u8>],
    level: u32,
    window_size: u32,
    dictionary: Dictionary,
) -> Result<&'a [u8], BrotliStatus> {
    let mut params = BrotliEncoderParams::default();
    params.quality = level as i32;
    params.lgwin = window_size as i32;
    if matches!(dictionary, Dictionary::StylusProgram) {
        params.use_dictionary = true;
    }

    let mut reader = SliceReader::new(input);
    let mut writer = SliceWriter::new(unsafe {
        // SAFETY: brotli will initialize every byte written
        core::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u8, output.len())
    });

    let mut input_buffer = [0u8; 4096];
    let mut output_buffer = [0u8; 4096];

    let mut nop_callback = |_data: &mut rust_brotli::interface::PredictionModeContextMap<
        rust_brotli::InputReferenceMut,
    >,
                            _cmds: &mut [rust_brotli::interface::StaticCommand],
                            _mb: rust_brotli::interface::InputPair,
                            _m: &mut StandardAlloc| ();

    let dict = dictionary.slice().unwrap_or(&[]);

    rust_brotli::BrotliCompressCustomIoCustomDict(
        &mut reader,
        &mut writer,
        &mut input_buffer,
        &mut output_buffer,
        &params,
        StandardAlloc::default(),
        &mut nop_callback,
        dict,
        (),
    )
    .map_err(|_| BrotliStatus::Failure)?;

    let bytes_written = writer.position();
    if bytes_written > output.len() {
        return Err(BrotliStatus::Failure);
    }

    // SAFETY: only initialized bytes are included in result
    Ok(unsafe {
        core::slice::from_raw_parts(output.as_ptr() as *const u8, bytes_written)
    })
}

/// Decompress a slice into a `Vec<u8>`.
pub fn decompress(input: &[u8], dictionary: Dictionary) -> Result<Vec<u8>, BrotliStatus> {
    let mut writer = VecWriter::with_capacity(input.len() * 2);
    let mut reader = SliceReader::new(input);

    let mut input_buffer = [0u8; 4096];
    let mut output_buffer = [0u8; 4096];

    let dict = dictionary.slice().unwrap_or(&[]);

    rust_brotli::BrotliDecompressCustomIoCustomDict(
        &mut reader,
        &mut writer,
        &mut input_buffer,
        &mut output_buffer,
        StandardAlloc::default(),
        StandardAlloc::default(),
        StandardAlloc::default(),
        dict.to_vec().into(),
        (),
    )
    .map_err(|_| BrotliStatus::Failure)?;

    Ok(writer.buf)
}

/// Decompress into a fixed-size uninitialized buffer.
pub fn decompress_fixed<'a>(
    input: &'a [u8],
    output: &'a mut [MaybeUninit<u8>],
    dictionary: Dictionary,
) -> Result<&'a [u8], BrotliStatus> {
    let mut reader = SliceReader::new(input);
    let mut writer = SliceWriter::new(unsafe {
        core::slice::from_raw_parts_mut(output.as_mut_ptr() as *mut u8, output.len())
    });

    let mut input_buffer = [0u8; 4096];
    let mut output_buffer = [0u8; 4096];

    let dict = dictionary.slice().unwrap_or(&[]);

    rust_brotli::BrotliDecompressCustomIoCustomDict(
        &mut reader,
        &mut writer,
        &mut input_buffer,
        &mut output_buffer,
        StandardAlloc::default(),
        StandardAlloc::default(),
        StandardAlloc::default(),
        dict.to_vec().into(),
        (),
    )
    .map_err(|_| BrotliStatus::Failure)?;

    let bytes_written = writer.position();
    if bytes_written > output.len() {
        return Err(BrotliStatus::Failure);
    }

    Ok(unsafe {
        core::slice::from_raw_parts(output.as_ptr() as *const u8, bytes_written)
    })
}

/// Writer that writes into a mutable byte slice.
pub struct SliceWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> SliceWriter<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }
}

impl<'a> CustomWrite<()> for SliceWriter<'a> {
    fn write(&mut self, data: &[u8]) -> Result<usize, ()> {
        let to_write = core::cmp::min(data.len(), self.remaining());
        if to_write == 0 {
            return Ok(0);
        }
        self.buf[self.pos..self.pos + to_write].copy_from_slice(&data[..to_write]);
        self.pos += to_write;
        Ok(to_write)
    }

    fn flush(&mut self) -> Result<(), ()> {
        Ok(())
    }
}

/// Reader that reads from a byte slice.
pub struct SliceReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> SliceReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }
}

impl<'a> CustomRead<()> for SliceReader<'a> {
    fn read(&mut self, data: &mut [u8]) -> Result<usize, ()> {
        let to_read = core::cmp::min(data.len(), self.remaining());
        if to_read == 0 {
            return Ok(0);
        }
        data[..to_read].copy_from_slice(&self.buf[self.pos..self.pos + to_read]);
        self.pos += to_read;
        Ok(to_read)
    }
}

/// Writer that appends to a `Vec<u8>`.
pub struct VecWriter {
    buf: Vec<u8>,
}

impl VecWriter {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self { buf: Vec::with_capacity(cap) }
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.buf
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }
}

impl Default for VecWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomWrite<()> for VecWriter {
    fn write(&mut self, data: &[u8]) -> Result<usize, ()> {
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> Result<(), ()> {
        Ok(())
    }
}
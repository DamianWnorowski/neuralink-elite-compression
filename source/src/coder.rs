use bitstream_io::{BigEndian, BitRead, BitReader, BitWrite, BitWriter};
use std::io::Cursor;
use anyhow::Result;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

/// Encode residuals using Rice Coding (Lossless)
pub fn encode_rice(residuals: &[i32], k: u32) -> Result<Vec<u8>> {
    let mut writer = BitWriter::endian(Vec::new(), BigEndian);
    let m = 1 << k;
    for &val in residuals {
        let u_val = if val >= 0 { (val as u32) << 1 } else { ((val.abs() as u32) << 1) - 1 };
        let q = u_val >> k;
        let r = u_val & (m - 1);
        for _ in 0..q { writer.write_bit(true)?; }
        writer.write_bit(false)?;
        writer.write(k, r)?;
    }
    writer.byte_align()?;
    Ok(writer.into_writer())
}

/// Decode residuals using Rice Coding
pub fn decode_rice(data: &[u8], count: usize, k: u32) -> Result<Vec<i32>> {
    let mut reader = BitReader::endian(Cursor::new(data), BigEndian);
    let mut residuals = Vec::with_capacity(count);
    for _ in 0..count {
        let mut q = 0;
        while reader.read_bit()? { q += 1; }
        let r = reader.read::<u32>(k)?;
        let u_val = (q << k) | r;
        let val = if u_val % 2 == 0 { (u_val >> 1) as i32 } else { -(((u_val + 1) >> 1) as i32) };
        residuals.push(val);
    }
    Ok(residuals)
}

const L_BITS: u32 = 16;
const L: u32 = 1 << L_BITS;
const M_BITS: u32 = 12;
const M: u32 = 1 << M_BITS;

pub struct RansCoder;

impl RansCoder {
    pub fn encode(data: &[u8]) -> Result<Vec<u8>> {
        if data.is_empty() { return Ok(Vec::new()); }
        let mut freq = [0u32; 256];
        for &b in data { freq[b as usize] += 1; }
        let total = data.len() as u64;
        let mut normalized_freq = [0u32; 256];
        let mut sum = 0;
        for i in 0..256 {
            if freq[i] > 0 {
                normalized_freq[i] = ((freq[i] as u64 * M as u64) / total).max(1) as u32;
                sum += normalized_freq[i];
            }
        }
        while sum > M { for i in 0..256 { if normalized_freq[i] > 1 { normalized_freq[i] -= 1; sum -= 1; if sum == M { break; } } } }
        while sum < M { for i in 0..256 { if freq[i] > 0 { normalized_freq[i] += 1; sum += 1; if sum == M { break; } } } }
        let mut cum_freq = [0u32; 257];
        for i in 0..256 { cum_freq[i + 1] = cum_freq[i] + normalized_freq[i]; }
        let mut state = L;
        let mut out = Vec::new();
        for &s in data.iter().rev() {
            let s = s as usize;
            let f = normalized_freq[s];
            let b = cum_freq[s];
            while state >= (f << (32 - M_BITS)) { out.push((state & 0xFF) as u8); state >>= 8; }
            state = ((state / f) << M_BITS) + (state % f) + b;
        }
        let mut final_out = Vec::new();
        final_out.write_u32::<LittleEndian>(state)?;
        final_out.extend_from_slice(&out);
        for i in 0..256 { final_out.write_u16::<LittleEndian>(normalized_freq[i] as u16)?; }
        Ok(final_out)
    }

    pub fn decode(data: &[u8], count: usize) -> Result<Vec<u8>> {
        if data.is_empty() { return Ok(Vec::new()); }
        let mut cursor = Cursor::new(data);
        let mut state = cursor.read_u32::<LittleEndian>()?;
        let freq_start = data.len() - 512;
        let mut normalized_freq = [0u32; 256];
        let mut f_cursor = Cursor::new(&data[freq_start..]);
        for i in 0..256 { normalized_freq[i] = f_cursor.read_u16::<LittleEndian>()? as u32; }
        let mut cum_freq = [0u32; 257];
        for i in 0..256 { cum_freq[i+1] = cum_freq[i] + normalized_freq[i]; }
        let mut symbol_map = [0u8; M as usize];
        for s in 0..256 { for j in cum_freq[s]..cum_freq[s+1] { symbol_map[j as usize] = s as u8; } }
        let mut out = Vec::with_capacity(count);
        let mut pos = 4;
        while (count == 0 && pos < freq_start) || (count > 0 && out.len() < count) {
            let slot = state & (M - 1);
            let s = symbol_map[slot as usize];
            out.push(s);
            let f = normalized_freq[s as usize];
            let b = cum_freq[s as usize];
            state = f * (state >> M_BITS) + slot - b;
            while state < L && pos < freq_start { state = (state << 8) | data[pos] as u32; pos += 1; }
        }
        Ok(out)
    }
}

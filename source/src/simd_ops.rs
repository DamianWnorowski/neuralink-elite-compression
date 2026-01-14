use byteorder::{BigEndian, WriteBytesExt, ReadBytesExt};
use std::io::Cursor;
use crate::sparse::SparseData;

/// SIMD-ready serialization logic for Sparse Data
pub struct SimdOps;

impl SimdOps {
    pub fn serialize(data: &SparseData) -> Vec<u8> {
        let mut out = Vec::new();
        out.write_u32::<BigEndian>(data.original_len).unwrap();
        out.write_u32::<BigEndian>(data.values.len() as u32).unwrap();
        
        for &v in &data.values {
            // ZigZag encode values for smaller representation
            let u = if v >= 0 { (v as u32) << 1 } else { ((v.abs() as u32) << 1) - 1 };
            out.write_u16::<BigEndian>(u as i16 as u16).unwrap();
        }
        
        for &idx in &data.indices {
            out.write_u32::<BigEndian>(idx).unwrap();
        }
        out
    }

    pub fn deserialize(bytes: &[u8]) -> SparseData {
        let mut cursor = Cursor::new(bytes);
        let original_len = cursor.read_u32::<BigEndian>().unwrap();
        let count = cursor.read_u32::<BigEndian>().unwrap() as usize;
        
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            let u = cursor.read_u16::<BigEndian>().unwrap() as u32;
            let v = if u % 2 == 0 { (u >> 1) as i32 } else { -(((u + 1) >> 1) as i32) };
            values.push(v);
        }
        
        let mut indices = Vec::with_capacity(count);
        for _ in 0..count {
            indices.push(cursor.read_u32::<BigEndian>().unwrap());
        }
        
        SparseData { values, indices, original_len }
    }
}

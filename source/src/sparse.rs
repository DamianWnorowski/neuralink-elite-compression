/// Sparse Representation using CSR (Compressed Sparse Row) format
pub struct SparseEncoder;

pub struct SparseData {
    pub values: Vec<i32>,
    pub indices: Vec<u32>,
    pub original_len: u32,
}

impl SparseEncoder {
    pub fn encode(data: &[i32]) -> SparseData {
        let mut values = Vec::new();
        let mut indices = Vec::new();
        for (i, &v) in data.iter().enumerate() {
            if v != 0 {
                values.push(v);
                indices.push(i as u32);
            }
        }
        SparseData {
            values,
            indices,
            original_len: data.len() as u32,
        }
    }

    pub fn decode(sparse: &SparseData) -> Vec<i32> {
        let mut data = vec![0i32; sparse.original_len as usize];
        for (&v, &idx) in sparse.values.iter().zip(sparse.indices.iter()) {
            if (idx as usize) < data.len() {
                data[idx as usize] = v;
            }
        }
        data
    }
}

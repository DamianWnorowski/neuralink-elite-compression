use anyhow::Result;
use std::io::{Cursor};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

/// Spike Event Coder with Vector Quantization (VQ)
pub struct SpikeCompressor {
    threshold_multiplier: f32,
    snippet_len: usize,
    codebook: Vec<Vec<i16>>,
}

impl SpikeCompressor {
    pub fn new(threshold_multiplier: f32) -> Self {
        // Initialize a pseudo-random but deterministic codebook for VQ
        // In a real system, this would be trained on neural data.
        let mut codebook = Vec::with_capacity(256);
        for i in 0..256 {
            let mut template = vec![0i16; 16];
            for j in 0..16 {
                // Generate various "spike-like" shapes
                let phase = (j as f32 / 16.0) * 2.0 * std::f32::consts::PI;
                let val = (i as f32 / 128.0 - 1.0) * (phase.sin() * 1000.0);
                template[j] = val as i16;
            }
            codebook.push(template);
        }

        Self {
            threshold_multiplier,
            snippet_len: 16,
            codebook,
        }
    }

    /// Vector Quantization: Find closest template in codebook
    fn quantize(&self, snippet: &[i16]) -> u8 {
        let mut best_idx = 0;
        let mut min_dist = f64::MAX;

        for (idx, template) in self.codebook.iter().enumerate() {
            let mut dist = 0.0;
            for i in 0..self.snippet_len {
                dist += (snippet[i] as f64 - template[i] as f64).powi(2);
            }
            if dist < min_dist {
                min_dist = dist;
                best_idx = idx as u8;
            }
        }
        best_idx
    }

    pub fn encode(&self, samples: &[i32]) -> Result<Vec<u8>> {
        let sum_sq: f64 = samples.iter().map(|&x| (x as f64).powi(2)).sum();
        let rms = (sum_sq / samples.len() as f64).sqrt();
        let threshold = rms * self.threshold_multiplier as f64;
        
        let mut events = Vec::new();
        let mut i = 0;
        let len = samples.len();

        while i < len {
            if (samples[i] as f64).abs() > threshold {
                let start = i.saturating_sub(self.snippet_len / 2);
                let mut snippet = Vec::with_capacity(self.snippet_len);
                for j in 0..self.snippet_len {
                    let idx = (start + j).min(len - 1);
                    snippet.push(samples[idx] as i16);
                }

                let template_idx = self.quantize(&snippet);
                events.push((i as u32, template_idx));
                i += self.snippet_len;
            } else {
                i += 1;
            }
        }

        let mut buffer = Vec::new();
        buffer.write_f32::<BigEndian>(rms as f32)?;
        buffer.write_u32::<BigEndian>(events.len() as u32)?;

        let mut last_ts = 0;
        for (ts, idx) in events {
            buffer.write_u32::<BigEndian>(ts - last_ts)?; // Delta timestamp
            buffer.write_u8(idx)?; // VQ index
            last_ts = ts;
        }

        Ok(buffer)
    }

    pub fn decode(&self, data: &[u8], total_samples: usize) -> Result<Vec<i32>> {
        let mut cursor = Cursor::new(data);
        let _rms = cursor.read_f32::<BigEndian>()?;
        let event_count = cursor.read_u32::<BigEndian>()?;
        
        let mut output = vec![0i32; total_samples];
        let mut current_ts = 0;

        for _ in 0..event_count {
            let delta = cursor.read_u32::<BigEndian>()?;
            let idx = cursor.read_u8()? as usize;
            current_ts += delta;

            let template = &self.codebook[idx];
            let start = (current_ts as usize).saturating_sub(self.snippet_len / 2);
            for j in 0..self.snippet_len {
                if start + j < total_samples {
                    output[start + j] = template[j] as i32;
                }
            }
        }

        Ok(output)
    }
}
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn write_wav(path: &Path, samples: &[i16], sample_rate: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("create wav");
    for &s in samples {
        writer.write_sample(s).expect("write sample");
    }
    writer.finalize().expect("finalize wav");
}

fn read_wav_samples(path: &Path) -> Vec<i16> {
    let mut reader = hound::WavReader::open(path).expect("open wav");
    reader.samples::<i16>().map(|s| s.expect("read sample")).collect()
}

fn lcg_noise(len: usize) -> Vec<i16> {
    let mut out = Vec::with_capacity(len);
    let mut state: u32 = 0x1234_5678;
    for _ in 0..len {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        let val = ((state >> 16) as i16).clamp(-30000, 30000);
        out.push(val);
    }
    out
}

fn sine_wave(len: usize, freq_hz: f32, sample_rate: u32) -> Vec<i16> {
    let mut out = Vec::with_capacity(len);
    let amp = 20000.0;
    let sr = sample_rate as f32;
    for i in 0..len {
        let t = i as f32 / sr;
        let v = (2.0 * std::f32::consts::PI * freq_hz * t).sin() * amp;
        out.push(v.round() as i16);
    }
    out
}

fn sparse_spikes(len: usize) -> Vec<i16> {
    let mut out = vec![0i16; len];
    let mut idx = 0usize;
    while idx < len {
        out[idx] = 28000;
        idx += 101;
    }
    out
}

fn temp_dir() -> PathBuf {
    let mut dir = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    dir.push(format!("ouroboros_batched_{}", nanos));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn find_exe() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_neuralink_compressor") {
        return PathBuf::from(path);
    }
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"));
    let exe = target_dir.join("debug").join("neuralink_compressor.exe");
    if exe.exists() {
        exe
    } else {
        target_dir.join("debug").join("neuralink_compressor")
    }
}

fn run_cli(args: &[&str]) {
    let exe = find_exe();
    assert!(exe.exists(), "binary not found at {:?}", exe);
    let status = Command::new(exe)
        .args(args)
        .status()
        .expect("run cli");
    assert!(status.success(), "cli failed for args: {:?}", args);
}

#[test]
fn lossless_roundtrip_batched() {
    let dir = temp_dir();
    let sample_rate = 1000;
    let cases = [
        ("sine", sine_wave(4096, 7.0, sample_rate)),
        ("noise", lcg_noise(4096)),
        ("spikes", sparse_spikes(4096)),
    ];
    let block_sizes = [32usize, 128usize];
    for (name, samples) in cases {
        let in_path = dir.join(format!("{}_in.wav", name));
        write_wav(&in_path, &samples, sample_rate);
        for block_size in block_sizes {
            let out_path = dir.join(format!("{}_lossless_{}.neur", name, block_size));
            let recon_path = dir.join(format!("{}_lossless_{}.wav", name, block_size));
            run_cli(&[
                "encode",
                in_path.to_str().unwrap(),
                out_path.to_str().unwrap(),
                "--mode",
                "lossless",
                "--block-size",
                &block_size.to_string(),
            ]);
            run_cli(&[
                "decode",
                out_path.to_str().unwrap(),
                recon_path.to_str().unwrap(),
            ]);
            let recon = read_wav_samples(&recon_path);
            assert_eq!(samples, recon, "lossless mismatch for {}", name);
            let orig_size = fs::metadata(&in_path).expect("orig metadata").len();
            let comp_size = fs::metadata(&out_path).expect("comp metadata").len();
            assert!(comp_size > 0 && orig_size > 0, "sizes must be nonzero");
        }
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn events_roundtrip_batched() {
    let dir = temp_dir();
    let sample_rate = 1000;
    let cases = [
        ("sine", sine_wave(4096, 11.0, sample_rate)),
        ("noise", lcg_noise(4096)),
        ("spikes", sparse_spikes(4096)),
    ];
    for (name, samples) in cases {
        let in_path = dir.join(format!("{}_in.wav", name));
        write_wav(&in_path, &samples, sample_rate);
        let out_path = dir.join(format!("{}_events.neur", name));
        let recon_path = dir.join(format!("{}_events.wav", name));
        run_cli(&[
            "encode",
            in_path.to_str().unwrap(),
            out_path.to_str().unwrap(),
            "--mode",
            "events",
            "--threshold",
            "4.0",
        ]);
        run_cli(&[
            "decode",
            out_path.to_str().unwrap(),
            recon_path.to_str().unwrap(),
        ]);
        let recon = read_wav_samples(&recon_path);
        assert_eq!(samples.len(), recon.len(), "length mismatch for {}", name);
        let orig_size = fs::metadata(&in_path).expect("orig metadata").len();
        let comp_size = fs::metadata(&out_path).expect("comp metadata").len();
        assert!(comp_size > 0 && orig_size > 0, "sizes must be nonzero");
    }
    let _ = fs::remove_dir_all(&dir);
}

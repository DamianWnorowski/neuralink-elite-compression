use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Deserialize)]
struct MultipovEntry {
    file: String,
    threshold: f64,
    ratio: f64,
    latency: f64,
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

fn temp_dir() -> PathBuf {
    let mut dir = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    dir.push(format!("ouroboros_multipov_{}", nanos));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn read_wav_samples(path: &Path) -> Vec<i16> {
    let mut reader = hound::WavReader::open(path).expect("open wav");
    reader.samples::<i16>().map(|s| s.expect("read sample")).collect()
}

fn multipov_root() -> PathBuf {
    let base_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("base dir")
        .to_path_buf();
    base_dir.join("multipov_results")
}

fn load_report(path: &Path) -> Vec<MultipovEntry> {
    let data = fs::read_to_string(path).expect("read multipov report");
    serde_json::from_str(&data).expect("parse multipov report")
}

fn find_reference_wav(root: &Path, file: &str) -> PathBuf {
    let restored = root.join(format!("{}.restored.wav", file));
    if restored.exists() {
        return restored;
    }
    let direct = root.join(file);
    if direct.exists() {
        return direct;
    }
    panic!("missing reference wav for {}", file);
}

fn group_by_file(entries: &[MultipovEntry]) -> HashMap<String, Vec<&MultipovEntry>> {
    let mut map: HashMap<String, Vec<&MultipovEntry>> = HashMap::new();
    for entry in entries {
        map.entry(entry.file.clone()).or_default().push(entry);
    }
    for values in map.values_mut() {
        values.sort_by(|a, b| a.threshold.partial_cmp(&b.threshold).unwrap());
    }
    map
}

#[test]
fn multipov_ratios_match_report() {
    let root = multipov_root();
    let report_path = root.join("multipov_report.json");
    assert!(report_path.exists(), "missing multipov_report.json");
    let entries = load_report(&report_path);
    assert!(!entries.is_empty(), "empty multipov report");

    let mut ref_cache: HashMap<String, u64> = HashMap::new();
    for entry in entries {
        assert!(entry.latency > 0.0, "latency must be positive");
        let threshold = format!("{:.1}", entry.threshold);
        let elite_path = root.join(format!("{}.t{}.elite", entry.file, threshold));
        assert!(elite_path.exists(), "missing elite file {:?}", elite_path);
        let ref_size = ref_cache.entry(entry.file.clone()).or_insert_with(|| {
            let ref_wav = find_reference_wav(&root, &entry.file);
            fs::metadata(ref_wav).expect("reference metadata").len()
        });
        let elite_size = fs::metadata(&elite_path).expect("elite metadata").len();
        assert!(elite_size > 0, "elite size must be nonzero");
        let ratio = *ref_size as f64 / elite_size as f64;
        let diff = (ratio - entry.ratio).abs();
        let rel = diff / entry.ratio.max(1.0);
        assert!(
            rel <= 0.001 || diff <= 0.05,
            "ratio mismatch for {} t{}: report={}, got={}",
            entry.file,
            threshold,
            entry.ratio,
            ratio
        );
    }
}

#[test]
fn multipov_threshold_policy() {
    let root = multipov_root();
    let report_path = root.join("multipov_report.json");
    assert!(report_path.exists(), "missing multipov_report.json");
    let entries = load_report(&report_path);
    let grouped = group_by_file(&entries);
    let expected = [4.0, 6.0, 8.0, 10.0];
    for (file, values) in grouped {
        let thresholds: Vec<f64> = values.iter().map(|v| v.threshold).collect();
        assert_eq!(
            thresholds.len(),
            expected.len(),
            "unexpected threshold count for {}",
            file
        );
        for (got, exp) in thresholds.iter().zip(expected.iter()) {
            assert!(
                (got - exp).abs() <= 0.0001,
                "unexpected threshold for {}: got {}, expected {}",
                file,
                got,
                exp
            );
        }
        for window in values.windows(2) {
            let prev = window[0];
            let next = window[1];
            assert!(
                next.ratio + 1e-6 >= prev.ratio,
                "ratio decreased for {} from t{} to t{}",
                file,
                prev.threshold,
                next.threshold
            );
        }
    }
}

#[test]
fn multipov_lossless_roundtrip_matches_restored() {
    let root = multipov_root();
    let report_path = root.join("multipov_report.json");
    assert!(report_path.exists(), "missing multipov_report.json");
    let entries = load_report(&report_path);
    let mut files: Vec<String> = entries.into_iter().map(|e| e.file).collect();
    files.sort();
    files.dedup();
    let dir = temp_dir();
    for file in files {
        let lossless = root.join(format!("{}.lossless", file));
        let restored = root.join(format!("{}.restored.wav", file));
        assert!(lossless.exists(), "missing lossless file {:?}", lossless);
        assert!(restored.exists(), "missing restored wav {:?}", restored);
        let out = dir.join(format!("{}.decoded.wav", file));
        run_cli(&[
            "decode",
            lossless.to_str().unwrap(),
            out.to_str().unwrap(),
        ]);
        let restored_samples = read_wav_samples(&restored);
        let decoded_samples = read_wav_samples(&out);
        assert_eq!(
            restored_samples, decoded_samples,
            "lossless mismatch for {}",
            file
        );
    }
    let _ = fs::remove_dir_all(&dir);
}

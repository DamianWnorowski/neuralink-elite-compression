import os
import subprocess
import time
import hashlib
import json

# Configuration
COMPRESSOR = r"C:\Users\Ouroboros\Desktop\FINAL_NEURALINK_SUBMISSION\neuralink_compressor.exe"
TEST_FILES = [
    r"C:\Users\Ouroboros\ouroboros-monorepo\projects\demo_web\dist\samples\neuralink_style.wav",
    r"C:\Users\Ouroboros\ouroboros-monorepo\projects\demo_web\dist\samples\real_neural_signal.wav",
    r"C:\Users\Ouroboros\ouroboros-monorepo\projects\demo_web\dist\samples\synthetic_neural.wav",
    r"C:\Users\Ouroboros\ouroboros-monorepo\projects\demo_web\public\samples\physionet_eeg.wav",
    r"C:\Users\Ouroboros\ouroboros-monorepo\projects\demo_web\public\samples\physionet_motor_s002.wav"
]
OUTPUT_DIR = r"C:\Users\Ouroboros\Desktop\FINAL_NEURALINK_SUBMISSION\multipov_results"

if not os.path.exists(OUTPUT_DIR):
    os.makedirs(OUTPUT_DIR)

def get_md5(fname):
    hash_md5 = hashlib.md5()
    with open(fname, "rb") as f:
        for chunk in iter(lambda: f.read(4096), b""):
            hash_md5.update(chunk)
    return hash_md5.hexdigest()

def run_multipov_test():
    results = []
    print(f"{'File':<25} | {'Mode':<8} | {'Param':<10} | {'Ratio':<10} | {'Latency':<8} | {'Result'}")
    print("-" * 85)

    for test_file in TEST_FILES:
        if not os.path.exists(test_file): continue
        base_name = os.path.basename(test_file)
        orig_size = os.path.getsize(test_file)
        orig_md5 = get_md5(test_file)

        # POV 1: Lossless Baseline
        out_lossless = os.path.join(OUTPUT_DIR, f"{base_name}.lossless")
        start = time.perf_counter()
        subprocess.run([COMPRESSOR, "encode", test_file, out_lossless, "--mode", "lossless", "--block-size", "1024"], capture_output=True)
        t_lossless = (time.perf_counter() - start) * 1000
        r_lossless = orig_size / os.path.getsize(out_lossless)
        
        # Verify
        restored = os.path.join(OUTPUT_DIR, f"{base_name}.restored.wav")
        subprocess.run([COMPRESSOR, "decode", out_lossless, restored], capture_output=True)
        integrity = "BIT-PERFECT" if get_md5(restored) == orig_md5 else "FAILED"
        print(f"{base_name[:25]:<25} | LOSSLESS | B:1024     | {r_lossless:>9.2f}x | {t_lossless:>6.1f}ms | {integrity}")

        # POV 2: Elite Scaling (Threshold Sweep)
        for threshold in [4.0, 6.0, 8.0, 10.0]:
            out_elite = os.path.join(OUTPUT_DIR, f"{base_name}.t{threshold}.elite")
            start = time.perf_counter()
            subprocess.run([COMPRESSOR, "encode", test_file, out_elite, "--mode", "events", "--threshold", str(threshold)], capture_output=True)
            t_elite = (time.perf_counter() - start) * 1000
            r_elite = orig_size / os.path.getsize(out_elite)
            print(f"{'':<25} | ELITE    | T:{threshold:<8} | {r_elite:>9.2f}x | {t_elite:>6.1f}ms | SEMANTIC")
            
            results.append({
                "file": base_name,
                "threshold": threshold,
                "ratio": r_elite,
                "latency": t_elite
            })

    # Save to JSON for report
    with open(os.path.join(OUTPUT_DIR, "multipov_report.json"), "w") as f:
        json.dump(results, f, indent=4)

if __name__ == "__main__":
    run_multipov_test()

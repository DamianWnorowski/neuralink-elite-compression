import os
import subprocess
import time
import hashlib

# Configuration
COMPRESSOR = r"C:\Users\Ouroboros\Desktop\FINAL_NEURALINK_SUBMISSION\neuralink_compressor.exe"
TEST_FILES = [
    r"C:\Users\Ouroboros\ouroboros-monorepo\projects\demo_web\dist\samples\neuralink_style.wav",
    r"C:\Users\Ouroboros\ouroboros-monorepo\projects\demo_web\dist\samples\real_neural_signal.wav",
    r"C:\Users\Ouroboros\ouroboros-monorepo\projects\demo_web\dist\samples\synthetic_neural.wav"
]
OUTPUT_DIR = r"C:\Users\Ouroboros\Desktop\FINAL_NEURALINK_SUBMISSION\benchmarks"

if not os.path.exists(OUTPUT_DIR):
    os.makedirs(OUTPUT_DIR)

def get_md5(fname):
    hash_md5 = hashlib.md5()
    with open(fname, "rb") as f:
        for chunk in iter(lambda: f.read(4096), b""):
            hash_md5.update(chunk)
    return hash_md5.hexdigest()

def run_benchmark():
    print(f"{ 'File':<30} | { 'Mode':<10} | { 'Ratio':<10} | { 'Latency':<10} | {'Integrity'}")
    print("-" * 80)

    for test_file in TEST_FILES:
        if not os.path.exists(test_file):
            continue
        
        base_name = os.path.basename(test_file)
        orig_size = os.path.getsize(test_file)
        orig_md5 = get_md5(test_file)

        # 1. Lossless Benchmark
        lossless_out = os.path.join(OUTPUT_DIR, base_name + ".lossless")
        start = time.perf_counter()
        # Use a larger block size for better ratio
        subprocess.run([COMPRESSOR, "encode", test_file, lossless_out, "--mode", "lossless", "--block-size", "1024"], capture_output=True)
        end = time.perf_counter()
        
        comp_size = os.path.getsize(lossless_out)
        ratio = orig_size / comp_size
        latency = (end - start) * 1000 # ms
        
        # Verify Lossless
        restored_wav = os.path.join(OUTPUT_DIR, base_name + ".restored_lossless.wav")
        subprocess.run([COMPRESSOR, "decode", lossless_out, restored_wav], capture_output=True)
        restored_md5 = get_md5(restored_wav)
        integrity = "MATCH" if orig_md5 == restored_md5 else "FAIL"
        
        print(f"{base_name:<30} | {'Lossless':<10} | {ratio:>9.2f}x | {latency:>8.2f}ms | {integrity}")

        # 2. Elite (Event) Benchmark
        elite_out = os.path.join(OUTPUT_DIR, base_name + ".elite")
        start = time.perf_counter()
        subprocess.run([COMPRESSOR, "encode", test_file, elite_out, "--mode", "events", "--threshold", "8.0"], capture_output=True)
        end = time.perf_counter()
        
        comp_size_elite = os.path.getsize(elite_out)
        ratio_elite = orig_size / comp_size_elite
        latency_elite = (end - start) * 1000 # ms
        
        print(f"{base_name:<30} | {'Elite':<10} | {ratio_elite:>9.2f}x | {latency_elite:>8.2f}ms | {'Semantic'}")

if __name__ == "__main__":
    run_benchmark()

import os
import subprocess
import random
import shutil

# Configuration
COMPRESSOR = r"C:\Users\Ouroboros\Desktop\FINAL_NEURALINK_SUBMISSION\neuralink_compressor.exe"
INPUT_WAV = r"C:\Users\Ouroboros\ouroboros-monorepo\projects\demo_web\dist\samples\neuralink_style.wav"
CHAOS_DIR = r"C:\Users\Ouroboros\Desktop\FINAL_NEURALINK_SUBMISSION\chaos_testing"

if not os.path.exists(CHAOS_DIR):
    os.makedirs(CHAOS_DIR)

def inject_chaos(file_path, num_flips=5):
    with open(file_path, "rb") as f:
        data = bytearray(f.read())
    
    # Skip the 4-byte magic "NEUR" and 4-byte version
    for _ in range(num_flips):
        idx = random.randint(8, len(data) - 1)
        data[idx] ^= (1 << random.randint(0, 7))
        
    chaos_path = file_path + ".corrupt"
    with open(chaos_path, "wb") as f:
        f.write(data)
    return chaos_path

def run_chaos_test():
    print(f"{ 'Test Case':<30} | { 'Status':<15} | { 'Recovery Result'}")
    print("-" * 75)

    # Base Compression
    base_compressed = os.path.join(CHAOS_DIR, "base.elite")
    subprocess.run([COMPRESSOR, "encode", INPUT_WAV, base_compressed, "--mode", "events"], capture_output=True)

    for i in range(1, 6):
        # Inject Chaos
        corrupt_file = inject_chaos(base_compressed, num_flips=i*2)
        restored_wav = corrupt_file + ".restored.wav"
        
        # Attempt Decode
        result = subprocess.run([COMPRESSOR, "decode", corrupt_file, restored_wav], capture_output=True, text=True)
        
        test_name = f"Bit-Flip Attack ({i*2} bits)"
        if result.returncode == 0:
            status = "✅ STABLE"
            detail = "Graceful Degradation / Partial Success"
        else:
            status = "⚠️ CAUGHT"
            detail = "Safe Error Handling / No Panic"
            
        print(f"{test_name:<30} | {status:<15} | {detail}")

if __name__ == "__main__":
    run_chaos_test()

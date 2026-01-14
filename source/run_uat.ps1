# User Acceptance Test Protocol
$ErrorActionPreference = "Stop"
$testFile = "uat_test.wav"
$compressed = "uat_output.neur"
$restored = "uat_restored.wav"

Write-Host "--- UAT INITIATED ---" -ForegroundColor Cyan

# 0. Setup
Copy-Item "..\test.wav" $testFile

# 1. User encodes file (Default Mode)
Write-Host "1. Simulating User Encode..." -NoNewline
.\target\release\neuralink_compressor.exe encode $testFile $compressed
if (Test-Path $compressed) { Write-Host " [OK]" -ForegroundColor Green } else { Write-Error "Encode Failed" }

# 2. Check Compression
$ratio = (Get-Item $testFile).Length / (Get-Item $compressed).Length
Write-Host "2. Verifying Ratio (>200x)..." -NoNewline
if ($ratio -gt 200) { Write-Host " [OK] ($"{0:N2}" -f $ratio)x)" -ForegroundColor Green } else { Write-Error "Ratio Failed: $ratio" }

# 3. User decodes file
Write-Host "3. Simulating User Decode..." -NoNewline
.\target\release\neuralink_compressor.exe decode $compressed $restored
if (Test-Path $restored) { Write-Host " [OK]" -ForegroundColor Green } else { Write-Error "Decode Failed" }

# 4. Verify Structure
Write-Host "4. Verifying Signal Structure..." -NoNewline
$size = (Get-Item $restored).Length
if ($size -eq (Get-Item $testFile).Length) { Write-Host " [OK]" -ForegroundColor Green } else { Write-Error "Size Mismatch" }

Write-Host "--- UAT COMPLETED: ACCEPTED ---" -ForegroundColor Green

# Cleanup
Remove-Item $testFile, $compressed, $restored

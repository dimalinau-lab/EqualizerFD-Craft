# Build and package Equalizer FD CRAFT v1.0.0
$ErrorActionPreference = "Stop"

Write-Host ">>> [1/6] Building release binaries via Cargo..."
cargo build --release --manifest-path crates/resona_apo/Cargo.toml
cargo build --release --manifest-path crates/eq_apo_cli/Cargo.toml

Write-Host ">>> [2/6] Copying release binaries to root directory..."
Stop-Process -Name "EqualizerFD-Craft", "EqualizerTray" -Force -ErrorAction SilentlyContinue
Start-Sleep -Milliseconds 500
Copy-Item "crates\eq_apo_cli\target\release\EqualizerFD-Craft.exe" -Destination "EqualizerFD-Craft.exe" -Force
Copy-Item "crates\resona_apo\target\release\resona_apo.dll" -Destination "ResonaAPO.dll" -Force

Write-Host ">>> [3/6] Compiling Inno Setup installers..."
if (-not (Test-Path "dist")) {
    New-Item -ItemType Directory -Path "dist" | Out-Null
}

& "D:\Inno Setup 6\ISCC.exe" "installer.iss"
& "D:\Inno Setup 6\ISCC.exe" "installer_update.iss"

Write-Host ">>> [4/6] Packaging EqualizerFD-Craft-Update.zip..."
$updateZipPath = "dist\EqualizerFD-Craft-Update.zip"
if (Test-Path $updateZipPath) { Remove-Item $updateZipPath -Force }

$stagingUpdate = "target\staging_update"
if (Test-Path $stagingUpdate) { Remove-Item $stagingUpdate -Recurse -Force }
New-Item -ItemType Directory -Path $stagingUpdate -Force | Out-Null
Copy-Item "EqualizerFD-Craft.exe" -Destination $stagingUpdate
Copy-Item "EqualizerFD-Craft.ico" -Destination $stagingUpdate
Copy-Item "EqualizerTray.exe" -Destination $stagingUpdate
Copy-Item "ResonaAPO.dll" -Destination $stagingUpdate
Copy-Item "web" -Destination $stagingUpdate -Recurse

Compress-Archive -Path "$stagingUpdate\*" -DestinationPath $updateZipPath -CompressionLevel Optimal
Remove-Item $stagingUpdate -Recurse -Force

Write-Host ">>> [5/6] Packaging EqualizerFD-Craft-v1.0.0-Portable.zip..."
$portableZipPath = "dist\EqualizerFD-Craft-v1.0.0-Portable.zip"
if (Test-Path $portableZipPath) { Remove-Item $portableZipPath -Force }

$stagingPortable = "target\staging_portable"
if (Test-Path $stagingPortable) { Remove-Item $stagingPortable -Recurse -Force }
New-Item -ItemType Directory -Path $stagingPortable -Force | Out-Null
Copy-Item "config.example.txt" -Destination $stagingPortable
Copy-Item "EqualizerFD-Craft.exe" -Destination $stagingPortable
Copy-Item "EqualizerFD-Craft.ico" -Destination $stagingPortable
Copy-Item "EqualizerTray.exe" -Destination $stagingPortable
Copy-Item "LICENSE" -Destination $stagingPortable
Copy-Item "README.md" -Destination $stagingPortable
Copy-Item "ResonaAPO.dll" -Destination $stagingPortable
Copy-Item "tools\vbcable" -Destination "$stagingPortable\vbcable" -Recurse
Copy-Item "web" -Destination $stagingPortable -Recurse

Compress-Archive -Path "$stagingPortable\*" -DestinationPath $portableZipPath -CompressionLevel Optimal
Remove-Item $stagingPortable -Recurse -Force

Write-Host "`n>>> [6/6] Build complete! Artifacts in dist/:"
Get-Item "dist\EqualizerFD-Craft-Setup-v1.0.0.exe", "dist\EqualizerFD-Craft-Update-Installer.exe", "dist\EqualizerFD-Craft-Update.zip", "dist\EqualizerFD-Craft-v1.0.0-Portable.zip" | Select-Object Name, Length, LastWriteTime | Format-Table

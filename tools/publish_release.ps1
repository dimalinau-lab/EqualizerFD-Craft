# Publish release v0.1.3-BETA to GitHub
param (
    [string]$TagName = "v0.1.3-BETA",
    [string]$ReleaseName = "Equalizer FD CRAFT v0.1.3-BETA"
)

$ErrorActionPreference = "Stop"

Write-Host ">>> [1/5] Extracting GitHub credential from Git Credential Manager..."
$inputCred = "protocol=https`nhost=github.com`n`n"
$out = $inputCred | git credential fill
$token = ($out | Select-String "password=(.+)").Matches.Groups[1].Value

if (-not $token) {
    Write-Error "Could not retrieve GitHub token."
    exit 1
}

$repo = "dimalinau-lab/EqualizerFD-Craft"
$headers = @{
    "Authorization" = "token $token"
    "User-Agent" = "EqualizerFD-Craft-Uploader"
    "Accept" = "application/vnd.github.v3+json"
}

Write-Host ">>> [2/5] Checking if release '$TagName' exists..."
$release = $null
try {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/tags/$TagName" -Headers $headers -Method Get
    Write-Host "Release already exists (ID: $($release.id))."
} catch {
    Write-Host "Release not found. Creating new release for tag '$TagName'..."
}

$releaseBody = @"
# Equalizer FD CRAFT v0.1.3-BETA

## 🎛️ Полный набор эффектов Roland BOSS RC-505 Mk2 (50 эффектов)
- Реализованы все **50 аудио-эффектов** знаменитой луп-станции BOSS RC-505 Mk2:
  - **DYNAMICS/FILTER (10)**: LPF, HPF, BPF, COMB FILTER, TRACK EQ, ISOLATOR, VOCODER, REVERSE, RING MOD, COMPRESSOR
  - **PITCH/MODULATION (10)**: PITCH BEND, HARMONIST, AUTO VOCODER, CHORUS, FLANGER, PHASER, TREMOLO, PAN, VIBRATO, OCTAVE
  - **DISTORTION/WARMTH (10)**: OVERDRIVE, DISTORTION, FUZZ, AMP SIM, BIT CRUSH, SATURATION, CRUNCH, TUBE DRIVE, OCTA FUZZ, RADIO
  - **DELAY/ECHO (10)**: TAPE ECHO, DIGITAL DELAY, PAN DELAY, REVERSE DELAY, DUCK DELAY, WARP, DUB ECHO, TIME WARP, SLICER, REPEAT THIN
  - **REVERB/SPECIAL (10)**: ROOM, HALL, PLATE, SPRING, SHIMMER, VINYL LO-FI, LOOPER GRAIN, BEAT REPEAT, FREEZE, MORPHING FILTER
- Интерактивное модальное окно каталога с мгновенным переключением и регулировкой параметров в реальном времени.

## 🎨 Новый минималистичный 4K интерфейс и иконка
- Новая ультра-четкая 4K монохромная векторная иконка для всех масштабов Windows (100%, 125%, 150%, 200%, 400%).
- Исправлено отображение иконки в превью миниатюры панели задач Windows (`SetClassLongPtrW` + сброс кеша фавиконов WebView2/Edge).

## 🪟 Надежный системный трей Windows
- Полная реставрация системного трея: надежное разворачивание окна из свернутого состояния с захватом фокуса Win32.
- Управление мостом WASAPI, запуск с правами администратора без вылета UAC, корректный выход из приложения.

## 📦 Варианты установки и скачивания
1. **`EqualizerFD-Craft-Setup-v0.1.3-BETA.exe`** (165.68 MB):
   - **Полный офлайн установщик Inno Setup** со всеми встроенными **98 профессиональными 64-битными VST плагинами** (ReaPlugs, Blue Cat, Kuassa и др.).
2. **`EqualizerFD-Craft-Update-Installer.exe`** (3.10 MB):
   - **Быстрое обновление (рекомендуется)**: легкий установщик Inno Setup. Обновляет исполняемые файлы программы, системный трей и веб-интерфейс, сохраняя все ваши пресеты и плагины.
3. **`EqualizerFD-Craft-Update.zip`** (1.85 MB):
   - **Быстрое обновление (ZIP-архив)** без повторного скачивания плагинов.
4. **`EqualizerFD-Craft-v0.1.3-BETA-Portable.zip`** (3.20 MB):
   - Портативная версия без инсталляции.
"@

if (-not $release) {
    $createPayload = @{
        tag_name = $TagName
        target_commitish = "main"
        name = $ReleaseName
        body = $releaseBody
        draft = $false
        prerelease = $true
    } | ConvertTo-Json -Depth 5

    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases" -Headers $headers -Method Post -Body $createPayload -ContentType "application/json; charset=utf-8"
    Write-Host "Created release with ID: $($release.id)"
} else {
    Write-Host "Updating release notes for existing release..."
    $updatePayload = @{
        name = $ReleaseName
        body = $releaseBody
        prerelease = $true
    } | ConvertTo-Json -Depth 5
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/$($release.id)" -Headers $headers -Method Patch -Body $updatePayload -ContentType "application/json; charset=utf-8"
}

$releaseId = $release.id
Write-Host "Release URL: $($release.html_url)"

Write-Host ">>> [3/5] Listing existing assets in release..."
$existingAssets = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/$releaseId/assets" -Headers $headers -Method Get

$filesToUpload = @(
    @{ Name = "EqualizerFD-Craft-Setup-v0.1.3-BETA.exe"; Path = "dist\EqualizerFD-Craft-Setup-v0.1.3-BETA.exe"; ContentType = "application/vnd.microsoft.portable-executable" },
    @{ Name = "EqualizerFD-Craft-Update-Installer.exe"; Path = "dist\EqualizerFD-Craft-Update-Installer.exe"; ContentType = "application/vnd.microsoft.portable-executable" },
    @{ Name = "EqualizerFD-Craft-Update.zip"; Path = "dist\EqualizerFD-Craft-Update.zip"; ContentType = "application/zip" },
    @{ Name = "EqualizerFD-Craft-v0.1.3-BETA-Portable.zip"; Path = "dist\EqualizerFD-Craft-v0.1.3-BETA-Portable.zip"; ContentType = "application/zip" }
)

Write-Host ">>> [4/5] Uploading release assets..."

foreach ($file in $filesToUpload) {
    $filePath = Resolve-Path $file.Path
    $fileName = $file.Name
    $cType = $file.ContentType
    $fileSizeMB = [math]::Round((Get-Item $filePath).Length / 1MB, 2)

    Write-Host "`nProcessing $fileName ($fileSizeMB MB)..."

    # Check if already present
    if ($existingAssets) {
        $found = $existingAssets | Where-Object { $_.name -eq $fileName }
        if ($found) {
            $localLen = (Get-Item $filePath).Length
            if ($found.size -eq $localLen -and $found.state -eq "uploaded") {
                Write-Host "Asset $fileName already exists and size matches ($($found.size) bytes). Skipping upload."
                continue
            }
            Write-Host "Asset $fileName exists with different size (remote: $($found.size), local: $localLen). Deleting previous version..."
            Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/assets/$($found.id)" -Headers $headers -Method Delete
            Start-Sleep -Seconds 1
        }
    }

    $uploadUri = "https://uploads.github.com/repos/$repo/releases/$releaseId/assets?name=$fileName"
    Write-Host "Uploading to GitHub: $uploadUri ..."

    $curlArgs = @(
        "-s", "-S",
        "-X", "POST",
        "-H", "Authorization: token $token",
        "-H", "Content-Type: $cType",
        "-H", "User-Agent: EqualizerFD-Craft-Uploader",
        "-H", "Accept: application/vnd.github.v3+json",
        "--data-binary", "@$filePath",
        "$uploadUri"
    )

    $result = & curl.exe @curlArgs
    try {
        $jsonResult = $result | ConvertFrom-Json
        if ($jsonResult.id) {
            Write-Host "Successfully uploaded $fileName (Asset ID: $($jsonResult.id), Size: $($jsonResult.size) bytes)"
        } else {
            Write-Host "Response: $result"
        }
    } catch {
        Write-Host "Raw response: $result"
    }
}

Write-Host "`n>>> [5/5] Final Verification of Release Assets..."
$finalAssets = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/$releaseId/assets" -Headers $headers -Method Get
$finalAssets | Select-Object name, size, download_count, browser_download_url | Format-Table

Write-Host "Release publishing completed successfully!"
Write-Host "Release URL: $($release.html_url)"

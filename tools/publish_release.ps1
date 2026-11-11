# Publish release v1.3.0 to GitHub (Official Release)
param (
    [string]$TagName = "v1.3.0",
    [string]$ReleaseName = "Equalizer FD CRAFT v1.3.0 (DeepFilterNet3 LL AI Noise Suppression & Monitoring Persistence)"
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
# Equalizer FD CRAFT v1.3.0 — AI-шумоподавление DeepFilterNet3 LL и сохранение прослушки

В релизе **v1.3.0** внедрён нейросетевой движок шумоподавления нового поколения **DeepFilterNet3 LL (48 kHz)** с модулем защиты речи `SpeechGuard`, общий очищенный поток для прослушки без дублирующей нагрузки на CPU и сохранение настроек мониторинга между перезапусками.

---

## Главные нововведения и исправления

### 1. Нейросетевое AI-шумоподавление DeepFilterNet3 LL (48 kHz) + SpeechGuard
- Полный переход на модель **DeepFilterNet3 Low-Latency (`DeepFilterNet3_ll_onnx.tar.gz`)** на частоте `48000 Hz` с zero-lookahead и фоновой неблокирующей загрузкой.
- Встроен модуль **`SpeechGuard`** с адаптивным `noise_floor_rms`, защитой от щелчков на атаке и динамическим порогом `min_db_thresh`.
- Пользовательский `Preamp` перенесён строго **после** AI-шумоподавления и `SpeechGuard` (`80 Hz` HPF остаётся до AI), а `Include:` конфиги разворачиваются автоматически.
- Автоматический fallback на RNNoise (`nnnoiseless`), если файл модели временно отсутствует.

### 2. Единый очищенный поток прослушки (Zero Duplicate AI CPU)
- Мониторинг в наушниках подключается к уже очищенному `48 kHz` потоку VB-Cable моста через `SHARED_TAP_HUB` без повторного прогона нейросети на CPU.

### 3. Сохранение настроек прослушки и интерфейс v1.3.0
- Выбранное устройство мониторинга, активность (`[В ЭФИРЕ]` / `[ВЫКЛ]`) и громкость сохраняются в `settings.json` и автоматически восстанавливаются после перезапуска.
- Добавлен тонкий монохромный ползунок громкости мониторинга в шапке (`#toolbarMonitorVol`).
- Синхронизирована версия приложения, автоапдейтера и окна «О программе» через `CARGO_PKG_VERSION` (`1.3.0`).

---

## Варианты загрузки
1. **`EqualizerFD-Craft-Setup-v1.3.0.exe`**:
   - Полный офлайн установщик Inno Setup со всеми 98 студийными 64-битными VST-плагинами (ReaPlugs, Blue Cat, Kuassa) и AI-моделью DeepFilterNet3 LL.
2. **`EqualizerFD-Craft-Update-Installer.exe`**:
   - Быстрое обновление (рекомендуется): обновляет исполняемые файлы, AI-модель DeepFilterNet3 LL, системный трей и веб-интерфейс, сохраняя ваши пресеты и установленные VST-плагины.
3. **`EqualizerFD-Craft-Update.zip`**:
   - Быстрое обновление в ZIP-архиве (включая AI-модель, без VST-плагинов).
4. **`EqualizerFD-Craft-v1.3.0-Portable.zip`**:
   - Портативная версия без необходимости установки.
5. **`EqualizerFD-Craft.exe`** & **`ResonaAPO.dll`**:
   - Отдельные скомпилированные бинарники для прямой замены.
"@

if (-not $release) {
    $createPayload = @{
        tag_name = $TagName
        target_commitish = "main"
        name = $ReleaseName
        body = $releaseBody
        draft = $false
        prerelease = $false
    } | ConvertTo-Json -Depth 5

    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases" -Headers $headers -Method Post -Body $createPayload -ContentType "application/json; charset=utf-8"
    Write-Host "Created release with ID: $($release.id)"
} else {
    Write-Host "Updating release notes for existing release..."
    $updatePayload = @{
        name = $ReleaseName
        body = $releaseBody
        prerelease = $false
    } | ConvertTo-Json -Depth 5
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/$($release.id)" -Headers $headers -Method Patch -Body $updatePayload -ContentType "application/json; charset=utf-8"
}

$releaseId = $release.id
Write-Host "Release URL: $($release.html_url)"

Write-Host ">>> [3/5] Listing existing assets in release..."
$existingAssets = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/$releaseId/assets" -Headers $headers -Method Get

$filesToUpload = @(
    @{ Name = "EqualizerFD-Craft-Setup-v1.3.0.exe"; Path = "dist\EqualizerFD-Craft-Setup-v1.3.0.exe"; ContentType = "application/vnd.microsoft.portable-executable" },
    @{ Name = "EqualizerFD-Craft-Update-Installer.exe"; Path = "dist\EqualizerFD-Craft-Update-Installer.exe"; ContentType = "application/vnd.microsoft.portable-executable" },
    @{ Name = "EqualizerFD-Craft-Update.zip"; Path = "dist\EqualizerFD-Craft-Update.zip"; ContentType = "application/zip" },
    @{ Name = "EqualizerFD-Craft-v1.3.0-Portable.zip"; Path = "dist\EqualizerFD-Craft-v1.3.0-Portable.zip"; ContentType = "application/zip" },
    @{ Name = "EqualizerFD-Craft.exe"; Path = "EqualizerFD-Craft.exe"; ContentType = "application/vnd.microsoft.portable-executable" },
    @{ Name = "ResonaAPO.dll"; Path = "ResonaAPO.dll"; ContentType = "application/vnd.microsoft.portable-executable" }
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

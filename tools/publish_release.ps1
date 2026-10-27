# Publish release v1.0.0 to GitHub (Official Release)
param (
    [string]$TagName = "v1.0.0",
    [string]$ReleaseName = "Equalizer FD CRAFT v1.0.0 (Official Release)"
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
# Equalizer FD CRAFT v1.0.0 — Официальный релиз

Первый стабильный релиз Equalizer FD CRAFT — выход из статуса Beta!

## Главные нововведения и улучшения

### 1. Нейросетевое AI-шумоподавление (RNNoise / Deep Learning)
- Рекуррентная нейросеть GRU в реальном времени устраняет фоновые шумы, клики клавиатуры и мыши, гул вентиляторов без искажения голоса.
- Безопасная реализация на чистом Rust (`nnnoiseless`) без нестабильных сторонних библиотек.
- Stage 0 обработка до параметрики и VST.
- Динамический индикатор Voice Activity Detection (VAD) с отображением вероятности речи в процентах.
- Ползунок глубины подавления (0–100%) и быстрые пресеты (50% Мягкий, 85% Оптимальный, 100% Максимальный).

### 2. Интерактивный параметрический эквалайзер на графике АЧХ (FabFilter-Style)
- Живое перетаскивание узлов фильтров мышью прямо на кривой АЧХ (частота и усиление).
- Регулировка добротности (Q) колесиком мыши с визуализацией полосы пропускания под курсором.
- Фиксация узла кликом мыши: карточка действий закрепляется на графике и не исчезает при движении курсора.
- Быстрое переключение типов фильтров: PK (колокол), LS (low-shelf), HS (high-shelf), HP (high-pass), LP (low-pass), NO (notch).
- Быстрые действия: Bypass фильтра, инвертирование усиления, удаление фильтра.
- Спектроанализатор реального времени RTA 60 FPS с переключением режима и откреплением в отдельное окно.

### 3. Интерфейс и дизайн
- Полностью удалены лишние эмодзи — строгий профессиональный стиль студийного оборудования.
- Две выверенные темы: монохромная черная темная тема (OLED Black) и нативная светлая тема в стиле Windows Desktop.
- Оптимизированные области клика для быстрого и удобного управления параметрами.

### 4. Встроенный авто-апдейтер v1.0.0
- Быстрое фоновое обновление без перекачивания плагинов (~3 МБ).
- Полный офлайн установщик со всеми 98 студийными VST-плагинами (~166 МБ).

## Варианты загрузки
1. **`EqualizerFD-Craft-Setup-v1.0.0.exe`** (~166 MB):
   - Полный офлайн установщик Inno Setup со всеми 98 студийными 64-битными VST-плагинами (ReaPlugs, Blue Cat, Kuassa и др.).
2. **`EqualizerFD-Craft-Update-Installer.exe`** (~3.4 MB):
   - Быстрое обновление (рекомендуется): обновляет исполняемые файлы программы, системный трей и веб-интерфейс, сохраняя все ваши пресеты и плагины.
3. **`EqualizerFD-Craft-Update.zip`** (~2.8 MB):
   - Быстрое обновление в ZIP-архиве без повторного скачивания плагинов.
4. **`EqualizerFD-Craft-v1.0.0-Portable.zip`** (~4.1 MB):
   - Портативная версия без необходимости установки.
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
    @{ Name = "EqualizerFD-Craft-Setup-v1.0.0.exe"; Path = "dist\EqualizerFD-Craft-Setup-v1.0.0.exe"; ContentType = "application/vnd.microsoft.portable-executable" },
    @{ Name = "EqualizerFD-Craft-Update-Installer.exe"; Path = "dist\EqualizerFD-Craft-Update-Installer.exe"; ContentType = "application/vnd.microsoft.portable-executable" },
    @{ Name = "EqualizerFD-Craft-Update.zip"; Path = "dist\EqualizerFD-Craft-Update.zip"; ContentType = "application/zip" },
    @{ Name = "EqualizerFD-Craft-v1.0.0-Portable.zip"; Path = "dist\EqualizerFD-Craft-v1.0.0-Portable.zip"; ContentType = "application/zip" }
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

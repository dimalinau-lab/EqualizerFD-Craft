# Publish release v1.2.0 to GitHub (Official Release)
param (
    [string]$TagName = "v1.2.0",
    [string]$ReleaseName = "Equalizer FD CRAFT v1.2.0 (Studio Stability & Full 50 Boss Effects)"
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
# Equalizer FD CRAFT v1.2.0 — Обновление стабильности, шумоподавления и полная коллекция Boss RC-505

В релизе **v1.2.0** полностью решены проблемы с искажениями звука при непрерывной речи, модернизирована система шумоподавления с внедрением VAD-гейта, добавлено автоматическое восстановление аудиопотока после гибернации/сна Windows, а также завершена интеграция всех 50 аутентичных студийных эффектов Boss RC-505 Mk2.

---

## Главные нововведения и исправления

### 1. Устранение «репения» и треска при непрерывной речи (Pause-Only Drift Compensation)
- **Проблема:** Разница тактовой частоты USB-микрофона и драйвера VB-Cable при непрерывной речи приводила к переполнению джиттер-буфера и регулярному сбросу сэмплов из середины звуковой волны, что звучало как непрерывный роботизированный зуд/треск.
- **Решение:** Внедрен алгоритм неразрушающей компенсации тактового дрифта. Во время активной речи (`speech_prob >= 0.20`) ни один сэмпл не отбрасывается. Микро-подстройка буфера выполняется исключительно в естественных паузах между словами и фразами, когда это на 100% бесшумно.
- **Удаление фазового DeClicker:** Устранен агрессивный высокочастотный IIR-фильтр, который ложно срабатывал на согласных («с», «т», «ч», «ц», «к», «п») более 1100 раз в секунду и смещал фазу сигнала.

### 2. Студийный мягкий лимитер (Smooth Rational Soft Knee)
- Жесткий цифровой клиппинг `clamp(-1.0, 1.0)` заменен на гладкий C1-рациональный мягкий лимитер.
- До уровня `-2.5 dBFS` искажения равны 0.00%. Громкие всплески голоса и предусиление компрессируются тепло и гладко без цифрового хрипа и гармонических щелчков.

### 3. Автоматическое восстановление после спящего режима и гибернации (Hibernation Watchdog)
- При выходе Windows из сна или гибернации встроенный сторожевой таймер отслеживает скачки системного времени и состояние аудиосессии.
- Поток WASAPI и маршрутизация виртуального кабеля VB-Cable перезапускаются автоматически без необходимости перезагрузки ПК или ручного перезапуска драйверов.

### 4. Полная аутентичная коллекция 50 эффектов Boss RC-505 Mk2
- Реализованы и протестированы все 50 эффектов оригинального процессора Boss RC-505:
  - **Pitch / Vocal:** Pitch Shift, Auto-Pitch, Robot Voice, Vocoder, Voice Double, Octave, Harmonist.
  - **Dynamics & Filters:** Dynamic Compressor, Sustainer, Auto-Wah, Radio Voice, Telephone, Lo-Fi, Ring Modulator.
  - **Modulation & Space:** Stereo Chorus, Flanger, Phaser, Tremolo, Pan, Tape Echo, Stereo Delay, Hall Reverb, Plate Reverb.
- Полностью устранен паразитный пропуск сухого необработанного голоса при активных вокодерах и питч-шифтерах.

### 5. Neural VAD Noise Gate (-80 dB Silence)
- Интеллектуальный шумовой шлюз на базе вероятности речи нейросети RNNoise (`speech_prob`).
- В паузах между фразами уровень фонового шума подавляется до абсолютной тишины (-80 dB), полностью отсекая щелчки клавиатуры, клики мыши и комнатный гул.

---

## Варианты загрузки
1. **`EqualizerFD-Craft-Setup-v1.2.0.exe`** (~166 MB):
   - Полный офлайн установщик Inno Setup со всеми 98 студийными 64-битными VST-плагинами (ReaPlugs, Blue Cat, Kuassa и др.).
2. **`EqualizerFD-Craft-Update-Installer.exe`** (~3.4 MB):
   - Быстрое обновление (рекомендуется): обновляет исполняемые файлы программы, системный трей и веб-интерфейс, сохраняя все ваши пресеты и плагины.
3. **`EqualizerFD-Craft-Update.zip`** (~2.9 MB):
   - Быстрое обновление в ZIP-архиве без повторного скачивания плагинов.
4. **`EqualizerFD-Craft-v1.2.0-Portable.zip`** (~4.2 MB):
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
    @{ Name = "EqualizerFD-Craft-Setup-v1.2.0.exe"; Path = "dist\EqualizerFD-Craft-Setup-v1.2.0.exe"; ContentType = "application/vnd.microsoft.portable-executable" },
    @{ Name = "EqualizerFD-Craft-Update-Installer.exe"; Path = "dist\EqualizerFD-Craft-Update-Installer.exe"; ContentType = "application/vnd.microsoft.portable-executable" },
    @{ Name = "EqualizerFD-Craft-Update.zip"; Path = "dist\EqualizerFD-Craft-Update.zip"; ContentType = "application/zip" },
    @{ Name = "EqualizerFD-Craft-v1.2.0-Portable.zip"; Path = "dist\EqualizerFD-Craft-v1.2.0-Portable.zip"; ContentType = "application/zip" },
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

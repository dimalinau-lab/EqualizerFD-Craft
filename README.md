# Equalizer FD CRAFT — Studio Audio Engine & DSP Environment

[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%2F%2011%20(x64)-blue.svg)]()
[![Rust](https://img.shields.io/badge/Language-Rust%202021-orange.svg)]()
[![License](https://img.shields.io/badge/License-Proprietary-darkgreen.svg)]()
[![Repository](https://img.shields.io/badge/GitHub-dimalinau--lab%2FEqualizerFD--Craft-black.svg)](https://github.com/dimalinau-lab/EqualizerFD-Craft)

---

## [RU] Русская документация

**Equalizer FD CRAFT** — это профессиональный высокопроизводительный движок цифровой обработки звука (DSP) и конфигурационная студия для Windows. Программа объединяет возможности системного аудиодрайвера Equalizer FD CRAFT, сверхнизколатентный аудиомост WASAPI, процессор эффектов луп-станции **Roland BOSS RC-505 Mk2**, а также библиотеку из **98 встроенных VST плагинов** с поддержкой любых пользовательских 64-битных VST DLL.

---

### Схема прохождения звука (Signal Flow)

Для того чтобы обработанный звук попадал в Discord, Telegram, OBS, Skype и игры, используется следующая маршрутизация:

```text
[Ваш физический микрофон] (USB, Fifine, Realtek, HyperX, XLR)
                 │
                 ▼
[Equalizer FD CRAFT: Движок обработки в реальном времени]
  ├── Параметрический эквалайзер (Biquad фильтры, 15-полосный ISO)
  ├── Шумоподавление, гейты и компрессоры (ReaGate, ReaComp)
  ├── 98 встроенных студийных VST плагинов (ReaPlugs, Kuassa, SPAN)
  └── Roland BOSS RC-505 Mk2 (Granular Scatter, Space Reverb, Slicer, Vinyl)
                 │
                 ▼
[CABLE Input (VB-Audio Virtual Cable)]  <-- Выбирается в программе как Вывод (Render)
                 │ (передача внутри виртуального кабеля Windows)
                 ▼
[CABLE Output (VB-Audio Virtual Cable)] <-- Выбирается в Discord / OBS / Telegram как Микрофон
```

---

### Пошаговое руководство по настройке: что выбрать, чтобы всё работало

#### Шаг 1. Установка VB-Audio Virtual Cable (в 1 клик)
Программа поставляется со встроенным официальным драйвером `VB-Audio Virtual Cable` (в папке `tools/vbcable/`):
1. Запустите `EqualizerFD-Craft.exe`.
2. В верхнем меню откройте **Настройки** -> **Установить VB-Audio Virtual Cable...** (или нажмите кнопку установки в появившемся мастере настройки).
3. Нажмите кнопку **Установить VB-Audio Virtual Cable (запуск от Администратора)**.
4. В появившемся окне контроля учетных записей Windows (UAC) подтвердите запуск, а в окне официального инсталлятора нажмите **Install Driver**.
5. После завершения установки перезапустите аудиослужбу Windows через меню **Настройки** -> **Перезапустить аудиослужбу** или перезагрузите компьютер.

#### Шаг 2. Настройка устройств в Equalizer FD CRAFT
1. Откройте **Мастер быстрой настройки микрофона** (меню **Настройки** -> **Мастер быстрой настройки...**):
   - **Шаг 1 (Вход):** Выберите ваш рабочий физический микрофон (например, `fifine SC3`, `Realtek(R) Audio`, `USB Audio Device`).
   - **Шаг 2 (Вывод):** Выберите `CABLE Input (VB-Audio Virtual Cable)`.
2. Нажмите кнопку **Подключить и начать работу**.
3. Обратите внимание на индикатор на верхней панели инструментов:
   - Кнопка **WASAPI аудиомост:** должна отображать `[В ЭФИРЕ]` (зеленый индикатор). Если мост выключен, нажмите на кнопку, чтобы включить его.

#### Шаг 3. Настройка программ связи и стриминга (Discord, Telegram, OBS, игры)

##### Настройка в Discord:
1. Откройте Discord -> **Настройки пользователя** -> **Голос и видео**.
2. В пункте **Устройство ввода** выберите:
   `CABLE Output (VB-Audio Virtual Cable)` (НЕ выбирайте реальный микрофон, иначе ваши собеседники будут слышать сырой необработанный звук!).
3. В пункте **Устройство вывода** оставьте ваши обычные наушники или колонки.
4. **Важные настройки Discord для сохранения высокого качества звука:**
   - **Шумоподавление (Krisp):** установите в положение **Отключено** (поскольку качественный гейт и фильтрация уже производятся в Equalizer FD CRAFT, Krisp может срезать частоты эквалайзера и VST эффектов).
   - **Эхоподавление:** отключите.
   - **Расширенная активация по голосу:** включите.
   - **Автоматическая регулировка усиления:** отключите (компрессор в Equalizer FD CRAFT выравнивает громкость гораздо точнее).

##### Настройка в Telegram / Skype / Zoom:
1. Откройте **Настройки** -> **Звонки** -> **Настройки микрофона**.
2. В качестве устройства ввода выберите `CABLE Output (VB-Audio Virtual Cable)`.

##### Настройка в OBS Studio:
1. В окне **Микшер звука** добавьте источник **Захват входного аудиопотока**.
2. В выпадающем списке устройств выберите `CABLE Output (VB-Audio Virtual Cable)`.

##### Настройка в играх (Counter-Strike 2, Dota 2, GTA V, Valorant):
1. В настройках звука игры в пункте **Микрофон** выберите либо `CABLE Output (VB-Audio Virtual Cable)`, либо сделайте `CABLE Output` устройством записи по умолчанию в Windows.

---

### Режимы работы движка

Программа поддерживает два взаимодополняющих режима:

1. **Режим реального времени (WASAPI Real-Time Low-Latency Bridge)** — **Рекомендуется**
   - Работает сразу без изменения системного реестра и без перезагрузки Windows.
   - Поток захватывается с физического микрофона, обрабатывается движком с задержкой ~2 мс и мгновенно подается в виртуальный кабель.
   - Включается и выключается одной кнопкой `[В ЭФИРЕ]` на верхней панели.

2. **Режим системного аудиодрайвера (Equalizer FD CRAFT Hook)**
   - Прописывает перехватчик в системный реестр Windows Audio Engine.
   - Обработка происходит на уровне службы `audiodg.exe`.
   - Требует запуска программы от имени Администратора для прописки хуков в реестр.

---

### Эффекты Roland BOSS RC-505 Mk2 и VST плагины

1. **Granular Beat Scatter (Скаттер и глитч):**
   - Гранулярный аудиопроцессор в стиле серии Roland AIRA.
   - 4 режима: `0: Случайный скаттер`, `1: Прямой репит (stutter)`, `2: Реверс гранул`, `3: Питч-шифт вверх (+1 октава)`.
   - Регулировки: **Глубина (Depth)**, **Длина гранулы (Length)** и **Плотность (Density)**.
2. **Space Reverb & Shimmer:**
   - Объемная реверберация с октавным гармоническим хвостом (Shimmer).
3. **Slicer & Pattern Gate:**
   - Ритмическая нарезка сигнала по темпу (1/4, 1/8, 1/16, 1/32 и триоли).
4. **98 встроенных VST плагинов:**
   - Компрессоры, параметрические эквалайзеры, гейты, сатураторы, хорусы и анализаторы спектра.
5. **Подключение собственных 64-битных VST DLL:**
   - В меню **Добавить фильтр** выберите **VST плагин (свой путь)...**, укажите путь к файлу `.dll` через удобное диалоговое окно обзора.

---

### Решение частых вопросов (Troubleshooting)

- **Вопрос: Я слышу свой собственный голос в наушниках (эхо/самопрослушка). Как отключить?**
  - **Ответ:** Откройте классическую панель звука Windows (`mmsys.cpl`), перейдите на вкладку **Запись**, откройте свойства `CABLE Output`, перейдите на вкладку **Прослушать** и убедитесь, что снята галочка **Прослушивать с данного устройства**.
- **Вопрос: Собеседники в Discord меня не слышат.**
  - **Ответ:** 
    1. Убедитесь, что в Equalizer FD CRAFT кнопка **WASAPI аудиомост** находится в состоянии `[В ЭФИРЕ]`.
    2. Проверьте, что в мастере настройки в качестве Входа выбран ваш настоящий микрофон, а в качестве Вывода — `CABLE Input`.
    3. Проверьте, что в Discord в качестве устройства ввода выбран именно `CABLE Output (VB-Audio Virtual Cable)`.
- **Вопрос: Звук хрипит или слышны щелчки.**
  - **Ответ:** Откройте меню **Настройки** -> **Установить VB-Audio Virtual Cable...** -> **Открыть панель управления VB-Cable** и проверьте, чтобы частота дискретизации микрофона и виртуального кабеля в Windows совпадала (рекомендуется `48000 Hz, 24 bit` или `48000 Hz, 32 bit`).
- **Вопрос: Как обновить программу до последней версии?**
  - **Ответ:** Откройте меню **Настройки** -> **Проверить обновления (GitHub)...** и нажмите кнопку **Проверить сейчас**. При наличии обновления программа скачает новый бинарник и выполнит атомарную перезагрузку.

---

## [EN] English Documentation

**Equalizer FD CRAFT** is an ultra-fast, professional digital signal processing (DSP) engine and configuration studio for Windows. It combines low-latency direct WASAPI audio streaming, native Equalizer FD CRAFT driver integration, the **Roland BOSS RC-505 Mk2** loop station effect suite, and a built-in library of **98 studio VST plugins** with support for custom 64-bit VST DLLs.

---

### Signal Routing Architecture

To route your processed microphone audio into Discord, Telegram, OBS, Skype, and games, follow this flow:

```text
[Physical Microphone] (USB, Fifine, Realtek, HyperX, XLR)
                 │
                 ▼
[Equalizer FD CRAFT: Real-Time DSP Engine]
  ├── Parametric Equalizer (Biquad filters, 15-band ISO)
  ├── Noise suppression, gates, and compressors (ReaGate, ReaComp)
  ├── 98 built-in studio VST plugins (ReaPlugs, Kuassa, SPAN)
  └── Roland BOSS RC-505 Mk2 (Granular Scatter, Space Reverb, Slicer, Vinyl)
                 │
                 ▼
[CABLE Input (VB-Audio Virtual Cable)]  <-- Selected in Equalizer FD CRAFT as Render (Output)
                 │ (internal Windows audio cable routing)
                 ▼
[CABLE Output (VB-Audio Virtual Cable)] <-- Selected in Discord / OBS / Telegram as Microphone (Input)
```

---

### Step-by-Step Setup Guide: What to Select So Everything Works

#### Step 1. Install VB-Audio Virtual Cable (1-Click Offline)
The application includes the official `VB-Audio Virtual Cable` driver files in `tools/vbcable/`:
1. Launch `EqualizerFD-Craft.exe`.
2. Open top menu: **Settings** -> **Install VB-Audio Virtual Cable...** (or click the install button in the setup wizard).
3. Click **Install VB-Audio Virtual Cable (Run as Administrator)**.
4. Accept the Windows User Account Control (UAC) prompt, then click **Install Driver** in the official installer window.
5. After installation completes, restart the Windows audio service from **Settings** -> **Restart Audio Engine** or reboot your PC.

#### Step 2. Configure Devices in Equalizer FD CRAFT
1. Open the **Microphone Setup Wizard** (**Settings** -> **Microphone Setup Wizard...**):
   - **Step 1 (Input):** Choose your real physical microphone (e.g., `fifine SC3`, `Realtek(R) Audio`, `USB Audio Device`).
   - **Step 2 (Output):** Choose `CABLE Input (VB-Audio Virtual Cable)`.
2. Click **Connect and Start**.
3. Verify that the **WASAPI Audio Bridge** chip in the top toolbar shows `[ON AIR]` (green status). If paused, click it to activate live routing.

#### Step 3. Configure Communication Apps (Discord, Telegram, OBS, Games)

##### In Discord:
1. Open Discord -> **User Settings** -> **Voice & Video**.
2. Set **Input Device** to:
   `CABLE Output (VB-Audio Virtual Cable)` (DO NOT select your physical microphone, otherwise listeners will receive raw, unprocessed audio).
3. Set **Output Device** to your regular headphones or speakers.
4. **Important Discord settings for maximum audio quality:**
   - **Noise Suppression (Krisp):** Set to **None** (Equalizer FD CRAFT handles gating and noise suppression; Krisp will muffle EQ and VST dynamics).
   - **Echo Cancellation:** Disable.
   - **Automatic Gain Control:** Disable.

##### In Telegram / Skype / Zoom:
1. Go to **Settings** -> **Audio & Video** -> **Microphone**.
2. Select `CABLE Output (VB-Audio Virtual Cable)`.

##### In OBS Studio:
1. In the **Audio Mixer**, add an **Audio Input Capture** source.
2. Select `CABLE Output (VB-Audio Virtual Cable)`.

##### In Games (Counter-Strike 2, Dota 2, GTA V, Valorant):
1. In the in-game audio settings, set the microphone device to `CABLE Output (VB-Audio Virtual Cable)` or set it as the Windows default recording device.

---

### Engine Operating Modes

1. **WASAPI Real-Time Audio Bridge (Recommended)**
   - Zero registry modification, zero reboot required.
   - Directly streams from physical microphone through the DSP chain to Virtual Cable with sub-3ms latency.
   - Controlled via the `[ON AIR]` toolbar button.

2. **Equalizer FD CRAFT System Driver Hook**
   - Deep kernel-level audio driver integration via `audiodg.exe`.
   - Requires running as Administrator to write device endpoint keys to the registry.

---

### Roland BOSS RC-505 Mk2 DSP Suite & VST Plugins

1. **Granular Beat Scatter:**
   - Roland AIRA-inspired granular scatter and glitch engine.
   - 4 Modes: `0: Random Scatter`, `1: Beat Repeat / Stutter`, `2: Reverse Grains`, `3: Pitch-Shift Up (+1 Octave)`.
   - Adjustable **Depth**, **Length**, and **Density** parameters.
2. **Space Reverb & Shimmer:**
   - Expansive reverberation with harmonic octave shimmer feedback.
3. **Slicer & Pattern Gate:**
   - Tempo-synchronized rhythmic chopper with triplet and straight subdivisions.
4. **98 Built-in VST Plugins:**
   - Dynamics, parametric EQs, filters, saturation, and stereophonic processors.
5. **Custom 64-bit VST DLL Loading:**
   - Select **VST Plugin (Custom Path)...** to browse and link any `.dll` plugin directly.

---

### Troubleshooting & FAQ

- **Q: I hear my own voice in my headphones (loopback/echo). How to disable it?**
  - **A:** Open classic Windows Sound Control Panel (`mmsys.cpl`), switch to the **Recording** tab, open properties for `CABLE Output`, go to the **Listen** tab, and uncheck **Listen to this device**.
- **Q: People in Discord cannot hear me.**
  - **A:** 
    1. Check if the **WASAPI Audio Bridge** in Equalizer FD CRAFT is `[ON AIR]`.
    2. Confirm that **Input** is set to your physical microphone and **Output** is set to `CABLE Input`.
    3. Confirm that Discord is set to receive from `CABLE Output (VB-Audio Virtual Cable)`.
- **Q: Audio crackles or stutters.**
  - **A:** Open **Settings** -> **Install VB-Audio Virtual Cable...** -> **Open VB-Cable Control Panel** and ensure sample rates match between your physical microphone and the virtual cable (recommended: `48000 Hz, 24-bit` or `32-bit`).

---

### Project File Tree

```text
D:\testik\
├── EqualizerFD-Craft.exe           # Main application executable
├── config.txt                      # Active configuration preset
├── preset_2.txt                    # Secondary configuration preset
├── tools\
│   └── vbcable\                    # Official VB-Audio Virtual Cable driver pack
│       ├── VBCABLE_Setup_x64.exe   # 64-bit driver installer
│       ├── VBCABLE_ControlPanel.exe# Virtual cable control panel
│       └── vbMmeCable64_win10.inf  # Windows 10/11 x64 driver definition
├── crates\
│   ├── eq_apo_cli\                 # Studio server, CLI commands & GitHub updater
│   ├── eq_apo_core\                # Core DSP biquads & parser
│   ├── eq_apo_device\              # WASAPI real-time streaming engine
│   └── resona_apo\                 # Roland BOSS RC-505 Mk2 DSP & shared memory
├── vst_plugins\                    # 98 x64 studio VST plugins
└── web\                            # Studio interface (HTML, CSS, JS)
```

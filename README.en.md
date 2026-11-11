<div align="center">
  <img src="web/icon.png" width="96" height="96" alt="Equalizer FD CRAFT" />
  <h1>Equalizer FD CRAFT</h1>
  <p>
    <strong>Real-Time Microphone DSP Studio &amp; Audio Engine for Windows in Pure Rust</strong><br>
    <em>Студия цифровой обработки звука микрофона в реальном времени с WASAPI-мостом и DeepFilterNet3 LL</em>
  </p>
  <p>
    <img src="https://img.shields.io/badge/Language-Rust-e05d44?logo=rust&logoColor=white" alt="Language: Rust" />
    <img src="https://img.shields.io/badge/Platform-Windows_x64-007ec6" alt="Platform: Windows x64" />
    <img src="https://img.shields.io/badge/License-MIT-4c1" alt="License: MIT" />
    <img src="https://img.shields.io/badge/Audio-WASAPI_%E2%80%A2_DeepFilterNet3-7c3aed" alt="Audio: WASAPI • DeepFilterNet3" />
  </p>
  <p>
    <strong>English Documentation</strong> • <a href="README.md">Документация на русском</a>
  </p>
</div>

---

## Table of Contents

1. [Architecture and Signal Flow](#architecture-and-signal-flow)
2. [Installation and Quick Start](#installation-and-quick-start)
3. [DSP Modules and Features](#dsp-modules-and-features)
   - [Interactive Parametric EQ and 32-Band RTA Analyzer](#interactive-parametric-eq-and-32-band-rta-analyzer)
   - [Neural AI Noise Suppression (DeepFilterNet3 LL + SpeechGuard)](#neural-ai-noise-suppression-deepfilternet3-ll--speechguard)
   - [Preamp, Effects Rack, and Config Directives](#preamp-effects-rack-and-config-directives)
   - [Roland BOSS RC-505 Mk2 Effects Engine](#roland-boss-rc-505-mk2-effects-engine)
   - [VST 2.4 Plugin Hosting](#vst-24-plugin-hosting)
   - [WASAPI Bridge, Headphone Monitoring, and Stream Stability](#wasapi-bridge-headphone-monitoring-and-stream-stability)
   - [UI, System Tray, and Auto-Updater](#ui-system-tray-and-auto-updater)
4. [Application Setup (Discord, Steam, Telegram, OBS Studio)](#application-setup-discord-steam-telegram-obs-studio)
5. [Keyboard Shortcuts](#keyboard-shortcuts)
6. [Troubleshooting (FAQ)](#troubleshooting-faq)
7. [Building from Source](#building-from-source)

---

## Architecture and Signal Flow

Equalizer FD CRAFT captures input directly from your physical microphone via WASAPI, runs a deterministic multi-stage DSP chain, and routes the processed signal into `VB-Audio Virtual Cable`.

Signal processing order inside [`ResonaDspEngine`](crates/resona_apo/src/dsp.rs):

```text
[Physical Microphone (WASAPI Capture, 48 kHz)]
        │
        ▼
[Equalizer FD CRAFT / ResonaDSP]
  ├─ 1. 80 Hz Low-Cut High-Pass (runs before neural denoising)
  ├─ 2. DeepFilterNet3 LL (48 kHz, zero-lookahead) + SpeechGuard (fallback: RNNoise)
  ├─ 3. User Preamp (-30..+20 dB, strictly post-AI)
  ├─ 4. Parametric Biquad IIR EQ and Graphic EQ
  ├─ 5. Roland BOSS RC-505 Mk2 Effects (50 algorithms)
  ├─ 6. Smooth Rational Soft Limiter (clip-free peak protection)
  └─ 7. 64-bit VST 2.4 Plugin Chain
        │
        ├───────────────────────────────────────┐
        ▼                                       ▼
[CABLE Input (VB-Audio Virtual Cable)]   [Shared Monitoring Tap (SHARED_TAP_HUB)]
        │                                       │
        ▼                                       ▼
[CABLE Output -> Discord / OBS / Games]  [Headphones / Speakers (zero duplicate AI CPU)]
```

---

## Installation and Quick Start

1. **Install the Application:** download the latest installer from [GitHub Releases](https://github.com/dimalinau-lab/EqualizerFD-Craft/releases/latest) and run setup. The full installer includes the audio engine, the `DeepFilterNet3_ll_onnx.tar.gz` model archive, and 98 bundled 64-bit VST plugins.
2. **Install VB-Audio Virtual Cable:** open **Settings → Install VB-Audio Virtual Cable...**, launch the driver installer as Administrator, and approve the UAC prompt. Reboot Windows or restart the audio service if required.
3. **Align Sample Rates:** press `Win + R` → run `mmsys.cpl` → **Recording** tab. Ensure both your physical microphone and **`CABLE Output`** are set to **`48000 Hz`** (`24-bit` or `16-bit`).
4. **Select Virtual Input in Apps:** set **`CABLE Output (VB-Audio Virtual Cable)`** as the microphone input device in Discord, OBS Studio, Telegram, and games.

---

## DSP Modules and Features

### Interactive Parametric EQ and 32-Band RTA Analyzer

The frequency response canvas renders the combined filter curve overlaid with a real-time 32-band logarithmic Goertzel spectrum analyzer.

- **Node Controls:**
  - **Left-Click on Node:** selects and pins the parameter card in place; click empty canvas space to deselect.
  - **Drag Node:** horizontal movement adjusts frequency (Hz), vertical movement adjusts gain (dB).
  - **Mouse Wheel over Node:** adjusts filter bandwidth (`Q`).
- **Node Card Actions:** `Bypass / On`, `Invert` (flips gain sign), `Delete`.
- **Supported Biquad IIR Filter Types:**
  - `PK` (Peaking) — bell boost/cut at target frequency;
  - `LS` (Low Shelf) — boosts or attenuates frequencies below cutoff;
  - `HS` (High Shelf) — boosts or attenuates high frequencies;
  - `HP` (High Pass) — cuts low rumble, desk thumps, and plosives;
  - `LP` (Low Pass) — attenuates ultrasonic hiss;
  - `NO` (Notch) — narrow band-stop cut for 50/60 Hz hum or tonal ringing.
- **Canvas Controls:** toggle real-time spectrum (`RTA Spectrum`) and `Undock` to detach the graph into a separate window.

### Neural AI Noise Suppression (DeepFilterNet3 LL + SpeechGuard)

Noise suppression is driven by **DeepFilterNet3 Low-Latency** (`models/DeepFilterNet3_ll_onnx.tar.gz`, `48000 Hz`, `fft_size = 960`, `hop_size = 480`, zero lookahead) with non-blocking preload and cached model parameters. If the model archive is missing, the engine automatically falls back to RNNoise (`nnnoiseless`).

- **Pre-AI High-Pass & Post-AI Preamp:** a fixed `80 Hz` High-Pass removes sub-rumble before inference, while user `Preamp` runs strictly **after** AI denoising so noise floors are never amplified prior to separation.
- **`SpeechGuard`:** adaptive noise-floor tracking (`noise_floor_rms`), anti-click transient rejection comparing speech energy (`180–3600 Hz`) against high-frequency click energy (`> 5 kHz`), dynamic `min_db_thresh`, and word-tail hold (`180 ms` hold, `65 ms` release).
- **Live Telemetry:** reports active backend and SNR (`DFN3-LL | SNR xx dB`) alongside `VAD: Idle / Speech (xx%)`.
- **Wet/Dry Blend (`0%–100%`) & Presets:**
  - `65% (Studio)` — gentle cleanup for quiet rooms while preserving vocal overtones;
  - `85% (Discord / Krisp)` — balanced preset for voice chat and streaming;
  - `100% (Maximum)` — aggressive isolation for loud fans, street noise, and mechanical keyboards.

### Preamp, Effects Rack, and Config Directives

The effects rack executes blocks sequentially from top to bottom with per-block reorder and bypass toggles.

- **Preamp:** clean digital gain from `-30` to `+20 dB` applied after AI denoising.
- **Graphic Equalizers:** 15-band ISO and 31-band equalizer support.
- **Config Directives (`config.txt`):** supports recursive `Include:` expansion, `Device:`, `Channel:`, and `Copy:` channel routing matrices.

### Roland BOSS RC-505 Mk2 Effects Engine

Integrated DSP engine implementing 50 studio and vocal effect algorithms, including:
- **Pitch & Vocal:** Pitch Shift, Auto-Pitch, Robot Voice, Vocoder, Voice Double, Octave, Harmonist;
- **Dynamics & Filters:** Dynamic Compressor, Sustainer, Auto-Wah, Radio Voice, Telephone, Lo-Fi, Ring Modulator;
- **Modulation, Space & Rhythm:** Stereo Chorus, Flanger, Phaser, Tremolo, Pan, Tape Echo, Stereo Delay, Hall Reverb, Plate Reverb, Beat Repeat, Scatter, Slicer, Vinyl Flick.

Each effect block exposes parameter controls and curated factory presets.

### VST 2.4 Plugin Hosting

Supports native 64-bit VST 2.4 DLLs, including 98 pre-bundled studio plugins in `vst_plugins\` (Cockos ReaPlugs, Blue Cat, Kuassa, TDR, and more) as well as third-party DLLs. Supports vendor GUI windows, factory presets, parameter control, and bypass. On startup, the host automatically grants read/execute ACL permissions on `vst_plugins` and `models`.

### WASAPI Bridge, Headphone Monitoring, and Stream Stability

- **Direct WASAPI Audio Bridge:** operates with a `25 ms` capture buffer (`250_000` HNS) and `20 ms` / `10 ms` init buffering without requiring audio service restarts when parameters change.
- **Pause-Only Drift Compensation:** clock-rate drift adjustment between the USB microphone and virtual cable executes strictly during natural speech pauses so active speech samples are never dropped.
- **Hibernation Watchdog:** detects Windows sleep/hibernation resume events and automatically recovers WASAPI capture/render streams.
- **Headphone Monitoring (Sidetone):** toggle (`[ON AIR]` / `[OFF]`), render device selector, and volume slider in the top toolbar.
  - Subscribes to the already-cleaned 48 kHz output from the VB-Cable bridge via [`SHARED_TAP_HUB`](crates/eq_apo_device/src/stream.rs) with zero duplicate AI CPU overhead.
  - Selected output device, active toggle state, and volume persist in `settings.json` and restore automatically on restart.

### UI, System Tray, and Auto-Updater

- **Themes:** Light (native Windows desktop look) and Dark themes.
- **Localization:** instant switching between English and Russian.
- **System Tray (`EqualizerTray.exe`):** minimize-to-tray lifecycle, bridge status toggle, Windows autostart configuration, and `audiodg.exe` restart helper.
- **Auto-Updater:** checks GitHub Releases via `/api/update/check` using `CARGO_PKG_VERSION`, offering both lightweight updates (without re-downloading VST plugins) and the full setup installer.

---

## Application Setup (Discord, Steam, Telegram, OBS Studio)

1. **Discord:** *User Settings → Voice & Video → Input Device* → **`CABLE Output (VB-Audio Virtual Cable)`**. Disable Krisp noise suppression and Automatic Gain Control in Discord to prevent double processing.
2. **Steam / CS2 / Games:** *Settings → Audio → Microphone Device* → **`CABLE Output (VB-Audio Virtual Cable)`**.
3. **Telegram Desktop:** *Settings → Advanced → Call Settings → Microphone* → **`CABLE Output`**.
4. **OBS Studio:** *Settings → Audio → Mic/Auxiliary Audio* → **`CABLE Output`**.

---

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + N` | Create a new configuration / tab |
| `Ctrl + O` | Open a `.txt` configuration file |
| `Ctrl + S` | Save the active configuration |
| `Ctrl + Z` | Undo last edit |
| `Ctrl + Y` / `Ctrl + Shift + Z` | Redo undone edit |
| `F8` | Toggle the frequency response / RTA analysis panel |

---

## Troubleshooting (FAQ)

- **No audio in Discord or games:** verify **`CABLE Output (VB-Audio Virtual Cable)`** is selected as the input device, the WASAPI bridge is live, and `Preamp` is not attenuated.
- **Crackling, robotic artifacts, or stuttering:** open `mmsys.cpl` and confirm both your physical microphone and `CABLE Output` are set to **`48000 Hz`**.
- **Audio looping or repeating:** check the effects rack for active BOSS RC-505 rhythmic units (`Beat Repeat`, `Scatter`, `Slicer`, `Twist Delay`) or third-party VST delays with high feedback, and bypass or remove them.
- **Silent mic after reconnecting USB hardware:** click the bridge restart control in the app or system tray menu.

---

## Building from Source

Prerequisites: Rust (`cargo`), Windows SDK / MSVC toolchain, and optionally Inno Setup 6 (`D:\Inno Setup 6\ISCC.exe`) for installer packaging.

```powershell
# 1. Clone repository
git clone https://github.com/dimalinau-lab/EqualizerFD-Craft.git
cd EqualizerFD-Craft

# 2. Run test suites
cargo test --release --manifest-path crates/eq_apo_core/Cargo.toml
cargo test --release --manifest-path crates/eq_apo_cli/Cargo.toml
cargo test --release --manifest-path crates/resona_apo/Cargo.toml

# 3. Build release binaries
cargo build --release --manifest-path crates/resona_apo/Cargo.toml
cargo build --release --manifest-path crates/eq_apo_cli/Cargo.toml

# 4. Build Inno Setup installers and ZIP archives into dist/
pwsh -File tools/build_release.ps1
```

License: [MIT](LICENSE)

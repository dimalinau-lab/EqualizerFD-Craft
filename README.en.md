# Equalizer FD CRAFT v1.0.0

<p align="center">
  <img src="web/icon.png" width="100" height="100" alt="Equalizer FD CRAFT Logo" />
</p>

<p align="center">
  <strong>Professional Real-Time Microphone Digital Audio Processing Studio for Windows</strong><br>
  <em>Deep Learning Neural AI Noise Suppression (RNNoise), FabFilter-style interactive EQ, 98 bundled VST plugins, and ultra-low latency.</em>
</p>

<p align="center">
  <a href="https://github.com/dimalinau-lab/EqualizerFD-Craft/releases/latest"><img src="https://img.shields.io/github/v/release/dimalinau-lab/EqualizerFD-Craft?label=Release&color=blue" alt="Release" /></a>
  <img src="https://img.shields.io/badge/Version-1.0.0_Stable-success" alt="Version 1.0.0" />
  <img src="https://img.shields.io/badge/Platform-Windows_10_|_11_(x64)-informational" alt="Windows 10/11" />
  <img src="https://img.shields.io/badge/Engine-Pure_Rust_WASAPI-orange" alt="Rust WASAPI" />
  <img src="https://img.shields.io/badge/License-MIT-brightgreen" alt="MIT License" />
</p>

<p align="center">
  <strong>Documentation Language:</strong>
  <a href="README.md">🇷🇺 Русский</a> |
  <strong>🇬🇧 English</strong>
</p>

---

## Table of Contents
1. [In Plain English: What Does This Program Do?](#in-plain-english-what-does-this-program-do)
2. [Quick Start in 3 Easy Steps (For Beginners)](#quick-start-in-3-easy-steps-for-beginners)
3. [How Audio Works: Signal Flow Diagram](#how-audio-works-signal-flow-diagram)
4. [Installing and Configuring VB-Audio Virtual Cable](#installing-and-configuring-vb-audio-virtual-cable)
5. [Comprehensive Button-by-Button Guide](#comprehensive-button-by-button-guide)
   - [1. Top Control Bar](#1-top-control-bar)
   - [2. Interactive Frequency Curve (FabFilter-style EQ)](#2-interactive-frequency-curve-fabfilter-style-eq)
   - [3. Deep Learning AI Noise Suppression (RNNoise)](#3-deep-learning-ai-noise-suppression-rnnoise)
   - [4. Preamplification (Preamp) & Effects Rack](#4-preamplification-preamp--effects-rack)
   - [5. Bundled Studio VST Plugins](#5-bundled-studio-vst-plugins)
   - [6. Device Panel and Audio Bridge Status](#6-device-panel-and-audio-bridge-status)
6. [Configuring Microphone in Games and Apps (Discord, CS2, Telegram, OBS)](#configuring-microphone-in-games-and-apps)
7. [Keyboard Shortcuts Cheat Sheet](#keyboard-shortcuts-cheat-sheet)
8. [Frequently Asked Questions (FAQ)](#frequently-asked-questions-faq)
9. [Building from Source (For Developers)](#building-from-source-for-developers)

---

## In Plain English: What Does This Program Do?

Imagine a **professional sound engineer** sitting between your microphone and your listeners:
- **Silences all background noises:** mechanical keyboard typing, mouse clicks, computer fan whirring, and room echoes are completely wiped out in real time by an AI neural network (RNNoise).
- **Enriches your vocal timbre:** delivers broadcast-ready warmth, removes boxy muffled frequencies, cuts desk thumps, and adds high-end clarity like a radio host or pro streamer.
- **Works everywhere:** compatible with Discord, Telegram, CS2, Dota 2, OBS Studio, Zoom, Skype, and any Windows voice application.

![Equalizer FD CRAFT Main Studio Interface](docs/screenshots/01_main_studio_dark.png)

---

## Quick Start in 3 Easy Steps (For Beginners)

If you want your microphone to sound crystal clear right now without reading technical manuals, follow these 3 steps:

### Step 1. Download and Install the Application
Download **`EqualizerFD-Craft-Setup-v1.0.0.exe`** from the [GitHub Releases page](https://github.com/dimalinau-lab/EqualizerFD-Craft/releases/latest) and complete installation. It includes the audio engine and all 98 studio plugins.

### Step 2. Install the Virtual Cable (One Click)
Launch Equalizer FD CRAFT. On the top menu bar, select **"Devices"** &rarr; **"Install VB-Audio Virtual Cable"**. In the pop-up window, click the blue button:

![VB-Cable Installation Modal](docs/screenshots/04_vbcable_modal.png)

Approve the Windows User Account Control (UAC) prompt and wait for driver confirmation.

### Step 3. Select the Cable in Discord or Your Game
Open Discord (or your favorite game), navigate to **User Settings &rarr; Voice & Video &rarr; Input Device** and select:
> **`CABLE Output (VB-Audio Virtual Cable)`**

*You're all set! Your teammates and friends now hear your processed, studio-grade voice without noise.*

---

## How Audio Works: Signal Flow Diagram

Here is how your voice travels through the system:

```
 [Your Physical Microphone]
             │
             ▼
 [Equalizer FD CRAFT (Real-Time Engine)]
   ├─ 1. AI Noise Suppression (removes keyboard clicks and hum)
   ├─ 2. Preamplifier (volume sensitivity adjustment)
   ├─ 3. Interactive Parametric EQ (polishes speech frequencies)
   └─ 4. VST Plugins (compression, de-essing, tape warmth)
             │
             ▼
 [CABLE Input (Virtual Audio Cable - Input End)]
             │
             ▼
 [CABLE Output (Virtual Audio Cable - Output End)]
             │
             ▼
 [Discord / Steam Games / CS2 / Telegram / OBS Studio / Stream]
```

**What is a Virtual Audio Cable?**  
It is a virtual audio driver for Windows that acts like an invisible cable. Equalizer FD CRAFT feeds your processed audio into `CABLE Input`, and games/chat apps pick it up from `CABLE Output`.

---

## Installing and Configuring VB-Audio Virtual Cable

The virtual cable installer is bundled inside the app for convenience:

1. In the top menu, choose **"Devices"** &rarr; **"Install VB-Audio Virtual Cable"** (or click the cable icon on the bottom device bar).
2. Click **"Install VB-Audio Virtual Cable (Administrator Launch)"**.
3. When prompted by Windows UAC, click **"Yes"**.
4. Click **"Install Driver"** in the installer window.
5. Setup completes within seconds.
6. *(Recommended: restart your PC once so Windows finalizes the new audio endpoint).*

> [!TIP]
> **Matching Sample Rates (Eliminate Latency & Stutter):**  
> Press `Win + R`, type `mmsys.cpl` and hit Enter to open Windows Sound Control Panel.  
> Under both the **Recording** tab (your physical microphone & `CABLE Output`), open **Properties &rarr; Advanced** and ensure both are set to the same studio format: **`24-bit, 48000 Hz`** (or `16-bit, 48000 Hz`).

---

## Comprehensive Button-by-Button Guide

Every section of Equalizer FD CRAFT is tailored for zero-friction audio tweaking:

---

### 1. Top Control Bar

The classic desktop menu gives immediate access to presets and configuration tools:

| Menu / Button | Function & Purpose |
| :--- | :--- |
| **File &rarr; New...** | Creates a blank configuration or loads a clean starter preset. |
| **File &rarr; Open... (Ctrl+O)** | Opens any existing `.txt` preset configuration file. |
| **File &rarr; Save (Ctrl+S)** | Saves current filter chains, parameters, and VST states to disk. |
| **File &rarr; Export...** | Exports configuration for backups or transferring to another computer. |
| **Edit &rarr; Undo (Ctrl+Z)** | Reverts the last parameter adjustment or node position. |
| **Edit &rarr; Redo (Ctrl+Y)** | Redoes the previously undone action. |
| **Presets** | Curated presets for different voices: Streamer, Podcast, Clean Broadcast, Gaming. |
| **Devices** | Selects your input microphone hardware and stream audio bridge endpoints. |
| **Themes** | Instantly switches between ultra-clean **OLED Dark** and native **Windows Classic Light** theme. |
| **Language** | Toggles interface language between English and Russian on the fly. |
| **Microphone Wizard** | Interactive step-by-step assistant for setting optimal microphone volume levels. |
| **Updates** | Built-in GitHub Releases desktop updater with one-click background updates. |

---

### 2. Interactive Frequency Curve (FabFilter-style EQ)

The frequency response curve visualizes your microphone's tone across the entire human hearing spectrum:

![Interactive EQ Curve](docs/screenshots/02_fabfilter_eq_node.png)

#### Understanding Frequency Regions:
- **20 – 100 Hz (Sub-bass / Rumble):** Desk thumping, wind, air conditioning. Should be rolled off using a High-Pass filter.
- **100 – 350 Hz (Vocal Warmth & Chest):** Fundamental frequencies of male and female voices. Boost slightly for richness; cut if your voice sounds muddy.
- **350 – 800 Hz (Boxiness):** Hollow room reflections and cardboard resonances. A modest dip cleans up clarity.
- **1,000 – 4,000 Hz (Speech Presence):** Critical speech intelligibility range. Gentle boosts around 2–3.5 kHz help words cut through game sounds.
- **5,000 – 20,000 Hz (Air & Sparkle):** Crisp vocal detail and modern studio sheen.

#### Interacting with Nodes on the Canvas:
- **Left-Click on Any Node (e.g., P1, P2):** Selects and pins the action tooltip card open. The card stays in place while you adjust buttons. Click empty space on the graph to deselect.
- **Drag Node with Mouse:**
  - Dragging **horizontally (Left/Right)** adjusts frequency in Hertz (Hz).
  - Dragging **vertically (Up/Down)** adjusts boost or cut in Decibels (Gain in dB).
- **Mouse Wheel over Node:** Adjusts filter bandwidth ($Q$ value). A high $Q$ creates a surgical notch; a lower $Q$ produces musical, smooth curves.

#### Action Buttons in the Node Tooltip Card:
- **`Bypass` / `On`** — Temporarily disables this specific filter to A/B test your voice changes.
- **`Invert`** — Flips the gain sign (e.g., toggles $+4.0$ dB into $-4.0$ dB instantly).
- **`Delete`** — Removes the selected filter from the active block.
- **Filter Type Selector Pills:**
  - **`PK` (Peaking / Bell)** — Standard parametric bell curve for boost/cut.
  - **`LS` (Low Shelf)** — Uniformly boosts or cuts all frequencies below the cutoff.
  - **`HS` (High Shelf)** — Uniformly boosts or cuts all frequencies above the cutoff (vocal air).
  - **`HP` (High Pass)** — Cuts everything below the cutoff (vital for cutting desk thumps at 80 Hz).
  - **`LP` (Low Pass)** — Attenuates extreme ultrasonic frequencies and hiss.
  - **`NO` (Notch)** — Surgical ultra-deep cut for electrical hum (50/60 Hz).

#### Top-Right Graph Controls:
- **`RTA Spectrum [On/Off]`** — Toggles the real-time 60 FPS spectrum analyzer overlay matching your speech.
- **`Undock`** — Pops the EQ canvas out into an independent floating window (ideal for dual-monitor setups).

---

### 3. Deep Learning AI Noise Suppression (RNNoise)

An advanced deep neural network (GRU) engine implemented directly in pure Rust. Runs with sub-millisecond processing and less than 1% CPU footprint:

![RNNoise AI Noise Suppression Card](docs/screenshots/03_neural_ai_rnnoise.png)

- **Power Toggle Button:** Enables or bypasses the neural noise gate in the signal chain.
- **`VAD: Idle / Speech (xx%)` Indicator:** Real-time Voice Activity Detection. When you are silent, background noise is blocked. When you speak, the indicator turns green and cleanly passes your natural voice.
- **Suppression Strength Slider (0% to 100%):**
  - `0%` — Raw microphone sound without filtering.
  - `100%` — Maximum background isolation.
- **Quick Preset Pills:**
  - **`50% (Gentle)`** — Light filtering for already quiet rooms, preserving maximum natural vocal harmonics.
  - **`85% (Optimal)`** — Recommended balance for everyday streaming, gaming, and calls.
  - **`100% (Maximum)`** — Heavy duty suppression for noisy environments, mechanical switches, and nearby fans.

---

### 4. Preamplification (Preamp) & Effects Rack

The effects rack processes blocks sequentially from top to bottom. Reorder them using up/down arrows or toggle them via individual power buttons:

- **Preamp Block:**
  - Gain slider ranging from $-30.00$ dB to $+20.00$ dB.
  - If your USB or XLR microphone signal is naturally quiet, boost $+3$ to $+6$ dB cleanly in digital space without Windows analog hiss.
- **"+ Add Filter / Effect" Button:**
  - Opens the comprehensive library: Parametric Biquad, 15/31-Band Graphic EQ, Delay, Channel Copy/Mixer matrix, Roland BOSS RC-505 processors, and VST plugins:

![Effects and Plugins Library](docs/screenshots/06_add_effect_menu.png)

---

### 5. Bundled Studio VST Plugins

Equalizer FD CRAFT includes **98 curated 64-bit VST plugins**:
- **Dynamic Compressors (`ReaComp`, `MJUC jr`, `Outlaw`):** Level out volume so whispers remain audible while loud screams never distort.
- **De-essers (`SPATIAL`, `TDR Nova`):** Tame harsh, piercing sibilance ("S" and "Sh" sounds).
- **Analog Saturators (`IVGI2`, `VariSaturator`):** Impart subtle tape warmth and radio richness.

Click **"Open Plugin Window"** on any VST card to access vendor GUIs, tweak parameters, and save presets.

---

### 6. Device Panel and Audio Bridge Status

The footer bar anchors your hardware connections:
- **Input Device Selector:** Choose your physical hardware mic (e.g., `fifine SC3`, `HyperX QuadCast`, `Focusrite Scarlett`).
- **Bridge Indicator ("WASAPI Bridge: Active"):** Confirms low-latency audio transmission directly to VB-Cable.
- **Restart Bridge Button:** Instantly reconnects the stream if you unplug or reconnect your USB microphone.

---

## Configuring Microphone in Games and Apps

To deliver your polished audio to other people, point their input settings to the virtual cable:

### 1. Discord
1. Open **User Settings** (gear icon) &rarr; **Voice & Video**.
2. Set **Input Device** to: `CABLE Output (VB-Audio Virtual Cable)`.
3. Keep **Output Device** set to your regular headphones or speakers.
4. *(Recommendation):* Under Voice Processing in Discord, disable "Krisp Noise Suppression" and "Automatic Gain Control", as Equalizer FD CRAFT handles this with superior quality.

### 2. Counter-Strike 2 & Steam Games
1. Open game Settings &rarr; **Audio**.
2. Set **Microphone Device** to `CABLE Output (VB-Audio Virtual Cable)`.
3. Choose your activation mode (Push-to-Talk or Voice Activity).

### 3. Telegram Desktop
1. Go to **Settings &rarr; Advanced &rarr; Call Settings**.
2. Select `CABLE Output` as your microphone.

### 4. OBS Studio
1. Open **Settings &rarr; Audio**.
2. Under **Mic/Auxiliary Audio**, select `CABLE Output`.

---

## Keyboard Shortcuts Cheat Sheet

| Shortcut | Action |
| :--- | :--- |
| **Ctrl + S** | Save active preset configuration to disk. |
| **Ctrl + Z** | Undo last edit or parameter change. |
| **Ctrl + Y** | Redo previously undone action. |
| **Ctrl + O** | Open an existing configuration file. |
| **F5** | Re-read and resynchronize studio state. |
| **Delete** | Delete highlighted EQ node or effect card. |
| **Space** | Toggle global studio audio bypass. |

---

## Frequently Asked Questions (FAQ)

### 1. Nobody hears me in Discord. What should I check?
- Ensure Discord's Input Device is set specifically to **`CABLE Output`** and not your physical microphone.
- Look at Equalizer FD CRAFT's bottom status bar: verify that **`WASAPI Bridge: Active`** is displayed in green.
- Make sure Preamp is not muted and the Master volume slider is above zero.

### 2. Audio stutters or sounds robotic
- This happens when your microphone and VB-Cable use mismatched sample rates.
- Press `Win + R`, type `mmsys.cpl` and press Enter.
- Under **Recording**, right-click your physical microphone &rarr; **Properties &rarr; Advanced** &rarr; set to **`24-bit, 48000 Hz`**.
- Do the exact same for **`CABLE Output`** (ensure it is also set to `48000 Hz`). Click Apply.

### 3. Audio degraded or sounds stuck on repeat (like a plugin looping)?
- **Cause 1: A loop / stutter effect is active in the rack.**
  - Check your right effect rack. Roland BOSS RC-505 units such as **`Beat Repeat / Roll`**, **`Scatter`**, **`Slicer`**, or **`Twist Delay`** are specifically designed to slice and continuously repeat incoming audio grains.
  - Simply click the power toggle on that effect card or delete it (the cross button) to restore pristine audio.
- **Cause 2: A third-party VST plugin buffer is stuck.**
  - Third-party VSTs with extreme feedback settings (like a delay at 100% feedback or a frozen noise gate) can recirculate audio endlessly.
  - Click the power switch on that VST card or press **`F5`** in the application window to immediately flush and resynchronize the audio engine.
- **Cause 3: Windows sample rate mismatch.**
  - If the microphone runs at 44100 Hz while VB-Cable is at 48000 Hz, Windows will drop sample packets continuously, producing robotic stutter.
  - Open `mmsys.cpl` and ensure both devices are set strictly to **`48000 Hz`**.
- **Cause 4: Acoustic feedback loop.**
  - Verify that microphone input is not playing back through open speakers, creating an acoustic feedback loop.

### 4. Can the app run automatically when Windows starts?
- Yes! In the official installer, tick the checkbox *«Start Equalizer FD CRAFT with Windows»*.
- It minimizes to the system notification area (system tray) near the clock and runs silently without cluttering your desktop.

---

## Building from Source (For Developers)

To build Equalizer FD CRAFT from source on Windows:

```powershell
# 1. Clone repository
git clone https://github.com/dimalinau-lab/EqualizerFD-Craft.git
cd EqualizerFD-Craft

# 2. Compile release binaries
cargo build --release --manifest-path crates/resona_apo/Cargo.toml
cargo build --release --manifest-path crates/eq_apo_cli/Cargo.toml

# 3. Compile Inno Setup installers & portable archives
pwsh -File tools/build_release.ps1
```

---

<p align="center">
  Crafted with passion for audio fidelity by the <strong>Equalizer FD CRAFT Team</strong> • 2026<br>
  Released under the <a href="LICENSE">MIT License</a>
</p>

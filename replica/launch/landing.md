# ResonaDSP Landing Page Architecture & Copy

> **Angle:** The bulletproof, zero-latency Windows system audio equalizer that never breaks on Windows updates.
> **Brand Voice:** Surgical, precise, engineering-driven, transparent.

---

## 1. Hero Section
- **Headline (H1):** The Windows System Equalizer That Stays Alive After Every Update.
- **Subheadline:** Studio-grade parametric EQ, arbitrary-band splines, and channel matrix routing powered by a zero-latency Rust DSP engine. With an auto-healing registry guard that prevents Windows Update from wiping your audio tuning.
- **Primary CTA Button:** Download ResonaDSP (Free)
- **Secondary CTA / Micro-copy:** Windows 10 & 11 (64-bit) • Zero background telemetry • Local offline processing
- **Hero Visual:** Interactive frequency curve with real-time draggable biquad nodes and live driver hook status indicator (`● Audio Engine: SFX/EFX Active • 0.00 ms added latency`).

---

## 2. The Problem (Community Pain Points)
- **Headline (H2):** Why Windows Audio Tuning Usually Breaks.
- **Problem 1: The Monthly Driver Wipe.**
  Every cumulative Windows update or GPU/audio driver patch silently overwrites your audio endpoint registry keys (`FxProperties`). You boot your PC, launch your favorite game or playlist, and your carefully calibrated headphone correction is gone without any warning.
- **Problem 2: Troubleshooting Mode Hell.**
  Modern Windows 11 audio stacks choke on legacy LFX/GFX hooks. Users spend hours toggling obscure checkboxes, guessing whether Pre-mix or Post-mix stages are active, or wondering if Discord and games are bypassing their microphone processing entirely.

---

## 3. How It Works
- **Headline (H2):** Seamless Tuning in Three Steps.
- **Step 1: Auto-Detect & One-Click Endpoint Hooking.**
  ResonaDSP automatically scans your system render and capture devices, selecting the optimal driver architecture (SFX/EFX with Pre-mix) for your exact Windows build.
- **Step 2: Shape Your Sound with Drag-and-Drop Precision.**
  Tune frequencies across parametric biquad filters, 31-band graphic splines, channel routing matrices, and import existing REW or AutoEq calibration profiles in one click.
- **Step 3: Forget About Updates Forever.**
  The lightweight Resona Guard service monitors audio endpoint keys in the background. If a Windows update resets your driver, ResonaDSP restores your APO hooks instantaneously—without requiring a reboot.

---

## 4. Key Features (Fixes First, Parity Included)
- **Headline (H2):** Engineered for Stability, Designed for Pro Audio.
- **Feature 1: Auto-Healing Registry Guard (The Windows Update Shield).**
  Continuous watchdog for endpoint registry hooks. No more broken configs after "Patch Tuesday".
- **Feature 2: Driver Signal Probe & Real-Time VU Meter.**
  Inspect live audio engine input versus processed output. Know immediately whether your filters are active or bypassed.
- **Feature 3: Interactive Draggable Parametric Canvas.**
  Move beyond rigid tables. Click and drag filter nodes directly on a high-precision logarithmic frequency response curve (10 Hz – 24 kHz) with real-time dB readout.
- **Feature 4: Monotone Cubic Spline GraphicEQ.**
  Smooth, artifact-free frequency curves across arbitrary points or standard ISO 31-band 1/3-octave grids.
- **Feature 5: Arithmetic Channel Matrix.**
  Route, copy, attenuate, or invert channels with declarative formulas (`L = 0.7*L + 0.3*R`) for custom downmixing, crossfeed, and spatial widening.
- **Feature 6: Instant 1-Click Profile & REW Import.**
  Seamlessly load Room EQ Wizard (REW) exports and industry-standard declarative config files.

---

## 5. Transparent Pricing
- **Headline (H2):** Fair, Transparent Pricing. No Subscriptions.
- **Subheadline:** Core DSP and update protection are free forever. Upgrade to Pro for cloud presets and advanced acoustics.

| Tier | Price | Who It Is For | What Is Included |
|---|---|---|---|
| **Free / Community** | **$0** (Free Forever) | Gamers, casual listeners, and daily desktop users | Full Biquad IIR filters, GraphicEQ spline, Channel matrix, REW import, and Windows Update auto-healing guard. |
| **Pro Lifetime** | **$24** (One-time payment) | Audiophiles, streamers, and studio engineers | Everything in Free + Built-in AutoEq cloud database (4,000+ headphones), VST3 container host, Room Impulse Response convolution engine, and live audio signal probe. |

---

## 6. Frequently Asked Questions (FAQ)
- **Q: Does ResonaDSP add any audio latency or buffer delay?**
  **A:** No. ResonaDSP compiles directly into the Windows Audio Engine (`audiodg.exe`) pipeline using SIMD-accelerated Rust code, executing in sub-microsecond blocks with 0.00 ms introduced buffering.
- **Q: Will my EQ settings survive Windows 11 cumulative updates?**
  **A:** Yes. Unlike legacy tools where updates reset `FxProperties`, ResonaDSP's background guard detects driver replacements and restores your hooks immediately without a reboot.
- **Q: Can I use my existing Equalizer APO configs or Room EQ Wizard (REW) files?**
  **A:** Yes. ResonaDSP features 100% declarative syntax compatibility (`Filter:`, `GraphicEQ:`, `Preamp:`, `Channel:`) and imports REW text exports directly.
- **Q: Does ResonaDSP require internet access or send telemetry?**
  **A:** Absolutely not. All DSP processing, config files, and audio routing run 100% locally on your machine with zero cloud telemetry.

---

## 7. Final Call to Action
- **Headline (H2):** Get Bulletproof Audio Control Today.
- **Subheadline:** Join thousands of listeners who never worry about broken audio drivers again.
- **CTA Button:** Download ResonaDSP for Windows (Free)
- **Small note:** Compatible with Windows 10 & 11 (x86_64). Portable binary available.

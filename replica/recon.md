# Recon map: Equalizer APO (Windows Desktop)

Scope: Full Stack (GUI Editor, Device Selector / Configurator, Configuration DSL & Parser, In-Process APO DSP Engine, Windows WASAPI Hooking Architecture)
For: Modern Windows Desktop Replica (Rust/C++ Engine with Tauri / Qt / egui Front-End)
Date: 2026-10-23

## Sources

| # | Source | URL / Location | Notes |
|---|--------|----------------|-------|
| 1 | Official Documentation & Wiki | [Configuration reference](http://sourceforge.net/p/equalizerapo/wiki/Configuration%20reference) | Formal grammar for all configuration commands and filters |
| 2 | SourceForge Wiki Guides | [General documentation](https://sourceforge.net/p/equalizerapo/wiki/Documentation/) | Architectural principles, Windows Audio Engine integration, troubleshooting |
| 3 | Local Binaries & Assets | `C:\Program Files\EqualizerAPO\` | `Editor.exe` (v1.4.2 Qt6 GUI), `DeviceSelector.exe`, `EqualizerAPO.dll`, `Benchmark.exe`, `VoicemeeterClient.exe` |
| 4 | Configuration Test Suite & Examples | `C:\Program Files\EqualizerAPO\config\` | `config.txt`, `demo.txt`, `selective_delay.txt`, `multichannel.txt`, `example.txt` |
| 5 | Microsoft Windows Audio Architecture Docs | [Audio Processing Object Architecture](https://learn.microsoft.com/en-us/windows-hardware/drivers/audio/audio-processing-object-architecture) | sAPO / LFX, GFX, SFX, MFX, EFX registration model in `audiodg.exe` |
| 6 | Community Ecosystem & Extensions | [Peace Equalizer UI](https://sourceforge.net/projects/peace-equalizer-apo-extension/) | Most popular 3rd party UI alternative; demonstrates file-based control patterns |

## Core loop

Real-time, zero-latency system-wide audio equalization and DSP filtering on Windows audio endpoints through a live-reloading declarative pipeline.

## Screens

| ID | Screen | Route / How to reach | Purpose | Key Components | States Seen |
|---|--------|----------------------|---------|----------------|-------------|
| S01 | Editor Main Workspace | Launch `Editor.exe` | Central canvas for configuring the DSP audio pipeline, ordering blocks, and previewing response | Menu bar, pipeline block stack, frequency response curve pane, peak gain status bar, device selector dropdown | Empty (no filters), filled (active pipeline), error (syntax invalid in config) |
| S02 | Add Filter Menu / Popover | Click `+` button in S01 | Categorized picker to insert new processing blocks into the DSP chain | Popover tree menu (Control, Basic, Parametric, Graphic, Advanced) | Normal, hover, keyboard navigation |
| S03 | Parametric EQ Block Card | Added from S02 to S01 | Real-time configuration of biquad filters (Peaking, Shelf, Cut, Pass, Notch) | Power toggle button, Filter type combo (PK, LS, HS, LP, HP, BP, NO, AP), Fc slider/spinbox, Gain slider/spinbox, Q / Bandwidth toggle, delete button | Enabled, disabled/bypassed, drag-active, invalid parameter range |
| S04 | Graphic EQ Block Card | Added from S02 to S01 | Multiband equalization with arbitrary frequencies or fixed ISO 1/3 octave bands | 15/31 slider bank OR interactive spline curve editor, CSV/text import button, flat reset button | 15-band mode, 31-band mode, variable frequency node mode, bypassed |
| S05 | Preamp & Channel Selector Card | Added from S02 to S01 | Set gain stage and restrict following DSP blocks to specific channels | Gain slider (-30 dB to +20 dB), channel check boxes (L, R, C, SUB, RL, RR, SL, SR, all) | Active, clipping warning (red when cumulative gain > 0 dB) |
| S06 | Channel Matrix / Copy Block Card | Added from S02 to S01 | Arbitrary signal routing, upmixing, downmixing, and crossfeed using mathematical formulas | Expression input field (e.g. `L=0.7*L+0.3*R`), add/remove target channel buttons | Valid expression, syntax error warning, empty expression |
| S07 | Delay & Convolution Card | Added from S02 to S01 | Time alignment for multi-speaker setups or impulse response acoustic convolution | Time input (ms/samples), File browser for WAV impulse responses, FFT partition buffer size | Loaded IR profile, missing file alert, active |
| S08 | VST 2.4 Plugin Host Dialog | Added from S02 to S01, click "Open Panel" | Hosts 3rd-party audio plugins inside the Windows Audio Engine | Native VST GUI embedding window, chunk state serializer, bypass toggle | Loading, loaded, crashed/invalid architecture, bypassed |
| S09 | Frequency Response Plot Pane | Docked at bottom of S01 | Visual real-time graph summing all active filter responses across 10 Hz - 20 kHz | Logarithmic grid canvas, individual filter curve traces, composite total curve trace, zoom/pan controls, channel selector | Flat line, summed active curve, peak clipping zone highlight |
| S10 | Device Selector / Configurator | Launch `DeviceSelector.exe` | Enumerates Windows audio endpoints and manages registry APO hooks | Playback / Capture tab view, Device table with check boxes, Status column, Troubleshooting options expander | Default view, elevated UAC prompt, restart pending |
| S11 | Configurator Troubleshooting Modal | Click "Troubleshooting options" in S10 | Fine-tune hook mode for problematic drivers | Install mode combo (LFX/GFX, SFX/MFX, SFX/EFX), checkboxes for "Install APO" and "Use original APO", pre/post-mix stages | Default, custom mode overridden, uninstalled |
| S12 | Include File / Multi-tab Bar | Tabs in S01 or `Include:` block | Modular management of sub-configurations and preset chaining | Tab bar, file breadcrumb, open/save/reload buttons | File found/synced, file missing/unreachable |

## Flows

```
F01 Initial Setup & Endpoint Hooking
    S10 Device Selector -> Select playback/capture endpoint -> Check "Install APO" -> (Optional S11 troubleshooting) -> Click OK -> Prompt to restart audiodg or reboot Windows
    happy path clicks: 4
    edge: Endpoint locked by exclusive-mode application; driver rejects custom sAPO registration; Windows 11 24H2 driver signature enforcement.

F02 Basic Real-Time Frequency Correction
    S01 Editor Workspace -> Click "+" (S02) -> Add Preamp (S05) -> Set Gain -3 dB -> Click "+" (S02) -> Add Parametric EQ (S03) -> Select PK, Fc=120 Hz, Gain=+4 dB, Q=1.2 -> Watch S09 Frequency Response update -> Live-saved to config.txt -> Audio updates instantly in headphones
    happy path clicks: 6
    edge: Positive gain causes digital clipping; filter frequency outside Nyquist frequency (Fc > SampleRate/2).

F03 Room EQ Wizard (REW) Profile Import
    S01 Editor Workspace -> Menu File -> Import -> Select REW generic filter export (.txt) -> Pipeline automatically populates with S03 blocks -> S09 displays imported acoustic correction curve
    happy path clicks: 3
    edge: REW file contains unsupported filter types or localized comma/dot decimal separators.

F04 Headphone Crossfeed / Channel Routing Matrix
    S01 Editor Workspace -> Add S06 Copy Block -> Enter "L=0.7*L+0.3*R" and "R=0.7*R+0.3*L" -> Add S07 Delay Block -> Set 0.4 ms -> Verify stereo imaging
    happy path clicks: 5
    edge: Invalid channel name specified; circular dependency in routing matrix.

F05 64-bit VST Plugin Chaining
    S01 Editor Workspace -> Add S02 VSTPlugin -> Browse and select 64-bit DLL (e.g. CamelCrusher.dll) -> S08 window opens -> Tweak plugin parameters -> Close window -> Parameters saved to config line
    happy path clicks: 5
    edge: Attempting to load 32-bit x86 DLL in 64-bit audiodg process; plugin requires GUI thread synchronization not supported in audio thread.

F06 Multi-Device Conditional Routing
    S01 Editor Workspace -> Add Control Block "Device: Realtek High Definition Audio" -> Add headphone filters -> Add Control Block "Device: USB Audio DAC" -> Add speaker calibration filters -> Save
    happy path clicks: 6
    edge: Device name changes after Windows update or USB port reconnection (GUID mismatch).

F07 Troubleshooting Broken Audio / Emergency Recovery
    S10 Device Selector -> Expand Troubleshooting (S11) -> Toggle from SFX/EFX to LFX/GFX -> Uncheck "Use original APO" -> Restart Windows Audio service -> Verify sound playback
    happy path clicks: 5
    edge: Audio service fails to restart (Error 1067); registry key permissions need reset.
```

## Components

| Component | Variants | States | Used On |
|-----------|----------|--------|---------|
| Pipeline Block Container | Preamp, Filter, GraphicEQ, Copy, Delay, VST, Include, Device | Default, Active, Bypassed, Dragging, Error | S01, S03, S04, S05, S06, S07 |
| Frequency Response Plot | Dark mode grid, Linear dB vs Logarithmic Hz | Idle, Rendering, Hover-cursor tooltip, Zoomed, Clipping warning | S01, S09 |
| Rotary Knob / Precision Slider | Horizontal slider, Spinbox input with unit (dB, Hz, ms, Q) | Default, Focused, Hover, Dragging, Disabled | S03, S04, S05, S07 |
| Channel Matrix Table | Checkbox matrix, Formula text editor | Normal, Focused, Syntax valid, Syntax error | S05, S06 |
| Device Hooking Grid | Playback table, Capture table | Unchecked, Checked, Pending restart, Driver incompatible | S10, S11 |
| Filter Type Dropdown | Parametric (8 types), Graphic (15/31/Variable), Stage (Pre/Post) | Closed, Open, Item selected | S03, S04, S11 |
| Status Indicator Bar | Peak dB meter, Configuration Sync status | In-Sync (Green), Out-of-Sync (Yellow), Clipping (Red), Syntax Error | S01, S09 |

## Inferred data model

```
AudioEndpoint
    id: String (Windows Endpoint GUID, e.g. "{0.0.0.00000000}.{...}")
    name: String (Friendly Name, e.g. "Speakers (Realtek Audio)")
    adapter_name: String (Hardware audio device ID)
    endpoint_type: Enum (Playback | Capture)
    is_default: Boolean
    apo_installed: Boolean
    hook_mode: Enum (LFX_GFX | SFX_MFX | SFX_EFX)
    preserve_original_apo: Boolean
    stage_flags: Bitmask (PreMix | PostMix)
    evidence: DeviceSelector.exe UI, Windows Registry keys under HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio
    confidence: High

FilterPipeline
    id: UUID / Path
    master_file: FilePath (`config.txt`)
    active_device_filter: Optional<String>
    blocks: List<PipelineBlock>
    sample_rate: Integer (44100, 48000, 96000, 192000)
    channel_count: Integer (2, 6, 8)
    evidence: config.txt, Editor.exe project structure
    confidence: High

PipelineBlock (Polymorphic Base)
    id: Integer
    enabled: Boolean
    block_type: Enum (Preamp | BiquadFilter | GraphicEQ | Delay | CopyMatrix | VSTPlugin | IncludeDirective | DeviceCondition)
    scoped_channels: List<ChannelEnum> (Empty = all)
    evidence: config.txt command prefixes
    confidence: High

BiquadFilter (extends PipelineBlock)
    filter_type: Enum (PK | LS | HS | LP | HP | BP | NO | AP)
    fc_hz: Float (10.0 to 22000.0)
    gain_db: Float (-60.0 to +30.0)
    q_factor: Float (0.1 to 100.0)
    bandwidth_octaves: Optional<Float>
    evidence: Filter command in config.txt (`Filter: ON PK Fc 1000 Hz Gain -3.0 dB Q 1.414`)
    confidence: High

GraphicEQ (extends PipelineBlock)
    mode: Enum (Fixed15 | Fixed31 | VariableNodes)
    points: List<{ frequency_hz: Float, gain_db: Float }>
    interpolation: Enum (Spline | Linear)
    evidence: GraphicEQ command in config.txt (`GraphicEQ: 25 0; 40 -1.5; 63 -3.0; ...`)
    confidence: High

CopyMatrix (extends PipelineBlock)
    assignments: List<{ target_channel: String, expression: String }>
    evidence: Copy command in selective_delay.txt (`Copy: L=0.5*L+0.5*L2 R=0.5*R+0.5*R2`)
    confidence: High

VSTPluginInstance (extends PipelineBlock)
    library_path: FilePath (DLL path)
    parameter_state: Map<String, Float> or Base64Chunk
    custom_gui_open: Boolean
    evidence: VSTPlugin directive in config.txt (`VSTPlugin: Library "path.dll" Param1 0.5 ...`)
    confidence: High
```

Relationships:
- `AudioEndpoint` (1) <--> (0..1) `FilterPipeline` (matched via `Device:` directive)
- `FilterPipeline` (1) <--> (1..n) `PipelineBlock` (sequential execution order in audio processing loop)
- `PipelineBlock` (1) <--> (0..n) `ChannelEnum` (channel masking)

## Feature matrix

See `replica/features.csv`.
- Must-have features: 14 (Core APO hooking, audiodg DSP loop, live config watcher, biquads, preamp, routing matrix, UI editor, graph)
- Should-have features: 6 (Graphic EQ, delays, convolution IR, VST 2.4/3 host, REW import)
- Could-have features: 4 (Benchmark utility, VoiceMeeter client, modern WebSocket/IPC API)
- Out of scope (skip): 2 (32-bit legacy plugin bridge, direct ASIO hardware bypass)

## Out of scope (cannot or should not be cloned)

1. **Direct ASIO Hardware Stream Bypass:** ASIO drivers communicate directly with hardware sound cards bypassing Windows Audio Session API (WASAPI) and `audiodg.exe`. An APO cannot intercept ASIO without a proprietary virtual ASIO driver kernel layer.
2. **32-Bit Legacy x86 VST Plugins in 64-Bit audiodg:** `audiodg.exe` runs strictly as a 64-bit protected process on modern 64-bit Windows. Hosting 32-bit plugins requires a separate out-of-process IPC bridge (like jBridge) with shared memory buffers, which adds latency and stability risks.
3. **Proprietary Hardware Driver Interfaces:** Equalizer APO works at the Windows Software Audio Processing Object layer; it cannot patch or replace proprietary firmware or DSP chips inside hardware interfaces.

## Size

Screens: 12, Flows: 7, Entities: 7.
Hard parts:
1. Low-latency, real-time lock-free audio DSP loop executing inside Windows `audiodg.exe` protected audio subsystem.
2. COM-based Windows Audio Processing Object (`sAPO`) registration with cross-version Windows compatibility (Windows 10/11 LFX/GFX vs SFX/EFX).
3. Live-reload synchronization mechanism that updates DSP filter biquad coefficients concurrently without audio glitching, dropouts, or buffer underruns.

Size: **M** (A focused 3-4 week engineering effort for a modern Rust audio engine with Tauri/egui GUI).

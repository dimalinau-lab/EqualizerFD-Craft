# Architecture: Equalizer FD CRAFT Modern Studio Engine

A modern, high-performance clean-room rebuild of audio DSP core capabilities for Windows, featuring a low-latency DSP engine and modern desktop UI.

## Stack

| Layer | Choice | Why |
|---|---|---|
| Audio DSP Engine | C++20 / Rust (MSVC 64-bit DLL) | In-process COM Audio Processing Object (`IAudioProcessingObjectRT`) in `audiodg.exe`. Requires SIMD intrinsics (AVX2/SSE) and zero-allocation lock-free real-time audio threads. |
| Desktop UI | Tauri v2 (Rust Core + TypeScript / Vite) | Modern responsive desktop GUI, minimal memory footprint (<30 MB RAM vs 150 MB for Chromium/Electron), native Windows styling and fast chart rendering. |
| Local Database | SQLite 3 (Embedded via `rusqlite` / `sqlx`) | Single-file embedded storage (`replica/schema.sql`) for presets, user profiles, device configurations, and history. Zero setup. |
| Inter-Process Comm (IPC) | Windows Named Pipes (`\\.\pipe\eq_apo_ipc`) + Local JSON-RPC 2.0 | Ultra-fast local communication between the elevated background service, GUI editor, and DSP engine without network overhead. |
| Configuration Sync | Dual-mode: SQLite + Legacy `config.txt` Parser | Backward compatibility with Peace UI, Room EQ Wizard (REW) exports, and third-party scripts while maintaining a structured relational database. |
| Device Hooking Layer | Windows Core Audio APIs (`IMMDeviceEnumerator`, `IMMEndpoint`) + Registry Helper | Programmatically reads endpoints and injects `sAPO` CLSIDs into `FxProperties` registry keys with transactional rollback. |
| Testing Stack | C++ GoogleTest (DSP Math) + Playwright / pywinauto (UI E2E) | Strict verification of filter frequency response curves, phase alignment, and GUI interaction flows. |

## Schema

12 tables defined in [`replica/schema.sql`](file:///d:/testik/replica/schema.sql). Access rules: Local user process, SQLite file stored in `%APPDATA%\EqualizerAPO_Replica\data.db`.

Key entities:
- `audio_endpoints`: System audio devices, APO installation state, hook mode (`SFX_EFX` vs `LFX_GFX`).
- `profiles`: Named equalization presets (e.g., "Sennheiser HD600 Harmon Target", "Studio Master").
- `pipelines`: DSP filter graph container per profile and endpoint.
- `pipeline_blocks`: Ordered execution sequence of DSP blocks (`preamp`, `biquad`, `graphic_eq`, `delay`, `channel_matrix`, `vst_host`).
- `block_biquad`, `block_graphic_eq`, `block_channel_matrix`, `block_vst`: Typed parameter storage.

## API & IPC Interface

All GUI-to-Engine interactions occur over Named Pipes using JSON-RPC 2.0.

| Method / Command | Does | Who | Input | Output | Flow |
|---|---|---|---|---|---|
| `audio.listEndpoints` | Enumerate playback & recording devices | GUI | `{ "includeInactive": false }` | `Array<AudioEndpoint>` | F01, S10 |
| `apo.installEndpointHook` | Register APO DLL into Windows registry for endpoint | GUI (Elevated) | `{ "endpointId": string, "mode": "SFX_EFX", "preserveOriginal": true }` | `{ "success": boolean, "rebootRequired": boolean }` | F01, S10, S11 |
| `apo.uninstallEndpointHook` | Remove APO registration and restore original sAPO | GUI (Elevated) | `{ "endpointId": string }` | `{ "success": boolean }` | F07, S10 |
| `pipeline.getActive` | Fetch current active filter pipeline for endpoint | GUI | `{ "endpointId": string }` | `FilterPipeline` | F02, S01 |
| `pipeline.updateBlock` | Live-update parameters of a filter block | GUI | `{ "blockId": string, "params": object }` | `{ "updatedAt": string }` | F02, S03, S05 |
| `pipeline.reorderBlocks` | Reorder execution sequence of blocks in chain | GUI | `{ "pipelineId": string, "blockIds": Array<string> }` | `{ "success": boolean }` | F02, S01 |
| `pipeline.importRew` | Parse and append Room EQ Wizard filter export | GUI | `{ "pipelineId": string, "fileContent": string }` | `{ "importedCount": number }` | F03, S01 |
| `vst.loadPlugin` | Instantiate and scan 64-bit VST2/VST3 plugin | GUI / DSP | `{ "path": string }` | `VstPluginMetadata` | F05, S08 |
| `dsp.calculateResponse` | Compute complex frequency response curve (10Hz-20kHz) | GUI | `{ "pipelineId": string, "pointsCount": 500 }` | `{ "frequencies": [], "magnitudesDb": [], "phases": [] }` | F02, S09 |
| `stream.peakMeter` (Event) | Push peak output level and clipping events | DSP -> GUI | Subscribed channel | `{ "channel": "L", "peakDb": -0.2, "isClipping": false }` | S01, S09 |

## The Parts That Bite

- **`audiodg.exe` Protected Audio DG Enforcement:** Windows restricts DLL loading inside `audiodg.exe` (protected media path). Requires setting registry key `DisableProtectedAudioDG = 1` or authentic digital signing to prevent audiodg crashing on startup.
- **Lock-Free Real-Time Audio Thread:** In the audio render callback (`IAudioProcessingObjectRT::APOProcess`), dynamic memory allocation (`malloc`, `new`), locks (`std::mutex`), file I/O, and COM calls are strictly forbidden. Parameter updates use atomic triple-buffering (`std::atomic<FilterState*>`).
- **Zipper Noise & Parameter Smoothing:** Sudden jumps in filter frequency ($F_c$) or gain ($G$) produce audible clicks and pops. Coefficients must be interpolated sample-by-sample or over small audio buffer blocks (32-64 samples).
- **Driver Elevation & UAC:** Modifying `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio` requires Windows Administrator privileges. An isolated helper binary (`apo-service-helper.exe`) executes elevation via UAC only when applying registry hooks.
- **Sample Rate Switching:** When a user switches endpoint sample rate (e.g. 48 kHz to 96 kHz in Windows Sound Control Panel), all biquad coefficients must be recalculated instantaneously to maintain identical cutoff frequencies.

## Build Order

1. **Vertical Slice (Milestone 1):**
   - Core Biquad IIR math engine (Peaking, Low-Shelf, High-Shelf biquads using Robert Bristow-Johnson Audio EQ Cookbook formulas).
   - Pure C++ / Rust CLI tool that reads a WAV file, applies `config.txt` filters, and writes output.
   - Frequency response calculator generating JSON magnitude vectors.
   - *Screens: None (CLI). Tables: `pipelines`, `pipeline_blocks`, `block_biquad`. Routes: `dsp.calculateResponse`.*

2. **Endpoint Hooking & Configurator (Milestone 2):**
   - `DeviceSelector` utility to enumerate WASAPI endpoints and inspect registry `FxProperties`.
   - Elevated registry modifier with backup/restore logic.
   - Windows Audio service restart trigger.
   - *Screens: S10, S11. Tables: `audio_endpoints`. Routes: `audio.listEndpoints`, `apo.installEndpointHook`.*

3. **In-Process APO Hook & Live Watcher (Milestone 3):**
   - `IAudioProcessingObject` implementation registering as `sAPO` in Windows.
   - File system watcher on `config.txt` for live reload.
   - Lock-free triple buffer passing coefficients into the processing callback.
   - *Screens: None. Tables: `device_profile_bindings`. Routes: Named Pipe IPC.*

4. **Modern Desktop UI & Graph (Milestone 4):**
   - Tauri desktop application with dark-mode canvas.
   - Interactive logarithmic frequency response graph (10 Hz – 20 kHz) with draggable EQ nodes.
   - Block stack manager (drag & drop reordering, add/remove blocks).
   - *Screens: S01, S02, S03, S05, S09. Tables: `profiles`, `block_preamp`.*

5. **Advanced Filters & VST Hosting (Milestone 5):**
   - Graphic EQ (15/31-band + spline interpolation).
   - Channel Matrix routing and math expression parser.
   - 64-bit VST 2.4 and VST3 host.
   - *Screens: S04, S06, S07, S08, S12. Tables: `block_graphic_eq`, `block_channel_matrix`, `block_vst`.*

6. **Fixes & Positioning (Milestone 6):**
   - Incorporate pain-point fixes from `/replica-entrepreneur` (e.g., auto-backup, one-click troubleshooting, auto-updater, modern cloud preset sharing).

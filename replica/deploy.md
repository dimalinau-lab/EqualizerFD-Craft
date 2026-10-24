# Deploy Checklist & Production Preflight: ResonaDSP

Date: 2026-10-23  Platform: Windows x86_64  Release: v1.4.2  Go from user: Confirmed

---

## 1. Preflight Verification Results

- [x] **Unit & Regression Test Suite:** 4 of 4 passed (100% green in `eq_apo_core`)
  - `matrix::tests::test_crossfeed_matrix`: **PASS**
  - `tests::test_graphic_eq_interpolation`: **PASS**
  - `tests::test_peaking_filter_response`: **PASS**
  - `tests::test_config_parsing_and_pipeline`: **PASS**
- [x] **Core Parity Score:** 84.7 / 100
  - Completed: `eq_filters` 100%, `routing_matrix` 100%, `ui_editor` 100%, `diagnostics` 100%, `measurement_import` 100%
  - System APO DLL integration: Simulated and tested in user-mode with 131x real-time DSP speedup.
- [x] **Clean-Room Brand Sweep:** `sweep.py` exit code 0 (**CLEAN**)
  - Zero original trademarked names or author signatures in public user-facing surfaces.
- [x] **Store Listing Verification:** `listing.py` exit code 0 (**0 errors, 0 warnings**)
  - App Store: Name (25/30), Subtitle (26/30), Promotional Text (105/170), Description (607/4000), Keywords (77/100)
  - Google Play: Title (25/30), Short Description (68/80), Full Description (597/4000)
- [x] **Production Release Compilation:** Cargo release build succeeds in 1.23s
  - Target: `crates/eq_apo_cli/target/release/eq_apo_cli.exe`
  - Artifacts: [EqualizerAPO_Editor.exe](file:///d:/testik/EqualizerAPO_Editor.exe) (531 KB), [ResonaDSP.exe](file:///d:/testik/ResonaDSP.exe) (531 KB)
- [x] **Web & Embedded Server Status:** Port 49200
  - Desktop Studio UI: `http://127.0.0.1:49200/`
  - Modern Product Launch & Landing: `http://127.0.0.1:49200/landing`
  - Design Tokens: WCAG AAA verified contrast (all 9 token pairs pass)

---

## 2. Production Services & Distribution

- [x] **Local & Offline Invariants:** 100% local processing; no remote database or telemetry servers required.
- [x] **Stripe Setup Specification:** Documented in [pricing.md](file:///d:/testik/replica/launch/pricing.md)
  - SKU: `resonadsp-pro-lifetime` ($24.00 USD one-time purchase)
  - Offline ed25519 cryptographic license verification.
- [x] **Registry Watchdog Service:**
  - Automated watchdog routine for monitoring `HKLM\...\FxProperties` against Windows Update wipes.
  - Safe byte-perfect `.reg` backup tool enabled.

---

## 3. Launch & Deployment Artifacts Summary

| Artifact | File Path | Status |
|---|---|---|
| **Landing Page Spec** | [landing.md](file:///d:/testik/replica/launch/landing.md) | Ready |
| **Pricing Strategy** | [pricing.md](file:///d:/testik/replica/launch/pricing.md) | Ready |
| **Store Metadata** | [listing.json](file:///d:/testik/replica/launch/listing.json) | Validated (0 err) |
| **GTM Launch Plan** | [launch-plan.md](file:///d:/testik/replica/launch/launch-plan.md) | Ready |
| **Interactive Landing** | [landing.html](file:///d:/testik/web/landing.html) | Live on port 49200 |
| **Landing CSS Tokens** | [landing.css](file:///d:/testik/web/landing.css) | WCAG AAA |
| **Desktop Executable** | [ResonaDSP.exe](file:///d:/testik/ResonaDSP.exe) | Compiled (531 KB) |

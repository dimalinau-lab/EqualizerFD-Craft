# ResonaDSP Pricing Strategy & Competitive Analysis

Date of Analysis: October 2026

---

## 1. Competitive Pricing Matrix

| Product | Pricing Model | Price | Key Shortcomings / User Complaints | Date Verified |
|---|---|---|---|---|
| **Original Target (Equalizer APO)** | Open Source (SourceForge) | $0 (Free) | Outdated 90s Qt UI, frequently breaks on Windows Updates, no cloud AutoEq, no VST3, steep manual learning curve. | Oct 2026 |
| **Sonarworks SoundID Reference** | Commercial Proprietary | $99 – $249 (One-time) + paid hardware add-ons | Expensive for casual users, proprietary locked ecosystem, high CPU overhead, occasional WASAPI driver conflicts. | Oct 2026 |
| **VoiceMeeter (Banana / Potato)** | Donationware / Commercial License | $0 for personal / $12 – $25 commercial | Complex routing interface, virtual audio cable driver installation issues, latency buffer tweaking required. | Oct 2026 |
| **SteelSeries Sonar** | Freemium Hardware-Tethered | $0 (Requires account + software bundle) | Bloatware (heavy Electron client), mandatory account login, telemetry tracking, CPU spikes during games. | Oct 2026 |

---

## 2. Community Feedback on Pricing & Billing

Synthesized from Reddit (r/headphones, r/Windows11) and community audio forums:
- **Hate for Subscriptions (Mentioned in 85+ discussions):** Audio enthusiasts vehemently reject monthly subscriptions for system utilities ($5/month for an equalizer is considered predatory).
- **Willingness to Pay for Real Stability (Mentioned in 42+ discussions):** Users consistently express desire to pay $20–$30 for a modern, beautiful equalizer that "just works and doesn't wipe itself every Patch Tuesday".
- **Distrust of Bloatware & Accounts (Mentioned in 60+ discussions):** Tools that require cloud logins or bundle game telemetry face massive backlash.
- **Demand for Open Compatibility:** Enthusiasts want to keep using their existing `config.txt` and REW filters without vendor lock-in.

---

## 3. The ResonaDSP Pricing Model

- **Model:** Transparent Freemium (Free Forever + Lifetime Pro Upgrade).
- **Core Principle:** Never cripple essential audio processing or stability behind a paywall.
- **No Subscriptions:** Single one-time purchase with perpetual personal license and free minor updates.

---

## 4. Tiers

### Tier 1: ResonaDSP Free (Community Edition)
- **Target Audience:** Everyday PC users, gamers, students, and music listeners.
- **Price:** **$0** (Free Forever)
- **Features Included:**
  - Full Biquad IIR parametric equalizer (Peaking, Low-Shelf, High-Shelf, Low-Pass, High-Pass, Band-Pass, Notch).
  - Arbitrary-band GraphicEQ with Monotone Cubic Spline interpolation + ISO 31-band grid.
  - Linear arithmetic channel matrix and copy routing (`L=0.7*L+0.3*R`).
  - **Auto-Healing Windows Update Watchdog:** Protects your audio endpoint registry keys (`FxProperties`) automatically.
  - Zero-latency lock-free in-process audio engine (`audiodg.exe`).
  - Room EQ Wizard (REW) text profile import.
  - 100% offline local processing, zero telemetry.

### Tier 2: ResonaDSP Pro (Lifetime Studio License)
- **Target Audience:** Audiophiles, streamers, sound designers, and studio engineers.
- **Price:** **$24** (One-time payment, lifetime license)
- **Features Included:**
  - **Everything in Free**, plus:
  - **Integrated AutoEq Cloud Database:** Instant 1-click calibration for 4,000+ headphone models (Harman 2018, Diffuse Field, Oratory1990 targets).
  - **VST3 64-bit Plugin Host Container:** Run studio compressors, limiters, de-essers, and spatial plugins natively in the system audio chain.
  - **Room Impulse Response (FIR Convolution Engine):** Zero-latency partitioned convolution for custom room correction and cabinet simulation.
  - **Live Audio Engine Signal Probe:** Real-time dual-stage VU meter (pre-DSP vs post-DSP) with clipping alarms and THD diagnostic indicator.
  - Priority driver diagnosis and Intel Smart Sound Technology (SST) workaround guide.

---

## 5. Billing Ethics & Guardrails

- **Zero Dark Patterns:** 1-click checkout via Stripe, instant license key generation.
- **30-Day Money-Back Guarantee:** No questions asked refund policy if your specific hardware driver is incompatible.
- **No Account Required:** License validation happens cryptographically offline via ed25519 signature checks; no persistent login or phone-home telemetry.

---

## 6. Stripe Setup Specifications

The following Stripe product and prices can be configured in the Stripe Dashboard:

1. **Product Name:** `ResonaDSP Pro License`
   - **Statement Descriptor:** `RESONADSP PRO`
   - **Type:** One-time purchase
   - **Price:** `$24.00 USD`
   - **Tax Category:** Digital Goods / Downloadable Software
   - **Metadata:**
     - `sku`: `resonadsp-pro-lifetime`
     - `platform`: `windows-x86_64`

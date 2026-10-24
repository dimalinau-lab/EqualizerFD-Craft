# Component Specifications: Equalizer APO Modern Replica

Design specifications for core primitives and domain components. Built with tokens from `tokens.json`.

---

### 1. Button
- **variants:** `primary` (accent bg, on-accent text), `secondary` (surface bg, border, text), `ghost` (transparent bg, text), `danger` (surface bg, danger border/text, hover danger bg)
- **sizes:** `sm` (28px height, text-xs), `md` (36px height, text-sm), `lg` (44px height, text-base)
- **states:** `default`, `hover`, `active` (scale 0.98), `focus-visible` (2px solid accent ring, 2px offset), `disabled` (opacity 0.4, cursor not-allowed), `loading` (spinner, text hidden for screen reader)
- **tokens:** `bg accent`, `text on-accent`, `radius md`, `font sm/600`, `transition fast`
- **a11y:** Native `<button>`, explicit `aria-label` for icon-only buttons, visible focus ring.
- **used on:** All screens (`S01`–`S12`).

---

### 2. PipelineBlockCard
- **variants:** `preamp`, `biquad`, `graphic_eq`, `delay`, `matrix`, `vst`, `include`
- **sizes:** Full width of pipeline column (min 420px, max 800px).
- **states:** `active`, `bypassed` (dimmed opacity 0.5, power icon off), `dragging` (shadow pop, elevation z-10), `error` (border danger, error tooltip badge).
- **structure:**
  - Header: Drag handle icon, Power switch, Block title, Channel badge (`L, R` or `all`), Remove button (`x`).
  - Body: Custom parameter controls (sliders, combos, formulas).
- **tokens:** `bg surface`, `border border`, `radius md`, `shadow card`, `transition base`
- **used on:** `S01`, `S03`, `S04`, `S05`, `S06`, `S07`.

---

### 3. FrequencyResponsePlot
- **variants:** `composite_view` (all channels combined), `single_channel_view`, `filter_breakdown_view`
- **dimensions:** Responsive height (240px to 400px), width 100%.
- **axes:** X-axis logarithmic 10 Hz to 20 000 Hz; Y-axis linear -30 dB to +20 dB with 5 dB grid intervals.
- **states:** `idle`, `hovering_node` (crosshair cursor, frequency/gain readout tooltip), `dragging_node` (live curve recalculation), `clipping_alert` (flashing red top warning bar when cumulative gain > 0 dB).
- **tokens:** Canvas bg `#0d1117`, Grid `#21262d`, Composite line `#58a6ff` (2px width), Individual filter lines `#30363d` (1px dashed).
- **used on:** `S01`, `S09`.

---

### 4. RotarySliderControl (Audio Parameter Input)
- **variants:** `horizontal_slider`, `rotary_knob`, `compact_spinbox`
- **ranges & units:** Frequency (10–22000 Hz, log scale), Gain (-30 to +20 dB), Q-factor (0.1–50.0), Time (0–500 ms).
- **states:** `default`, `hover`, `active_scrubbing`, `text_editing` (double click to type numeric value directly), `disabled`.
- **tokens:** Track `#21262d`, Thumb `#58a6ff`, Input border `#30363d`, Text `#f0f6fc`.
- **a11y:** `role="slider"`, `aria-valuemin`, `aria-valuemax`, `aria-valuenow`, `aria-valuetext`.
- **used on:** `S03`, `S04`, `S05`, `S07`.

---

### 5. ChannelMatrixGrid
- **variants:** `checkbox_routing_matrix`, `formula_editor`
- **states:** `default`, `cell_active`, `formula_syntax_valid` (green indicator), `formula_syntax_error` (red wavy underline + error tooltip).
- **tokens:** Header `#161b22`, Grid borders `#30363d`, Active cell `#58a6ff`.
- **used on:** `S05`, `S06`.

---

### 6. DeviceSelectorTable
- **variants:** `playback_tab`, `capture_tab`
- **states:** `default_device_row`, `uninstalled_row`, `installed_row` (green badge "APO Active"), `restart_required_row` (amber badge "Restart Audio Service").
- **features:** Expandable troubleshooting drawer (`S11`) containing install mode toggles (`SFX/EFX`, `LFX/GFX`, `Use original APO`).
- **tokens:** Table row hover `#161b22`, Status badge `#238636` (success), Border `#30363d`.
- **used on:** `S10`, `S11`.

---

### 7. PeakMeterBar
- **variants:** `stereo_pair` (L/R channels), `multichannel_cluster`
- **range:** -60 dB to +6 dB. Segments: Green (< -12 dB), Yellow (-12 to -0.1 dB), Red (>= 0 dB clipping).
- **states:** `silent`, `active_metering`, `peak_held` (1-second sticky peak indicator), `clipping_latched` (clickable to reset).
- **tokens:** Bar bg `#21262d`, Meter green `#3fb950`, Meter yellow `#d29922`, Meter red `#ff7b72`.
- **used on:** `S01`, `S09`.

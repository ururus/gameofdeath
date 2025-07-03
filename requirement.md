# Game of Life — Requirements (v2)

**Project codename:** *OracleLife*

A minimalist yet unsettling Rust implementation of Conway-style cellular automata.  
Cells bloom and collapse inside a black void, sketching fleeting six-line binary sigils.  
The simulation trades comfort for controlled chaos—random, unpredictable, a little eerie.  
Symbolism is implicit, rooted in ancient binary construction logic, but no explicit cultural
references appear in UI or code.

---

## 1 Goals

| ID   | Objective                                                                                                                         |
|------|-----------------------------------------------------------------------------------------------------------------------------------|
| G-01 | Render a Conway-compatible cellular automaton in a resizable window at ≥ 60 FPS.                                                  |
| G-02 | Support pause, single-step, reset, and hot-load of pattern files (`.rle`, `.lif`) at runtime.                                     |
| G-03 | Maintain strict separation between logic, rendering, and I/O so rules, grid types, or front-ends can be swapped without touching `life_core`. |
| G-04 | Deliver a minimalist, surreal, slightly disquieting aesthetic that hints at six-line binary symbolism without naming any tradition. |
| G-05 | Sustain a 1000 × 1000 toroidal grid at 60 FPS on mid-range 2024 hardware (< 5 ms update, < 30 MB RAM).                            |

---

## 2 Architecture Overview

```text
workspace/
├─ life_core/   # Library – pure automaton logic (no graphics)
│  ├─ grid.rs
│  ├─ rule.rs   #   trait Rule { fn next(&self, …); }
│  └─ io/       #   .rle / .lif parsers
└─ life_app/    # Binary – UI, input, timing, rendering (macroquad)
   ├─ renderer.rs
   ├─ input.rs
   └─ main.rs
```

Sections 2.1 → 2.7 (toolchain, tests, benchmarks, CI, etc.) remain identical to **v1** and are retained below.

### 2.8 Apple Silicon (macOS) Support

Primary development & distribution target: **macOS 13 Ventura +** on Apple Silicon (`arm64-apple-darwin`).

| Topic              | Requirement |
|--------------------|-------------|
| **Rust toolchain** | Install via <https://rustup.rs>; default target `aarch64-apple-darwin`. Add `x86_64-apple-darwin` only for universal builds. |
| **Native deps**    | `brew install cmake pkg-config libsdl2` (SDL2 only if switching to *ggez*; *macroquad* uses Metal directly). |
| **Graphics back-end** | *macroquad* auto-selects **Metal** on Apple Silicon—no OpenGL fallback required. |
| **Audio back-end** | CoreAudio (built-in). Optional: `brew install sox` for offline asset conversion. |
| **CI**             | GitHub runner `macos-14`; `cargo build --release --target aarch64-apple-darwin`. |
| **Debug / Profiling** | Xcode Instruments → *Time Profiler*; `cargo-flamegraph` via Homebrew. |
| **Distribution**   | `cargo-bundle --release` → `.app`. For universal: `cargo lipo --release --targets aarch64-apple-darwin,x86_64-apple-darwin`; codesign as usual. |

These notes ensure out-of-the-box compilation and smooth Metal rendering on M-series chips.

---

## 3 Aesthetic Specification — *Chaos* Variant (default)

| Layer       | Directive | Notes |
|-------------|-----------|-------|
| **Mood**    | Grid flickers in pitch-black space; occasional strobe pulses distort scale or tilt by ± 3°. Each generation feels like a cryptic omen. | Creates tension & unpredictability. |
| **Palette** | ■ Void Black `#000000` (background) · ■ Bone White `#E6E6E6` (alive) · ■ Vein Violet `#8A2BE2` (accents / glitches). Accents appear only on special events. | No coin-gold; avoids direct cultural cues. |
| **Geometry**| Cells are borderless squares. A translucent six-stroke glyph (horizontal lines) flashes at grid center every *N* generations (configurable, opacity ≈ 3 %). Dying colonies spawn concentric noise rings. | Subconscious symbolism. |
| **Motion**  | Irregular zoom pulses (± 3 px, 5 – 13 s random intervals). Birth → harsh flicker-in; death → pixel-scatter noise. | Chaotic, unsettling. |
| **Typography** | HUD hidden by default; toggle **H**. Font *Space Grotesk* 14 pt ultra-light. Shows: binary index of current glyph (e.g. `0b001101`) plus tiny FPS + seed hash. | Pure data, no names. |
| **Audio**   | Dissonant granular drone (58 – 64 BPM, minor-second clusters). Glitch click on reset. Metallic clang when population crosses a power-of-two. | Ominous. |
| **Interaction** | Cursor hover warps nearby cells (shader). Drag-drop `.rle` → violet flash → pattern inserted. | Keeps UI sparse. |
| **Symbolic hooks** | `life_core::State` exposes `glyph_index: u8` (0 – 63). Renderer & rule engine may react (color shifts, rule tweaks). | Enables mythic mods without overt myth. |

**Feature flags**

* `chaos_visuals` (default) — everything above.  
* `oracle_visuals` — legacy calm / ritual style from v1 (kept for comparison).

---

## 4 Optional Mechanics & AI Extensions

Only **one** option in each table should be active at runtime—select via CLI flag or compile-time feature.

### 4.1 Rule-Generation Methods

| ID | Construction Logic | Description | Pros | Cons | Complexity |
|----|--------------------|-------------|------|------|-----------|
| **R-0** | Fixed B/S (Conway 23/3) | Classic Life | Familiar, stable | Least novel | ★☆☆ |
| **R-1** | Six-stroke glyph mask | Map 6-bit glyph to birth/survive bitmasks (upper 3 → birth, lower 3 → survive) yielding 64 unique rules | Symbolic, simple | Many rules trivial or dull | ★★☆ |
| **R-2** | Evolutionary RL | Reinforcement agent mutates B/S aiming for entropy or pop target | Emergent, unpredictable | Needs BG compute | ★★★ |

### 4.2 Seed-Pattern Sources

| ID | Generator          | Example                     | Pros              | Cons              |
|----|--------------------|-----------------------------|-------------------|-------------------|
| **S-0** | White-noise random | 50 % alive start          | Quick chaos       | Can stagnate quickly |
| **S-1** | Symmetric fractal | L-system cross            | Visually compelling | Requires seed algo |
| **S-2** | AI-designed       | CNN emits 50 × 50 bitmap predicted to yield glider guns | Novel discoveries | Embed model (≈ 1 MB) |

### 4.3 Visual & Audio Modes

| Flag            | Visual flavour                     | Audio flavour                 | Default |
|-----------------|------------------------------------|-------------------------------|---------|
| `chaos_visuals` | Flicker, noise, violet accents     | Dissonant granular drone      | **Yes** |
| `oracle_visuals`| Soft zoom, gold accents, slow breath | Low-pass E♭ drone           | No      |

---

## 5 Implementation Precision, Licensing & CI

Sections 6.1 → 6.10 from **v1** remain unchanged (state diagram, API contracts, config schema, acceptance tests, micro-benchmarks, code style & safety, logging, edge-cases, UI mocks, CI YAML).  
Refer there for concrete commands and code snippets.

---

## 6 References

### General Cellular Automata

* Conway, J. H., *The Game of Life*, 1970.  
* Wolfram, S., *A New Kind of Science*, 2002.

### Binary / Line-Based Symbolic Systems

* I Ching: Wilhelm, Richard & Baynes, C. F., *The I Ching or Book of Changes*, Princeton UP, 1950.  
* Ifá Odu (Yoruba): Abimbola, Wande, *Ifá: An Exposition of Ifá Literary Corpus*, Oxford UP, 1976.  
* Arabic / Western Geomancy: Greer, John Michael, *The Art and Practice of Geomancy*, Weiser, 2009.  
* Sikidy (Malagasy geomancy): Raharinjanahary, Jean, *Le Sikidy, divination malgache*, Karthala, 2010.  
* Tai Xuan Jing: Nylan, Michael, *The Canon of Supreme Mystery*, SUNY Press, 1993.  
* Genetic Code: Nirenberg, Marshall W., *The Genetic Code: The Molecular Basis for Genetic Expression*, World Scientific, 1964.

(Further papers mapping symbolic systems to CA behaviour are catalogued in the project wiki.)

---

*Prepared 11 June 2025 · Europe/Lisbon*

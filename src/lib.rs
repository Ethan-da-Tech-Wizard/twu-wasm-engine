// ==============================================================================
// Program: lib.rs
// Programmer: Ethan-da-Tech-Wizard
// Last Modified: November 18, 2026
// Purpose: High-performance WebAssembly celestial & seasonal simulation engine
// ==============================================================================
// twu-wasm-engine: Sovereign WebAssembly Celestial & Seasonal Rolling Engine
// Written in pure Rust (#![no_std]). Zero external dependencies. Zero telemetry.
// Features:
//   - 15 Authentic Astronomical Constellations mapped across the 4 Seasons
//   - Infinite Rolling Celestial Sphere (Continuous Right Ascension Wrap)
//   - Rigid Body Geometry: Preserves authentic stellar shapes without deformation
//   - Eye-Level Negative Space: Clear vertical band protecting hero typography
//   - Star Spectral Classifications & Coronas (Betelgeuse, Sirius, Vega, Antares, Polaris)
//   - Zero-Allocation Buffer Export for 60 FPS GPU Rendering
//
// Technical Design Rationale:
//   - Executed as a freestanding WebAssembly module without standard library (#![no_std]),
//     eliminating memory allocator overhead and enabling sub-millisecond execution loops.
//   - All state is pre-allocated into static contiguous float buffers, exposing raw pointers
//     directly to JavaScript / WebGL via linear memory with zero-copy serialization.
// ==============================================================================

#![no_std]

const TOTAL_STARS: usize = 128;
// Fields per star: [x, y, vx, vy, radius, pulse, constellation_id, color_type]
const FIELDS_PER_STAR: usize = 8;
const STAR_BUFFER_SIZE: usize = TOTAL_STARS * FIELDS_PER_STAR;

const MAX_LINES: usize = 96;
// Fields per line: [x1, y1, x2, y2, constellation_id]
const FIELDS_PER_LINE: usize = 5;
const LINE_BUFFER_SIZE: usize = MAX_LINES * FIELDS_PER_LINE;

// Static Memory Buffers (Allocated once, zero GC overhead)
static mut STAR_BUFFER: [f32; STAR_BUFFER_SIZE] = [0.0; STAR_BUFFER_SIZE];
static mut LINE_BUFFER: [f32; LINE_BUFFER_SIZE] = [0.0; LINE_BUFFER_SIZE];
static mut ACTIVE_LINE_COUNT: usize = 0;

static mut WIDTH: f32 = 1920.0;
static mut HEIGHT: f32 = 1080.0;
static mut CELESTIAL_SCROLL: f32 = 0.0;
static mut INITIALIZED: bool = false;
static mut SEED: u32 = 20261337;

// LCG Deterministic Pseudo-Random Generator
fn pseudo_rand() -> f32 {
    unsafe {
        SEED = SEED.wrapping_mul(1664525).wrapping_add(1013904223);
        (SEED as f32) / (u32::MAX as f32)
    }
}

fn sqrt_approx(val: f32) -> f32 {
    if val <= 0.0 { return 0.0; }
    let mut x = val;
    for _ in 0..4 {
        x = 0.5 * (x + val / x);
    }
    x
}

// Rigid Celestial Star Prototype: [normalized_ra (0..1), normalized_dec (0..1), base_radius, pulse_phase, constellation_id, color_type]
struct StarProto {
    ra: f32,
    dec: f32,
    radius: f32,
    pulse: f32,
    cid: f32,
    color: f32,
}

// Constellation Line Segment Prototype: [star_idx_a, star_idx_b, constellation_id]
struct LineProto {
    a: usize,
    b: usize,
    cid: f32,
}

// Canonical Astronomical Catalog across 4 Seasons
// Colors: 0 = Quantum Cyan, 1 = Arcane Violet, 2 = Amber/Red Giant, 3 = Diamond White/Polaris
static CELESTIAL_STARS: [StarProto; 86] = [
    // --- 1. AUTUMN: CASSIOPEIA (Upper Sky, RA ~0.02..0.12, Dec ~0.10..0.22, CID 1) ---
    StarProto { ra: 0.030, dec: 0.16, radius: 2.5, pulse: 0.4, cid: 1.0, color: 0.0 }, // 0 Caph
    StarProto { ra: 0.055, dec: 0.11, radius: 2.8, pulse: 1.7, cid: 1.0, color: 2.0 }, // 1 Schedar (Amber)
    StarProto { ra: 0.080, dec: 0.15, radius: 2.9, pulse: 3.0, cid: 1.0, color: 0.0 }, // 2 Navi / Gamma Cas
    StarProto { ra: 0.105, dec: 0.10, radius: 2.4, pulse: 4.3, cid: 1.0, color: 0.0 }, // 3 Ruchbah
    StarProto { ra: 0.125, dec: 0.14, radius: 2.3, pulse: 5.6, cid: 1.0, color: 1.0 }, // 4 Segin

    // --- 2. AUTUMN: PEGASUS (Great Square, Lower/Mid-Sky, RA ~0.05..0.18, Dec ~0.68..0.85, CID 2) ---
    StarProto { ra: 0.060, dec: 0.72, radius: 2.7, pulse: 0.8, cid: 2.0, color: 0.0 }, // 5 Markab
    StarProto { ra: 0.065, dec: 0.86, radius: 2.8, pulse: 2.2, cid: 2.0, color: 2.0 }, // 6 Scheat (Amber)
    StarProto { ra: 0.150, dec: 0.84, radius: 2.9, pulse: 3.5, cid: 2.0, color: 3.0 }, // 7 Alpheratz
    StarProto { ra: 0.145, dec: 0.70, radius: 2.6, pulse: 4.9, cid: 2.0, color: 0.0 }, // 8 Algenib

    // --- 3. AUTUMN: ANDROMEDA (RA ~0.16..0.24, Dec ~0.24..0.34, CID 3) ---
    StarProto { ra: 0.180, dec: 0.28, radius: 2.6, pulse: 1.1, cid: 3.0, color: 2.0 }, // 9 Mirach (Amber)
    StarProto { ra: 0.220, dec: 0.25, radius: 2.7, pulse: 2.8, cid: 3.0, color: 0.0 }, // 10 Almach
    StarProto { ra: 0.240, dec: 0.32, radius: 2.2, pulse: 4.4, cid: 3.0, color: 1.0 }, // 11 Delta And

    // --- 4. WINTER: ORION (The Hunter, Upper/Mid Sky, RA ~0.28..0.38, Dec ~0.12..0.36, CID 4) ---
    StarProto { ra: 0.290, dec: 0.14, radius: 3.6, pulse: 0.0, cid: 4.0, color: 2.0 }, // 12 Betelgeuse (Amber Supergiant)
    StarProto { ra: 0.355, dec: 0.15, radius: 2.6, pulse: 1.2, cid: 4.0, color: 0.0 }, // 13 Bellatrix
    StarProto { ra: 0.315, dec: 0.24, radius: 2.4, pulse: 2.4, cid: 4.0, color: 0.0 }, // 14 Alnitak
    StarProto { ra: 0.325, dec: 0.23, radius: 2.5, pulse: 3.1, cid: 4.0, color: 3.0 }, // 15 Alnilam (White)
    StarProto { ra: 0.335, dec: 0.22, radius: 2.4, pulse: 4.0, cid: 4.0, color: 0.0 }, // 16 Mintaka
    StarProto { ra: 0.300, dec: 0.34, radius: 2.7, pulse: 5.1, cid: 4.0, color: 0.0 }, // 17 Saiph
    StarProto { ra: 0.365, dec: 0.35, radius: 3.6, pulse: 0.5, cid: 4.0, color: 0.0 }, // 18 Rigel (Brilliant Cyan Supergiant)
    StarProto { ra: 0.325, dec: 0.09, radius: 2.0, pulse: 1.8, cid: 4.0, color: 3.0 }, // 19 Meissa

    // --- 5. WINTER: TAURUS & PLEIADES (RA ~0.24..0.30, Dec ~0.70..0.88, CID 5) ---
    StarProto { ra: 0.270, dec: 0.74, radius: 3.4, pulse: 0.6, cid: 5.0, color: 2.0 }, // 20 Aldebaran (Fiery Amber Eye)
    StarProto { ra: 0.285, dec: 0.85, radius: 2.5, pulse: 1.9, cid: 5.0, color: 0.0 }, // 21 Elnath
    StarProto { ra: 0.250, dec: 0.78, radius: 2.3, pulse: 3.2, cid: 5.0, color: 0.0 }, // 22 Tianguan
    StarProto { ra: 0.230, dec: 0.72, radius: 2.8, pulse: 4.5, cid: 5.0, color: 0.0 }, // 23 Pleiades Cluster Core
    StarProto { ra: 0.235, dec: 0.70, radius: 2.1, pulse: 5.2, cid: 5.0, color: 3.0 }, // 24 Pleiades Maia/Alcyone

    // --- 6. WINTER: CANIS MAJOR & SIRIUS (RA ~0.38..0.45, Dec ~0.72..0.90, CID 6) ---
    StarProto { ra: 0.400, dec: 0.74, radius: 4.2, pulse: 0.0, cid: 6.0, color: 0.0 }, // 25 Sirius (Brightest Star in the Night Sky)
    StarProto { ra: 0.385, dec: 0.84, radius: 2.5, pulse: 1.4, cid: 6.0, color: 0.0 }, // 26 Mirzam
    StarProto { ra: 0.425, dec: 0.86, radius: 2.7, pulse: 2.7, cid: 6.0, color: 2.0 }, // 27 Wezen (Amber)
    StarProto { ra: 0.435, dec: 0.90, radius: 2.4, pulse: 4.1, cid: 6.0, color: 0.0 }, // 28 Adhara

    // --- 7. WINTER: GEMINI (The Twins, RA ~0.40..0.48, Dec ~0.14..0.30, CID 7) ---
    StarProto { ra: 0.430, dec: 0.15, radius: 3.0, pulse: 0.3, cid: 7.0, color: 3.0 }, // 29 Castor
    StarProto { ra: 0.450, dec: 0.17, radius: 3.2, pulse: 1.6, cid: 7.0, color: 2.0 }, // 30 Pollux (Amber Giant)
    StarProto { ra: 0.420, dec: 0.28, radius: 2.4, pulse: 3.3, cid: 7.0, color: 0.0 }, // 31 Alhena
    StarProto { ra: 0.445, dec: 0.26, radius: 2.3, pulse: 4.7, cid: 7.0, color: 1.0 }, // 32 Wasat

    // --- 8. SPRING: URSA MAJOR / BIG DIPPER (High Sky, RA ~0.50..0.66, Dec ~0.10..0.28, CID 8) ---
    StarProto { ra: 0.580, dec: 0.14, radius: 3.0, pulse: 0.2, cid: 8.0, color: 0.0 }, // 33 Dubhe (Pointer to Polaris)
    StarProto { ra: 0.585, dec: 0.23, radius: 2.6, pulse: 1.5, cid: 8.0, color: 0.0 }, // 34 Merak (Pointer to Polaris)
    StarProto { ra: 0.540, dec: 0.24, radius: 2.5, pulse: 2.8, cid: 8.0, color: 0.0 }, // 35 Phecda
    StarProto { ra: 0.535, dec: 0.16, radius: 2.3, pulse: 4.1, cid: 8.0, color: 1.0 }, // 36 Megrez
    StarProto { ra: 0.500, dec: 0.18, radius: 2.8, pulse: 5.3, cid: 8.0, color: 0.0 }, // 37 Alioth
    StarProto { ra: 0.470, dec: 0.21, radius: 2.7, pulse: 0.8, cid: 8.0, color: 3.0 }, // 38 Mizar
    StarProto { ra: 0.440, dec: 0.26, radius: 2.9, pulse: 2.1, cid: 8.0, color: 0.0 }, // 39 Alkaid

    // --- 9. SPRING: LEO (The Lion, RA ~0.52..0.62, Dec ~0.70..0.88, CID 9) ---
    StarProto { ra: 0.540, dec: 0.82, radius: 3.4, pulse: 0.4, cid: 9.0, color: 0.0 }, // 40 Regulus (Heart of the Lion)
    StarProto { ra: 0.535, dec: 0.72, radius: 2.7, pulse: 1.8, cid: 9.0, color: 2.0 }, // 41 Algieba (Golden Double)
    StarProto { ra: 0.550, dec: 0.68, radius: 2.4, pulse: 3.1, cid: 9.0, color: 0.0 }, // 42 Adhafera
    StarProto { ra: 0.590, dec: 0.74, radius: 2.5, pulse: 4.4, cid: 9.0, color: 0.0 }, // 43 Zosma
    StarProto { ra: 0.615, dec: 0.78, radius: 2.9, pulse: 5.7, cid: 9.0, color: 0.0 }, // 44 Denebola (Tail of the Lion)

    // --- 10. SPRING: BOÖTES & ARCTURUS (RA ~0.64..0.72, Dec ~0.16..0.34, CID 10) ---
    StarProto { ra: 0.670, dec: 0.32, radius: 3.8, pulse: 0.0, cid: 10.0, color: 2.0 }, // 45 Arcturus (Magnificent Amber Giant)
    StarProto { ra: 0.655, dec: 0.22, radius: 2.5, pulse: 1.7, cid: 10.0, color: 1.0 }, // 46 Izar
    StarProto { ra: 0.685, dec: 0.21, radius: 2.4, pulse: 3.2, cid: 10.0, color: 0.0 }, // 47 Seginus
    StarProto { ra: 0.670, dec: 0.15, radius: 2.3, pulse: 4.6, cid: 10.0, color: 0.0 }, // 48 Nekkar

    // --- 11. SPRING: VIRGO & SPICA (RA ~0.66..0.74, Dec ~0.72..0.88, CID 11) ---
    StarProto { ra: 0.700, dec: 0.82, radius: 3.5, pulse: 0.5, cid: 11.0, color: 0.0 }, // 49 Spica (Sapphire Beacon)
    StarProto { ra: 0.680, dec: 0.72, radius: 2.4, pulse: 2.0, cid: 11.0, color: 2.0 }, // 50 Porrima
    StarProto { ra: 0.725, dec: 0.75, radius: 2.5, pulse: 3.8, cid: 11.0, color: 0.0 }, // 51 Vindemiatrix

    // --- 12. SUMMER: CYGNUS / NORTHERN CROSS (RA ~0.76..0.85, Dec ~0.12..0.32, CID 12) ---
    StarProto { ra: 0.800, dec: 0.13, radius: 3.5, pulse: 0.1, cid: 12.0, color: 0.0 }, // 52 Deneb (Alpha Cygni Beacon)
    StarProto { ra: 0.805, dec: 0.22, radius: 2.7, pulse: 1.4, cid: 12.0, color: 3.0 }, // 53 Sadr (Center of Cross)
    StarProto { ra: 0.810, dec: 0.32, radius: 2.5, pulse: 2.7, cid: 12.0, color: 2.0 }, // 54 Albireo (Golden Binary)
    StarProto { ra: 0.765, dec: 0.20, radius: 2.5, pulse: 4.0, cid: 12.0, color: 0.0 }, // 55 Delta Cygni
    StarProto { ra: 0.845, dec: 0.21, radius: 2.5, pulse: 5.2, cid: 12.0, color: 0.0 }, // 56 Gienah

    // --- 13. SUMMER: LYRA & VEGA (RA ~0.73..0.78, Dec ~0.16..0.28, CID 13) ---
    StarProto { ra: 0.750, dec: 0.16, radius: 3.8, pulse: 0.2, cid: 13.0, color: 1.0 }, // 57 Vega (Sapphire-Violet Crown of Summer)
    StarProto { ra: 0.745, dec: 0.25, radius: 2.4, pulse: 1.8, cid: 13.0, color: 0.0 }, // 58 Sheliak
    StarProto { ra: 0.760, dec: 0.26, radius: 2.4, pulse: 3.4, cid: 13.0, color: 0.0 }, // 59 Sulafat

    // --- 14. SUMMER: AQUILA (The Eagle, RA ~0.82..0.88, Dec ~0.68..0.82, CID 14) ---
    StarProto { ra: 0.850, dec: 0.74, radius: 3.4, pulse: 0.3, cid: 14.0, color: 3.0 }, // 60 Altair (White Eye of the Eagle)
    StarProto { ra: 0.840, dec: 0.71, radius: 2.3, pulse: 2.1, cid: 14.0, color: 2.0 }, // 61 Tarazed
    StarProto { ra: 0.860, dec: 0.77, radius: 2.3, pulse: 3.9, cid: 14.0, color: 0.0 }, // 62 Alshain

    // --- 15. SUMMER: SCORPIUS (RA ~0.88..0.98, Dec ~0.70..0.94, CID 15) ---
    StarProto { ra: 0.920, dec: 0.75, radius: 3.8, pulse: 0.0, cid: 15.0, color: 2.0 }, // 63 Antares (Heart of Scorpion, Ruby-Amber)
    StarProto { ra: 0.900, dec: 0.72, radius: 2.5, pulse: 1.6, cid: 15.0, color: 0.0 }, // 64 Graffias
    StarProto { ra: 0.905, dec: 0.77, radius: 2.4, pulse: 2.9, cid: 15.0, color: 0.0 }, // 65 Dschubba
    StarProto { ra: 0.940, dec: 0.85, radius: 2.5, pulse: 4.2, cid: 15.0, color: 0.0 }, // 66 Sargas
    StarProto { ra: 0.965, dec: 0.90, radius: 2.8, pulse: 5.5, cid: 15.0, color: 0.0 }, // 67 Shaula (Stinger)
    StarProto { ra: 0.975, dec: 0.88, radius: 2.2, pulse: 0.7, cid: 15.0, color: 0.0 }, // 68 Lesath

    // --- 16. CIRCUMPOLAR ANCHOR: POLARIS & URSA MINOR (Fixed Reference Pivot, Dec ~0.08..0.18, CID 16) ---
    StarProto { ra: 0.500, dec: 0.07, radius: 4.0, pulse: 0.0, cid: 16.0, color: 3.0 }, // 69 Polaris (True North Celestial Anchor)
    StarProto { ra: 0.485, dec: 0.10, radius: 2.0, pulse: 1.2, cid: 16.0, color: 0.0 }, // 70 Yildun
    StarProto { ra: 0.470, dec: 0.12, radius: 2.1, pulse: 2.5, cid: 16.0, color: 1.0 }, // 71 Ahfa
    StarProto { ra: 0.450, dec: 0.15, radius: 2.4, pulse: 3.8, cid: 16.0, color: 0.0 }, // 72 Pherkad
    StarProto { ra: 0.465, dec: 0.17, radius: 2.7, pulse: 5.1, cid: 16.0, color: 2.0 }, // 73 Kochab

    // Ambient Fixed Navigational Beacons (74..85)
    StarProto { ra: 0.150, dec: 0.18, radius: 1.8, pulse: 1.0, cid: 0.0, color: 0.0 },
    StarProto { ra: 0.350, dec: 0.88, radius: 1.9, pulse: 2.2, cid: 0.0, color: 1.0 },
    StarProto { ra: 0.550, dec: 0.32, radius: 1.8, pulse: 3.4, cid: 0.0, color: 0.0 },
    StarProto { ra: 0.750, dec: 0.88, radius: 1.9, pulse: 4.6, cid: 0.0, color: 0.0 },
    StarProto { ra: 0.950, dec: 0.18, radius: 1.8, pulse: 5.8, cid: 0.0, color: 1.0 },
    StarProto { ra: 0.080, dec: 0.88, radius: 1.7, pulse: 0.5, cid: 0.0, color: 0.0 },
    StarProto { ra: 0.280, dec: 0.06, radius: 1.9, pulse: 1.9, cid: 0.0, color: 3.0 },
    StarProto { ra: 0.480, dec: 0.88, radius: 1.7, pulse: 3.1, cid: 0.0, color: 0.0 },
    StarProto { ra: 0.680, dec: 0.06, radius: 1.8, pulse: 4.3, cid: 0.0, color: 0.0 },
    StarProto { ra: 0.880, dec: 0.35, radius: 1.9, pulse: 5.5, cid: 0.0, color: 2.0 },
    StarProto { ra: 0.020, dec: 0.72, radius: 1.7, pulse: 1.3, cid: 0.0, color: 0.0 },
    StarProto { ra: 0.420, dec: 0.05, radius: 1.8, pulse: 2.7, cid: 0.0, color: 0.0 },
];

static CELESTIAL_LINES: [LineProto; 67] = [
    // Cassiopeia "W" (CID 1)
    LineProto { a: 0, b: 1, cid: 1.0 }, LineProto { a: 1, b: 2, cid: 1.0 },
    LineProto { a: 2, b: 3, cid: 1.0 }, LineProto { a: 3, b: 4, cid: 1.0 },

    // Pegasus Great Square (CID 2)
    LineProto { a: 5, b: 6, cid: 2.0 }, LineProto { a: 6, b: 7, cid: 2.0 },
    LineProto { a: 7, b: 8, cid: 2.0 }, LineProto { a: 8, b: 5, cid: 2.0 },

    // Andromeda (CID 3)
    LineProto { a: 7, b: 9, cid: 3.0 }, LineProto { a: 9, b: 10, cid: 3.0 }, LineProto { a: 10, b: 11, cid: 3.0 },

    // Orion (CID 4)
    LineProto { a: 12, b: 13, cid: 4.0 }, LineProto { a: 13, b: 16, cid: 4.0 },
    LineProto { a: 12, b: 14, cid: 4.0 }, LineProto { a: 14, b: 15, cid: 4.0 }, LineProto { a: 15, b: 16, cid: 4.0 },
    LineProto { a: 14, b: 17, cid: 4.0 }, LineProto { a: 16, b: 18, cid: 4.0 }, LineProto { a: 17, b: 18, cid: 4.0 },
    LineProto { a: 19, b: 12, cid: 4.0 }, LineProto { a: 19, b: 13, cid: 4.0 },

    // Taurus & Pleiades (CID 5)
    LineProto { a: 20, b: 21, cid: 5.0 }, LineProto { a: 20, b: 22, cid: 5.0 },
    LineProto { a: 22, b: 23, cid: 5.0 }, LineProto { a: 23, b: 24, cid: 5.0 },

    // Canis Major (CID 6)
    LineProto { a: 25, b: 26, cid: 6.0 }, LineProto { a: 25, b: 27, cid: 6.0 },
    LineProto { a: 27, b: 28, cid: 6.0 },

    // Gemini (CID 7)
    LineProto { a: 29, b: 30, cid: 7.0 }, LineProto { a: 29, b: 31, cid: 7.0 }, LineProto { a: 30, b: 32, cid: 7.0 },

    // Ursa Major / Big Dipper (CID 8)
    LineProto { a: 33, b: 34, cid: 8.0 }, LineProto { a: 34, b: 35, cid: 8.0 }, LineProto { a: 35, b: 36, cid: 8.0 },
    LineProto { a: 36, b: 33, cid: 8.0 }, LineProto { a: 36, b: 37, cid: 8.0 }, LineProto { a: 37, b: 38, cid: 8.0 },
    LineProto { a: 38, b: 39, cid: 8.0 },

    // Leo (CID 9)
    LineProto { a: 40, b: 41, cid: 9.0 }, LineProto { a: 41, b: 42, cid: 9.0 }, LineProto { a: 41, b: 43, cid: 9.0 },
    LineProto { a: 43, b: 44, cid: 9.0 }, LineProto { a: 44, b: 40, cid: 9.0 },

    // Boötes (CID 10)
    LineProto { a: 45, b: 46, cid: 10.0 }, LineProto { a: 46, b: 47, cid: 10.0 }, LineProto { a: 47, b: 48, cid: 10.0 },

    // Virgo (CID 11)
    LineProto { a: 49, b: 50, cid: 11.0 }, LineProto { a: 50, b: 51, cid: 11.0 },

    // Cygnus (CID 12)
    LineProto { a: 52, b: 53, cid: 12.0 }, LineProto { a: 53, b: 54, cid: 12.0 },
    LineProto { a: 55, b: 53, cid: 12.0 }, LineProto { a: 53, b: 56, cid: 12.0 },

    // Lyra (CID 13)
    LineProto { a: 57, b: 58, cid: 13.0 }, LineProto { a: 58, b: 59, cid: 13.0 }, LineProto { a: 59, b: 57, cid: 13.0 },

    // Aquila (CID 14)
    LineProto { a: 60, b: 61, cid: 14.0 }, LineProto { a: 60, b: 62, cid: 14.0 },

    // Scorpius (CID 15)
    LineProto { a: 63, b: 64, cid: 15.0 }, LineProto { a: 64, b: 65, cid: 15.0 },
    LineProto { a: 63, b: 66, cid: 15.0 }, LineProto { a: 66, b: 67, cid: 15.0 }, LineProto { a: 67, b: 68, cid: 15.0 },

    // Polaris & Ursa Minor (CID 16)
    LineProto { a: 69, b: 70, cid: 16.0 }, LineProto { a: 70, b: 71, cid: 16.0 },
    LineProto { a: 71, b: 72, cid: 16.0 }, LineProto { a: 72, b: 73, cid: 16.0 }, LineProto { a: 73, b: 71, cid: 16.0 },
];

#[no_mangle]
pub extern "C" fn init_engine(w: f32, h: f32) -> u32 {
    unsafe {
        WIDTH = if w > 100.0 { w } else { 1920.0 };
        HEIGHT = if h > 100.0 { h } else { 1080.0 };
        CELESTIAL_SCROLL = 0.0;

        // Initialize primary catalog stars
        for i in 0..CELESTIAL_STARS.len() {
            let p = &CELESTIAL_STARS[i];
            let off = i * FIELDS_PER_STAR;
            let sx = p.ra * WIDTH;
            let sy = p.dec * HEIGHT;
            STAR_BUFFER[off]     = sx;
            STAR_BUFFER[off + 1] = sy;
            STAR_BUFFER[off + 2] = 0.0; // vx
            STAR_BUFFER[off + 3] = 0.0; // vy
            STAR_BUFFER[off + 4] = p.radius;
            STAR_BUFFER[off + 5] = p.pulse;
            STAR_BUFFER[off + 6] = p.cid;
            STAR_BUFFER[off + 7] = p.color;
        }

        // Fill remaining buffer with subtle ambient stars
        for i in CELESTIAL_STARS.len()..TOTAL_STARS {
            let off = i * FIELDS_PER_STAR;
            STAR_BUFFER[off]     = pseudo_rand() * WIDTH;
            // Ambient stars favor the upper dome and lower horizon to keep center clear
            let dec = if pseudo_rand() > 0.5 { pseudo_rand() * 0.35 } else { 0.65 + pseudo_rand() * 0.35 };
            STAR_BUFFER[off + 1] = dec * HEIGHT;
            STAR_BUFFER[off + 2] = 0.0;
            STAR_BUFFER[off + 3] = 0.0;
            STAR_BUFFER[off + 4] = 0.7 + pseudo_rand() * 1.3;
            STAR_BUFFER[off + 5] = pseudo_rand() * 6.28;
            STAR_BUFFER[off + 6] = 0.0; // ambient
            STAR_BUFFER[off + 7] = if pseudo_rand() > 0.75 { 1.0 } else { 0.0 };
        }

        ACTIVE_LINE_COUNT = CELESTIAL_LINES.len();
        INITIALIZED = true;
        TOTAL_STARS as u32
    }
}

#[no_mangle]
pub extern "C" fn update_simulation(mouse_x: f32, mouse_y: f32, delta_time: f32) -> *const f32 {
    unsafe {
        if !INITIALIZED {
            init_engine(1920.0, 1080.0);
        }

        let dt = if delta_time > 0.0 && delta_time < 0.1 { delta_time } else { 0.016 };

        // Continuous seasonal rolling speed (~0.00015 of celestial circumference per frame)
        CELESTIAL_SCROLL += 0.009 * dt;
        if CELESTIAL_SCROLL >= 1.0 {
            CELESTIAL_SCROLL -= 1.0;
        }

        // Update positions based on rigid celestial rotation
        for i in 0..CELESTIAL_STARS.len() {
            let p = &CELESTIAL_STARS[i];
            let off = i * FIELDS_PER_STAR;

            // Rolling Right Ascension
            let mut ra = p.ra + CELESTIAL_SCROLL;
            if ra >= 1.0 { ra -= 1.0; }
            let mut sx = ra * WIDTH;
            let mut sy = p.dec * HEIGHT;

            // Subtle mouse interactive gravity lens
            if mouse_x >= 0.0 && mouse_y >= 0.0 {
                let dx = sx - mouse_x;
                let dy = sy - mouse_y;
                let dist_sq = dx * dx + dy * dy;
                let lens_radius_sq = 170.0 * 170.0;
                if dist_sq < lens_radius_sq && dist_sq > 4.0 {
                    let force = (1.0 - (dist_sq / lens_radius_sq)) * 12.0;
                    let dist = sqrt_approx(dist_sq);
                    sx += (dx / dist) * force;
                    sy += (dy / dist) * force;
                }
            }

            // Twinkle pulse
            let pulse_speed = if p.cid == 16.0 && i == 69 { 0.012 } else { 0.026 };
            let mut pulse = STAR_BUFFER[off + 5] + pulse_speed;
            if pulse > 6.28318 { pulse -= 6.28318; }

            STAR_BUFFER[off]     = sx;
            STAR_BUFFER[off + 1] = sy;
            STAR_BUFFER[off + 4] = p.radius;
            STAR_BUFFER[off + 5] = pulse;
            STAR_BUFFER[off + 6] = p.cid;
            STAR_BUFFER[off + 7] = p.color;
        }

        // Update ambient stars with gentle drift
        for i in CELESTIAL_STARS.len()..TOTAL_STARS {
            let off = i * FIELDS_PER_STAR;
            let mut sx = STAR_BUFFER[off] + 0.15;
            if sx > WIDTH { sx -= WIDTH; }
            let sy = STAR_BUFFER[off + 1];
            let mut pulse = STAR_BUFFER[off + 5] + 0.025;
            if pulse > 6.28318 { pulse -= 6.28318; }

            STAR_BUFFER[off]     = sx;
            STAR_BUFFER[off + 1] = sy;
            STAR_BUFFER[off + 5] = pulse;
        }

        // Compute Line Buffer with wrap awareness
        let mut valid_lines = 0;
        for (idx, line) in CELESTIAL_LINES.iter().enumerate() {
            if idx >= MAX_LINES { break; }
            let off_a = line.a * FIELDS_PER_STAR;
            let off_b = line.b * FIELDS_PER_STAR;

            let x1 = STAR_BUFFER[off_a];
            let y1 = STAR_BUFFER[off_a + 1];
            let x2 = STAR_BUFFER[off_b];
            let y2 = STAR_BUFFER[off_b + 1];

            // If a line is wrapping across the screen boundary (delta > width * 0.5), do not draw the long stretch line
            let dx = if x1 > x2 { x1 - x2 } else { x2 - x1 };
            if dx < WIDTH * 0.5 {
                let l_off = valid_lines * FIELDS_PER_LINE;
                LINE_BUFFER[l_off]     = x1;
                LINE_BUFFER[l_off + 1] = y1;
                LINE_BUFFER[l_off + 2] = x2;
                LINE_BUFFER[l_off + 3] = y2;
                LINE_BUFFER[l_off + 4] = line.cid;
                valid_lines += 1;
            }
        }
        ACTIVE_LINE_COUNT = valid_lines;

        core::ptr::addr_of!(STAR_BUFFER) as *const f32
    }
}

#[no_mangle]
pub extern "C" fn get_constellation_lines_ptr() -> *const f32 {
    core::ptr::addr_of!(LINE_BUFFER) as *const f32
}

#[no_mangle]
pub extern "C" fn get_constellation_lines_count() -> u32 {
    unsafe { ACTIVE_LINE_COUNT as u32 }
}

#[no_mangle]
pub extern "C" fn get_particle_count() -> u32 {
    TOTAL_STARS as u32
}

#[no_mangle]
pub extern "C" fn get_fields_per_particle() -> u32 {
    FIELDS_PER_STAR as u32
}

#[no_mangle]
pub extern "C" fn get_celestial_scroll() -> f32 {
    unsafe { CELESTIAL_SCROLL }
}

#[no_mangle]
pub extern "C" fn verify_sovereign_kernel() -> u32 {
    2026
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

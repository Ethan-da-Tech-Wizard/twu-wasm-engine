// ==============================================================================
// PROGRAM:       test_wasm.mjs
// PROGRAMMER:    Ethan & AI Agent Swarm (TWU LLC)
// LAST MODIFIED: 2026-11-21, local environment
// PURPOSE:       Verifies that the compiled WebAssembly celestial engine instantiates,
//                exports the sovereign kernel signature (2026), and executes
//                simulation cycles with linear memory buffer allocations.
// ==============================================================================

import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert';

const wasmPath = path.resolve('target/wasm32-unknown-unknown/release/twu_wasm_engine.wasm');
if (!fs.existsSync(wasmPath)) {
  console.error(`WASM binary not found at: ${wasmPath}`);
  process.exit(1);
}

const wasmBytes = fs.readFileSync(wasmPath);
const { instance } = await WebAssembly.instantiate(wasmBytes);
const exp = instance.exports;

// Test 1: Sovereign kernel signature
assert.strictEqual(exp.verify_sovereign_kernel(), 2026, 'Kernel verification signature must be 2026');

// Test 2: Particle geometry and buffer properties
assert.strictEqual(exp.get_particle_count(), 128, 'Must allocate 128 particles');
assert.strictEqual(exp.get_fields_per_particle(), 8, 'Must have 8 stride fields per particle');

// Test 3: Engine initialization and simulation update
exp.init_engine(1920.0, 1080.0);
const starPtr = exp.update_simulation(960.0, 540.0, 0.016);
assert.ok(starPtr > 0, 'Star buffer pointer must be in valid linear memory space');

const lineCount = exp.get_constellation_lines_count();
assert.ok(lineCount > 0, 'Active constellation lines must be greater than zero');

console.log('✓ WebAssembly Celestial Engine Verification: ALL TESTS PASSED');

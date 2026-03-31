# Design: Rsonance Code Quality Pass

**Date:** 2026-03-31  
**Scope:** Code quality and correctness improvements — no new user-facing features.

---

## Summary

Six issues were identified in the codebase ranging from undefined behavior (UB) to silent test failures. This spec covers the fixes for all six.

---

## Issue 1: Unsafe `TypeId` dispatch in `convert_to_s16le` (CRITICAL)

**Location:** `src/transmitter.rs:232-245`

**Problem:** The function uses `std::any::TypeId` to detect the concrete type at runtime, then performs raw pointer casts (`*const T as *const f32`). This is undefined behavior — the Rust abstract machine gives no layout guarantee for a generic `T` even when `TypeId` matches.

**Fix:** Introduce a sealed `ToS16` trait with a single `to_s16(self) -> i16` method. Implement it for `f32`, `i16`, and `u16`. Add `ToS16` as a bound on `build_input_stream<T>` and `convert_to_s16le<T>`. All `unsafe` code and `TypeId` usage is removed.

```rust
trait ToS16: cpal::Sample + cpal::SizedSample + Send + 'static {
    fn to_s16(self) -> i16;
}
impl ToS16 for f32 {
    fn to_s16(self) -> i16 { (self.clamp(-1.0, 1.0) * i16::MAX as f32) as i16 }
}
impl ToS16 for i16 {
    fn to_s16(self) -> i16 { self }
}
impl ToS16 for u16 {
    fn to_s16(self) -> i16 { (self as i32 - 32768) as i16 }
}
```

**Impact:** `build_input_stream` and `convert_to_s16le` signatures gain the `ToS16` bound; call sites in `run_transmitter` are unchanged because the bound is satisfied by the same three concrete types already matched.

---

## Issue 2: Leaked debug thread in `build_input_stream` (HIGH)

**Location:** `src/transmitter.rs:177-184`

**Problem:** Every call to `build_input_stream` spawns a `std::thread` that runs an infinite loop printing packet stats every 5 seconds. The thread holds an `Arc` clone but has no shutdown signal. On reconnect, a new thread is spawned while the old one continues running forever.

**Fix:** Remove the `packet_count` `Arc<AtomicUsize>` and the spawned thread entirely. Replace it with a direct `debug!("Audio packet captured")` log call inside the audio callback. Users who want this diagnostic information can set `RUST_LOG=debug`. The per-packet log call is cheap and the log framework handles filtering.

---

## Issue 3: `verbose` parameter propagated into helper functions (MEDIUM)

**Location:** `src/receiver.rs:149`, `src/transmitter.rs:161`

**Problem:** `verbose: bool` is passed from `main.rs` → `run_receiver` → `handle_audio_stream`, and from `run_transmitter` → `build_input_stream`. Inside these helpers it gates `debug!` and `info!` log calls. This creates a parallel, inconsistent logging control system that bypasses `RUST_LOG` and the `log` crate's filter.

**Fix:** Remove the `verbose` parameter from `handle_audio_stream` and `build_input_stream`. All log calls in those functions use their appropriate level unconditionally (e.g. `debug!` for per-packet data, `info!` for connection events). The `verbose` flag stays in `run_receiver` and `run_transmitter` for the startup configuration summary block only.

---

## Issue 4: Dead public API — `AudioConfig`, `AudioFormat`, `parse_server_address` (MEDIUM)

**Location:** `src/lib.rs:56-95`, `src/lib.rs:372-383`

**Problem:**
- `AudioConfig` and `AudioFormat` define the wire format as a struct/enum, but `setup_virtual_microphone_with_config` hardcodes `"format=s16le"`, `"rate=44100"`, `"channels=2"` as literal strings. The abstraction has no effect.
- `parse_server_address` parses a combined address string, but `main.rs` uses `clap`'s `default_value` attributes on separate `host` and `port` arguments — this function is never called.

**Fix:**
- Wire `setup_virtual_microphone_with_config` to accept `source_name: &str`, `fifo_path: &str`, and `config: &AudioConfig`. The `pactl` command arguments are derived from the config fields. `AudioFormat` gains a `as_pa_format(&self) -> &str` method returning `"s16le"` or `"f32le"`.
- Remove `parse_server_address` and its associated tests.

---

## Issue 5: Silent test assertions (LOW)

**Location:** `src/lib.rs:441`, `src/lib.rs:585`

**Problem:** Two tests call `matches!(...)` but do not assert on the result:

```rust
// test_audio_config_default
matches!(config.format, AudioFormat::S16LE);  // return value discarded

// test_audio_config_custom  
matches!(config.format, AudioFormat::F32LE);  // return value discarded
```

Both tests pass regardless of the actual format value, giving false confidence.

**Fix:** Change both lines to `assert!(matches!(...))`.

---

## Issue 6: Undocumented single-client FIFO limitation (LOW)

**Location:** `src/receiver.rs:116-132`

**Problem:** The receiver loop accepts multiple simultaneous connections and spawns a thread per client, all writing to the same FIFO. Concurrent writes produce corrupted audio. There is no documentation of this constraint and no enforcement.

**Fix:** Add a `# Limitations` section to the `run_receiver` doc comment:

> Only one active transmitter connection is supported at a time. The receiver accepts multiple connections, but all write to the same FIFO pipe. Concurrent connections will produce corrupted audio. A future version may enforce single-client access explicitly.

No code change is made to the accept loop (that would be Option C scope).

---

## Files Changed

| File | Changes |
|------|---------|
| `src/transmitter.rs` | Add `ToS16` trait; rewrite `convert_to_s16le`; remove `packet_count` thread; remove `verbose` param from `build_input_stream` |
| `src/receiver.rs` | Remove `verbose` param from `handle_audio_stream`; add limitation doc comment |
| `src/lib.rs` | Wire `AudioConfig` into `setup_virtual_microphone_with_config`; add `AudioFormat::as_pa_format`; remove `parse_server_address` and its tests; fix two silent `matches!` assertions |

---

## Out of Scope

- Single-client enforcement at runtime (Option C, deferred)
- New features (device selection, reconnect backoff tuning, etc.)
- CI/CD configuration

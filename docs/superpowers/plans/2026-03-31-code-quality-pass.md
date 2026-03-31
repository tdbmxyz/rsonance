# Code Quality Pass Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix six code quality and correctness issues identified in the spec: unsafe `TypeId` dispatch, a leaked debug thread, the `verbose` anti-pattern in helpers, dead public API, silent test assertions, and an undocumented single-client limitation.

**Architecture:** All changes are confined to the existing four source files (`lib.rs`, `transmitter.rs`, `receiver.rs`, `main.rs` untouched). Each task is independent and can be committed separately. No new files are created.

**Tech Stack:** Rust 2024 edition, `cpal` 0.16, `anyhow`, `log`/`env_logger`, `tokio`, `clap`.

---

## File Map

- Modify: `src/transmitter.rs` — trait-based sample conversion, remove leaked thread, remove verbose param from helper
- Modify: `src/lib.rs` — wire `AudioConfig`, remove `parse_server_address`, fix silent assertions
- Modify: `src/receiver.rs` — remove verbose param from helper, add limitation doc comment

---

### Task 1: Replace unsafe `TypeId` dispatch with a `ToS16` trait

**Files:**
- Modify: `src/transmitter.rs`

- [ ] **Step 1: Read the current `convert_to_s16le` and `build_input_stream` in `src/transmitter.rs`**

  Confirm the current signatures:
  ```
  fn convert_to_s16le<T>(data: &[T]) -> Vec<u8>
  where T: cpal::Sample + cpal::SizedSample + 'static
  
  fn build_input_stream<T>(...) -> anyhow::Result<cpal::Stream>
  where T: cpal::Sample + cpal::SizedSample + Send + 'static
  ```

- [ ] **Step 2: Write failing tests to verify trait-based conversion compiles and produces correct output**

  The existing tests in `src/transmitter.rs` already cover the correct output values. Run them first to confirm they currently pass with the unsafe version:

  ```bash
  cargo test test_convert -- --nocapture
  ```

  Expected: all `test_convert_*` tests PASS (they should be passing now — we need them to continue passing after the refactor).

- [ ] **Step 3: Add the `ToS16` trait and its three implementations at the top of `src/transmitter.rs`, after the `use` block**

  Insert after line 9 (after `use tokio::sync::mpsc;`):

  ```rust
  /// Sealed trait for converting audio samples to signed 16-bit little-endian.
  ///
  /// Implemented for `f32`, `i16`, and `u16` — the three sample formats
  /// supported by cpal that this tool handles. This replaces the previous
  /// unsafe `TypeId`-based dispatch.
  trait ToS16: cpal::Sample + cpal::SizedSample + Send + 'static {
      fn to_s16(self) -> i16;
  }

  impl ToS16 for f32 {
      fn to_s16(self) -> i16 {
          (self.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
      }
  }

  impl ToS16 for i16 {
      fn to_s16(self) -> i16 {
          self
      }
  }

  impl ToS16 for u16 {
      fn to_s16(self) -> i16 {
          (self as i32 - 32768) as i16
      }
  }
  ```

- [ ] **Step 4: Rewrite `convert_to_s16le` to use the `ToS16` trait**

  Replace the entire `convert_to_s16le` function body:

  ```rust
  fn convert_to_s16le<T: ToS16>(data: &[T]) -> Vec<u8> {
      let mut result = Vec::with_capacity(data.len() * 2);
      for sample in data.iter().copied() {
          result.extend_from_slice(&sample.to_s16().to_le_bytes());
      }
      result
  }
  ```

  The function signature changes from:
  ```rust
  fn convert_to_s16le<T>(data: &[T]) -> Vec<u8>
  where T: cpal::Sample + cpal::SizedSample + 'static
  ```
  to:
  ```rust
  fn convert_to_s16le<T: ToS16>(data: &[T]) -> Vec<u8>
  ```

- [ ] **Step 5: Update `build_input_stream` to add the `ToS16` bound**

  Change the where clause from:
  ```rust
  where
      T: cpal::Sample + cpal::SizedSample + Send + 'static,
  ```
  to:
  ```rust
  where
      T: ToS16,
  ```

  `ToS16` already implies `cpal::Sample + cpal::SizedSample + Send + 'static` by its definition, so no other changes are needed.

- [ ] **Step 6: Run the conversion tests to verify correct output is preserved**

  ```bash
  cargo test test_convert -- --nocapture
  ```

  Expected: all five `test_convert_*` tests PASS with identical output as before.

- [ ] **Step 7: Run clippy to verify no warnings**

  ```bash
  cargo clippy --all-targets -- -D warnings
  ```

  Expected: no warnings or errors.

- [ ] **Step 8: Commit**

  ```bash
  git add src/transmitter.rs
  git commit -m "refactor(transmitter): replace unsafe TypeId dispatch with ToS16 trait"
  ```

---

### Task 2: Remove the leaked debug thread from `build_input_stream`

**Files:**
- Modify: `src/transmitter.rs`

- [ ] **Step 1: Locate the `packet_count` code in `build_input_stream`**

  The block to remove is lines 170–184 in the current file (after Task 1's edits, line numbers may shift slightly):

  ```rust
  use std::sync::Arc;
  use std::sync::atomic::{AtomicUsize, Ordering};

  let packet_count = Arc::new(AtomicUsize::new(0));
  let packet_count_clone = packet_count.clone();

  // Print debug info every 5 seconds
  std::thread::spawn(move || {
      loop {
          std::thread::sleep(std::time::Duration::from_secs(5));
          let count = packet_count_clone.load(Ordering::Relaxed);
          debug!("Audio packets captured: {count} (in last 5 seconds)");
          packet_count_clone.store(0, Ordering::Relaxed);
      }
  });
  ```

  And inside the audio callback:
  ```rust
  packet_count.fetch_add(1, Ordering::Relaxed);
  ```

- [ ] **Step 2: Remove the `packet_count` Arc and the spawned thread; add a single `debug!` call in the callback**

  Replace the entire `build_input_stream` function body with:

  ```rust
  fn build_input_stream<T>(
      device: &cpal::Device,
      config: &cpal::StreamConfig,
      tx: mpsc::UnboundedSender<Vec<u8>>,
      err_fn: impl Fn(cpal::StreamError) + Send + 'static,
  ) -> anyhow::Result<cpal::Stream>
  where
      T: ToS16,
  {
      let stream = device.build_input_stream(
          config,
          move |data: &[T], _| {
              let converted_data = convert_to_s16le(data);
              debug!("Audio packet captured: {} bytes", converted_data.len());
              if let Err(e) = tx.send(converted_data) {
                  error!("Failed to send audio data to channel: {e}");
              }
          },
          err_fn,
          None,
      )?;

      Ok(stream)
  }
  ```

  Also remove the now-unused `use std::sync::Arc;` and `use std::sync::atomic::{AtomicUsize, Ordering};` imports if they are inside the function body. If they were added at the top of the file, verify whether any other code still uses them; if not, remove those `use` lines from the top-level imports.

- [ ] **Step 3: Run all tests**

  ```bash
  cargo test
  ```

  Expected: all tests pass.

- [ ] **Step 4: Run clippy**

  ```bash
  cargo clippy --all-targets -- -D warnings
  ```

  Expected: no warnings. In particular, no "unused import" warnings.

- [ ] **Step 5: Commit**

  ```bash
  git add src/transmitter.rs
  git commit -m "fix(transmitter): remove leaked packet-count debug thread"
  ```

---

### Task 3: Remove `verbose` parameter from `build_input_stream` and `handle_audio_stream`

**Files:**
- Modify: `src/transmitter.rs`
- Modify: `src/receiver.rs`

This task is already partially done for `build_input_stream` by Task 2 (which wrote the new body without `verbose`). This task handles the remaining `verbose` removal in `build_input_stream`'s call site and in `receiver.rs`.

- [ ] **Step 1: Verify `build_input_stream` no longer has a `verbose` parameter**

  After Task 2, the signature should be:
  ```rust
  fn build_input_stream<T>(
      device: &cpal::Device,
      config: &cpal::StreamConfig,
      tx: mpsc::UnboundedSender<Vec<u8>>,
      err_fn: impl Fn(cpal::StreamError) + Send + 'static,
  ) -> anyhow::Result<cpal::Stream>
  ```

  If `verbose` is still present, remove it now and also remove any `if verbose { ... }` guards inside the function body, replacing guarded `debug!`/`info!` calls with unconditional ones at the appropriate level.

- [ ] **Step 2: Verify the call sites for `build_input_stream` in `run_transmitter` do not pass `verbose`**

  In `src/transmitter.rs`, the three calls look like:
  ```rust
  cpal::SampleFormat::F32 => build_input_stream::<f32>(&device, &config, tx, err_fn)?,
  cpal::SampleFormat::I16 => build_input_stream::<i16>(&device, &config, tx, err_fn)?,
  cpal::SampleFormat::U16 => build_input_stream::<u16>(&device, &config, tx, err_fn)?,
  ```

  If any of these pass `verbose`, remove it.

- [ ] **Step 3: Remove `verbose` from `handle_audio_stream` in `src/receiver.rs`**

  Change the function signature from:
  ```rust
  fn handle_audio_stream(
      mut tcp_stream: TcpStream,
      fifo_path: String,
      buffer_size: usize,
      verbose: bool,
  ) -> anyhow::Result<()>
  ```
  to:
  ```rust
  fn handle_audio_stream(
      mut tcp_stream: TcpStream,
      fifo_path: String,
      buffer_size: usize,
  ) -> anyhow::Result<()>
  ```

- [ ] **Step 4: Remove the `if verbose { ... }` guards inside `handle_audio_stream`**

  Replace:
  ```rust
  if verbose {
      debug!("Starting audio stream handler");
      debug!("FIFO path: {fifo_path}");
      debug!("Using buffer size: {buffer_size} bytes");
  }
  ```
  with:
  ```rust
  debug!("Starting audio stream handler");
  debug!("FIFO path: {fifo_path}");
  debug!("Using buffer size: {buffer_size} bytes");
  ```

  Replace:
  ```rust
  Ok(0) => {
      if verbose {
          info!("Client disconnected");
      }
      break;
  }
  ```
  with:
  ```rust
  Ok(0) => {
      info!("Client disconnected");
      break;
  }
  ```

  Replace:
  ```rust
  Ok(n) => {
      if verbose {
          debug!("Received {n} bytes of audio data, writing to FIFO");
      }
      if let Err(e) = fifo.write_all(&buffer[..n]) {
  ```
  with:
  ```rust
  Ok(n) => {
      debug!("Received {n} bytes of audio data, writing to FIFO");
      if let Err(e) = fifo.write_all(&buffer[..n]) {
  ```

- [ ] **Step 5: Update the call site in `run_receiver` that passes `verbose` to `handle_audio_stream`**

  In `src/receiver.rs`, the thread spawn looks like:
  ```rust
  thread::spawn(move || {
      if let Err(e) = handle_audio_stream(stream, fifo_path, buffer_size, verbose) {
          error!("Error handling audio stream: {e}");
      }
  });
  ```

  Change it to:
  ```rust
  thread::spawn(move || {
      if let Err(e) = handle_audio_stream(stream, fifo_path, buffer_size) {
          error!("Error handling audio stream: {e}");
      }
  });
  ```

  Note: `verbose` no longer needs to be captured by this closure. If the closure previously captured `verbose` via a `let verbose = verbose;` binding or similar, remove that binding. If `verbose` is no longer used anywhere after the startup config block in `run_receiver`, Rust will warn you — remove any unused variable references.

- [ ] **Step 6: Run all tests**

  ```bash
  cargo test
  ```

  Expected: all tests pass.

- [ ] **Step 7: Run clippy**

  ```bash
  cargo clippy --all-targets -- -D warnings
  ```

  Expected: no warnings.

- [ ] **Step 8: Commit**

  ```bash
  git add src/transmitter.rs src/receiver.rs
  git commit -m "refactor: remove verbose param from internal helper functions"
  ```

---

### Task 4: Wire `AudioConfig` into `setup_virtual_microphone_with_config` and remove `parse_server_address`

**Files:**
- Modify: `src/lib.rs`
- Modify: `src/receiver.rs` (update call site)

- [ ] **Step 1: Add `as_pa_format` to `AudioFormat` in `src/lib.rs`**

  After the `AudioFormat` enum definition (currently around line 71), add an `impl` block:

  ```rust
  impl AudioFormat {
      /// Returns the PulseAudio format string for use with `pactl`.
      pub fn as_pa_format(&self) -> &str {
          match self {
              AudioFormat::S16LE => "s16le",
              AudioFormat::F32LE => "f32le",
          }
      }
  }
  ```

- [ ] **Step 2: Write a failing test for `as_pa_format`**

  Add to the `#[cfg(test)]` block in `src/lib.rs`:

  ```rust
  #[test]
  fn test_audio_format_as_pa_format() {
      assert_eq!(AudioFormat::S16LE.as_pa_format(), "s16le");
      assert_eq!(AudioFormat::F32LE.as_pa_format(), "f32le");
  }
  ```

- [ ] **Step 3: Run the test to verify it passes**

  ```bash
  cargo test test_audio_format_as_pa_format
  ```

  Expected: PASS.

- [ ] **Step 4: Update `setup_virtual_microphone_with_config` signature to accept `&AudioConfig`**

  Change the function signature from:
  ```rust
  pub fn setup_virtual_microphone_with_config(
      source_name: &str,
      fifo_path: &str,
  ) -> Result<VirtualMicResult>
  ```
  to:
  ```rust
  pub fn setup_virtual_microphone_with_config(
      source_name: &str,
      fifo_path: &str,
      config: &AudioConfig,
  ) -> Result<VirtualMicResult>
  ```

- [ ] **Step 5: Update the `pactl` command arguments to use `config` fields**

  Replace the hardcoded format/rate/channels arguments:
  ```rust
  "format=s16le",
  "rate=44100",
  "channels=2",
  ```
  with:
  ```rust
  &format!("format={}", config.format.as_pa_format()),
  &format!("rate={}", config.sample_rate),
  &format!("channels={}", config.channels),
  ```

- [ ] **Step 6: Update `setup_virtual_microphone` (the no-arg wrapper) to pass `AudioConfig::default()`**

  Change the call inside `setup_virtual_microphone`:
  ```rust
  setup_virtual_microphone_with_config("rsonance_virtual_microphone", "/tmp/rsonance_audio_pipe")
  ```
  to:
  ```rust
  setup_virtual_microphone_with_config(
      "rsonance_virtual_microphone",
      "/tmp/rsonance_audio_pipe",
      &AudioConfig::default(),
  )
  ```

- [ ] **Step 7: Update the call site in `src/receiver.rs`**

  In `run_receiver`, the call is:
  ```rust
  let result = setup_virtual_microphone_with_config(&microphone_name, &fifo_path)?;
  ```

  Change it to:
  ```rust
  let result = setup_virtual_microphone_with_config(
      &microphone_name,
      &fifo_path,
      &rsonance::AudioConfig::default(),
  )?;
  ```

  Since `receiver.rs` is inside the `rsonance` crate, the import path is just `crate::AudioConfig::default()`. Update the `use` block at the top of `receiver.rs`:

  Change:
  ```rust
  use crate::{
      VirtualMicResult, cleanup_virtual_microphone, setup_virtual_microphone_with_config,
      validate_buffer_size,
  };
  ```
  to:
  ```rust
  use crate::{
      AudioConfig, VirtualMicResult, cleanup_virtual_microphone,
      setup_virtual_microphone_with_config, validate_buffer_size,
  };
  ```

  And the call becomes:
  ```rust
  let result = setup_virtual_microphone_with_config(
      &microphone_name,
      &fifo_path,
      &AudioConfig::default(),
  )?;
  ```

- [ ] **Step 8: Remove `parse_server_address` and its tests from `src/lib.rs`**

  Delete:
  - The entire `parse_server_address` function (the doc comment + `pub fn parse_server_address(...) { ... }` block, currently around lines 343–383).
  - The following test functions from the `#[cfg(test)]` block:
    - `test_parse_server_address_with_port`
    - `test_parse_server_address_without_port`
    - `test_parse_server_address_empty`
    - `test_parse_server_address_none`
    - `test_parse_server_address_whitespace`
    - `test_parse_server_address_with_protocol`
    - `test_parse_server_address_edge_cases`

  Also remove `parse_server_address` from the doc example at the top of `lib.rs` if it appears there (it does not in the current version, but double-check).

- [ ] **Step 9: Update the existing `test_setup_virtual_microphone_with_custom_config` test to pass the new `config` argument**

  Current test:
  ```rust
  #[test]
  fn test_setup_virtual_microphone_with_custom_config() {
      let result =
          setup_virtual_microphone_with_config("test_virtual_mic", "/tmp/test_fifo_pipe");
      match result {
          Ok(_) | Err(_) => {}
      }
  }
  ```

  Update to:
  ```rust
  #[test]
  fn test_setup_virtual_microphone_with_custom_config() {
      let result = setup_virtual_microphone_with_config(
          "test_virtual_mic",
          "/tmp/test_fifo_pipe",
          &AudioConfig::default(),
      );
      match result {
          Ok(_) | Err(_) => {}
      }
  }
  ```

- [ ] **Step 10: Run all tests**

  ```bash
  cargo test
  ```

  Expected: all tests pass. The `parse_server_address` tests are gone; the `setup_virtual_microphone` tests and new `test_audio_format_as_pa_format` test pass.

- [ ] **Step 11: Run clippy**

  ```bash
  cargo clippy --all-targets -- -D warnings
  ```

  Expected: no warnings. In particular, no "unused" warnings on `AudioConfig`, `AudioFormat`, or their methods.

- [ ] **Step 12: Commit**

  ```bash
  git add src/lib.rs src/receiver.rs
  git commit -m "refactor(lib): wire AudioConfig into setup_virtual_microphone_with_config, remove unused parse_server_address"
  ```

---

### Task 5: Fix silent `matches!` assertions

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Fix `test_audio_config_default`**

  Locate in the test block:
  ```rust
  matches!(config.format, AudioFormat::S16LE);
  ```

  Change to:
  ```rust
  assert!(matches!(config.format, AudioFormat::S16LE));
  ```

- [ ] **Step 2: Fix `test_audio_config_custom`**

  Locate:
  ```rust
  matches!(config.format, AudioFormat::F32LE);
  ```

  Change to:
  ```rust
  assert!(matches!(config.format, AudioFormat::F32LE));
  ```

- [ ] **Step 3: Run the two affected tests to confirm they pass**

  ```bash
  cargo test test_audio_config_default test_audio_config_custom
  ```

  Expected: both PASS. (They should pass since the `Default` impl does return `S16LE` and the custom config uses `F32LE`.)

- [ ] **Step 4: Run clippy**

  ```bash
  cargo clippy --all-targets -- -D warnings
  ```

  Expected: no warnings. Clippy may have been warning about `unused_must_use` on the old `matches!` calls — verify those warnings are now gone.

- [ ] **Step 5: Commit**

  ```bash
  git add src/lib.rs
  git commit -m "fix(tests): assert on matches! results instead of discarding them"
  ```

---

### Task 6: Document the single-client FIFO limitation in `run_receiver`

**Files:**
- Modify: `src/receiver.rs`

- [ ] **Step 1: Add a `# Limitations` section to the `run_receiver` doc comment**

  The current doc comment for `run_receiver` ends before `pub fn run_receiver`. Add the following section before the closing of the doc comment (before the `# Example` block or at the end of the existing comment):

  ```rust
  /// # Limitations
  ///
  /// Only one active transmitter connection is supported at a time. The receiver
  /// accepts multiple simultaneous TCP connections and spawns a thread per client,
  /// but all clients write to the same FIFO pipe. Concurrent connections will
  /// produce corrupted audio. A future version may enforce single-client access
  /// explicitly (e.g., reject or queue additional connections).
  ```

  Place this after the existing `# Returns` section and before `# Example`.

- [ ] **Step 2: Run all tests to confirm nothing broke**

  ```bash
  cargo test
  ```

  Expected: all tests pass.

- [ ] **Step 3: Run `cargo fmt --check`**

  ```bash
  cargo fmt --check
  ```

  Expected: no formatting changes needed. If there are, run `cargo fmt` and check again.

- [ ] **Step 4: Run clippy**

  ```bash
  cargo clippy --all-targets -- -D warnings
  ```

  Expected: no warnings.

- [ ] **Step 5: Commit**

  ```bash
  git add src/receiver.rs
  git commit -m "docs(receiver): document single-client FIFO limitation in run_receiver"
  ```

---

## Final Verification

After all tasks are complete:

- [ ] **Run full test suite**

  ```bash
  cargo test
  ```

  Expected: all tests pass (28 unit + doc-tests, minus the removed `parse_server_address` tests, plus the new `test_audio_format_as_pa_format` test).

- [ ] **Run clippy clean**

  ```bash
  cargo clippy --all-targets -- -D warnings
  ```

  Expected: zero warnings.

- [ ] **Run fmt check**

  ```bash
  cargo fmt --check
  ```

  Expected: no changes needed.

---
name: rust-zero-drift
description: >-
  Zero-Drift Systems Rust Architecture, Compiler Guardrail & 20-Domain Pure-Rust
  Force-Multiplier Engine. Trigger whenever architecting, writing, refactoring,
  optimizing, or auditing greenfield or production Rust projects (low-latency
  engines, real-time Discord/gateway bots with twilight, async systems, native
  GUIs/GPU UIs/HUDs with Iced/Slint/GPUI/egui, local AI/ML inference,
  robotics/ECS simulations, eBPF, cryptography/EVM/zkVMs, OS automation, or
  single-binary packaging). Enforces #![forbid(unsafe_code)], Typestate/Newtype
  domain modeling, strict Clippy anti-hallucination lints, Sans-IO workspace
  layering, Rust 2024 idioms, closed-loop property/mutation verification, and
  seamless handoff to REA-Argonaut subagents for hot-kernel differential fuzzing.
---

# Rust Zero-Drift Architecture & Force-Multiplier Skill (`rust-zero-drift`)

Use this skill whenever building, expanding, refactoring, or auditing greenfield or production Rust codebases. While [REA-Argonaut](https://github.com/PNWcoastie/REA-Argonaut) handles binary reverse-engineering and legacy C/C++-to-Rust reconstruction, `rust-zero-drift` governs all native Rust engineering—turning the Rust type system and compiler into a mathematical guardrail where invalid states, silent overflows, unhandled edge cases, and lazy AI shortcuts cannot compile.

For the full memory-layout refresher, 20-domain crate ecosystem routing tables, and copy-paste templates, see [README.md](README.md) and the compilable `#![forbid(unsafe_code)]` reference implementation in [src/lib.rs](src/lib.rs).

---

## The 4-Stage Zero-Drift Rust Engineering Workflow

### Stage 1 — Compartmentalized Domain & Typestate Modeling (Before Implementation)
1. **Make Invalid States Unrepresentable:**
   - Ban primitive obsession (passing raw `f64`, `u64`, `usize`, or `String` across module boundaries).
   - Define `#[repr(transparent)]` Newtypes (`struct OrderId(pub u64);`) or `nutype` validated wrappers (`#[nutype(validate(finite, greater = 0.0))] pub struct Price(f64);`) so invalid values (`NaN`, negative quantities, mismatched IDs) cannot be constructed.
   - Encode protocol and workflow order into compile-time **Typestate state machines** (`Command<Unchecked> -> Command<Verified> -> Command<Committed>`) using `PhantomData<State>` and move-by-value transitions (`fn verify(self) -> Result<Command<Verified>, Error>`).
2. **Choose the Right Memory & Graph Topology Up Front:**
   - **Dataflow DAGs / Pipelines:** Use strict ownership transfer or bounded channels (`rtrb` wait-free SPSC ring buffers for thread-to-thread hot paths; bounded `tokio::sync::mpsc::channel(cap)` for async I/O).
   - **Cyclic Graphs / Simulations / UI Trees:** Never use `Rc<RefCell<Node>>` pointer webs. Use generational arenas (`slotmap::SlotMap<NodeKey, Node>`) or standalone Entity-Component-Systems (`bevy_ecs` / `hecs`) with Structure-of-Arrays (SoA) layout.
   - **Multi-Crate Sans-IO Workspaces:** Once a project exceeds ~2,000 LOC, split into `crates/domain-types` (zero I/O deps), `crates/core-engine` (pure deterministic `fn on_event(&mut self, ev: Event) -> Actions` with zero network/async calls), and `crates/io-adapters` (Tokio, WebSockets, DB, hardware buses).

### Stage 2 — Compiler Guardrail Lock-In & Rust 2024 Implementation
1. **Enforce `#![forbid(unsafe_code)]` & Strict Anti-Drift Lints from Line 1:**
   - Every root `Cargo.toml` must configure `[lints.rust] unsafe_code = "forbid"` and `[lints.clippy]` denying `.unwrap()`, `.expect()`, `todo!()`, `unimplemented!()`, `dbg!()`, exact float comparisons (`float_cmp`), and silent lossy integer casts (`cast_possible_truncation`, `cast_sign_loss`).
   - Add a `clippy.toml` at the workspace root when banning project-specific anti-patterns (`std::env::var` in hot paths, `std::sync::Mutex` in favor of `parking_lot::Mutex`, etc.).
2. **Enforce Modern Rust 2024 Idioms (Zero Legacy Training-Weight Drift):**
   - Use standard library `std::sync::LazyLock` and `std::sync::OnceLock` (never `lazy_static!` or `once_cell`).
   - Use native `async fn` in traits (only reach for `#[async_trait]` or `trait-variant` if dynamic `Box<dyn Trait>` object safety is explicitly required).
   - Use flat `let Some(val) = expr else { return Err(...); };` guards instead of deeply nested `match` pyramids.
   - Prefer generics, traits, `const fn`, and `#[derive]` over custom `macro_rules!` DSLs so `rust-analyzer` type inference remains instant.
3. **Production Async, Error, Secret & Concurrency Discipline:**
   - **Hardware-Adaptive Concurrency:** Scale worker pools dynamically at runtime via `std::thread::available_parallelism()` across `linux`, `darwin`, and `win32`—never hardcode core counts.
   - **Async Cancellation & Backpressure:** Ban `unbounded_channel()`. Ensure all `tokio::select!` branches are cancellation-safe, and wire `tokio_util::sync::CancellationToken` + `TaskTracker` for deterministic graceful shutdown.
   - **Two-Tier Structured Errors:** Use `thiserror` exhaustive `enum` errors inside domain/engine libraries and `miette` or `error-stack` at binary/CLI boundaries.
   - **Secret Hygiene:** Wrap all API keys, private keys, and tokens in `secrecy::SecretString` + `zeroize` (`ZeroizeOnDrop`) so credentials never leak via `Debug` formatting or linger in RAM.

### Stage 3 — 20-Domain Pure-Rust Ecosystem Routing (Zero Unnecessary Polyglot Bloat)
Before reaching for Python, JavaScript, or C/C++ glue, consult the 20-domain routing matrices in [README.md](README.md):
- **Native UI, GPU Engines, TUI & Custom HUD Bricks:** `iced` (Elm state-machine GUI), `slint` (ahead-of-time compiled declarative desktop/embedded UI), `gpui` (Zed's 120–240Hz GPU shader UI), `egui` (immediate-mode 144Hz GUI), `makepad` (live shader-driven UI), `floem` (fine-grained reactive native GUI), `xilem` + `vello` (GPU compute-shader vector UI), `freya` / `vizia` (Skia & audio-plugin UIs), `rerun` (3D/time-series telemetry), `ratatui` (120fps TUI), `dioxus` / `leptos` (Wasm/fullstack reactive), `tauri` (lightweight desktop core), `winit` + `tiny-skia` + `cosmic-text` + `sctk` (custom Wayland layer-shell HUD overlays).
- **Real-Time Chat Gateways, Bot Orchestration & Event Automation:** `twilight` (`twilight-gateway`, `twilight-http`, `twilight-standby`, `twilight-cache-inmemory`), `poise`, `teloxide` (compile-time dialogue state machines), `matrix-sdk`, `notify` / `watchexec`.
- **High-Frequency WebSockets, QUIC & Financial Wire Protocols:** `fastwebsockets` / `tokio-tungstenite`, `quinn` / `s2n-quic`, `tower` (mathematical backpressure middleware), `fefix` / `hotfix` (FIX `4.2`/`4.4`/`5.0` & SBE).
- **Stack-Inline Strings, SIMD JSON & Hot-Path Collections:** `compact_str` / `smol_str` (24B stack-inline strings), `smallvec` / `arrayvec`, `lasso` (32-bit string interning), `sonic-rs` / `simd-json` (SIMD JSON & lazy path extraction), `ahash` / `rustc-hash` (`FxHashMap`), `roaring` bitmaps, `lz4_flex`, `aho-corasick`, `ulid` / `uuid` v7.
- **Zero-Copy & Memory-Mapped IO:** `rkyv`, `zerocopy`, `bytemuck`, `memmap2`, `bytes`, `postcard`.
- **Concurrency, Lock-Free & Actors:** `tokio`, `monoio` / `glommio` (`io_uring`), `rayon`, `rtrb` / `ringbuf`, `crossbeam`, `dashmap`, `parking_lot`, `kameo` / `ractor` (Erlang-style supervision trees), `apalis` / `tokio-cron-scheduler`.
- **Native AI, GPU & Math:** `candle` (pure-Rust LLM/tensor inference), `burn`, `ort` (ONNX Runtime), `cubecl` / `wgpu` (GPU compute shaders), `faer` / `nalgebra` (linear algebra), `rust_decimal` / `fixed` (exact fixed-point math), `pulp` / `wide` (safe portable SIMD).
- **Data, Storage, Media & Typesetting:** `polars` / `datafusion` (Arrow columnar), `redb` / `fjall` (pure-Rust ACID KV stores), `sqlx` (compile-time checked SQL), `tantivy` (full-text search), `lancedb` (vector search), `fast_image_resize` / `image`, `typst` (millisecond PDF typesetting).
- **Specialized Systems & Hardware Verticals:**
  - **Native OS Automation & Telemetry:** `enigo` / `rdev` / `evdev`, `xcap` / `scap` (high-FPS screen capture), `arboard` (clipboard), `global-hotkey` / `tray-icon`, `sysinfo` / `nvml-wrapper` (GPU telemetry).
  - **eBPF & Kernel Networking:** `aya` (pure-Rust kernel XDP/eBPF + user space), `smoltcp`.
  - **Crypto, EVM & zkVMs:** `alloy`, `revm`, `sp1` / `risc0` (Rust zero-knowledge VMs), `plonky3`, `rustls`, `zeroize`.
  - **ECS & Deterministic Physics:** `bevy_ecs` / `hecs`, `rapier3d` / `rapier2d`, `parry`, `glam`.
  - **Stealth Networking & Streaming Scraping:** `rquest` / `impit` (TLS/JA3/JA4/HTTP2 emulation), `lol_html` (`O(1)` streaming HTML parser), `chromiumoxide`.
  - **Real-Time Audio, SDR & DSP:** `cpal` + `rtrb`, `fundsp` / `dasp`, `rustfft`, `futuresdr`.
  - **Robotics, Embedded & Wasm Sandboxing:** `dora-rs` / `copper-rs`, `zenoh`, `embassy` (`no_std` async firmware), `wasmtime` / `extism`, `pyo3` + `maturin`.
  - **Cross-Compilation & Single-Binary Packaging:** `cargo-zigbuild` (`musl` / macOS / Windows cross-linking), `rust-embed`.

### Stage 4 — Closed-Loop Verification & `REA-Argonaut` Subagent Bridge
1. **Property, Allocation & Mutation Verification:**
   - Verify core state transitions and parsers with `proptest` (1,000+ generated inputs) rather than only hardcoded example tests.
   - Use `dhat` (`dhat-rs`) to assert zero heap allocations inside hot loops, `turmoil` for deterministic network fault simulation, and `cargo-mutants` to ensure unit/property tests catch every mutated branch.
2. **Bridge to [REA-Argonaut](https://github.com/PNWcoastie/REA-Argonaut) Subagents When Applicable:**
   - **`safe-rust-reconstructor`:** Delegate self-contained, zero-unsafe math kernels, parsers, or bit-manipulation algorithms to `safe-rust-reconstructor`.
   - **`adversarial-parity-auditor` (`verify_rust_parity` in `c_source_code` mode):** When optimizing a subtle SIMD, fixed-point, DSP, or cryptographic function, write a simple reference implementation in `c_source_code` and invoke `adversarial-parity-auditor` to run 10,064 SplitMix64 + boundary seeds with binary-search counterexample shrinking and nanosecond speedup benchmarking.
   - **`subgraph-architect`:** Run on compiled `target/release/<binary>` artifacts via `inspect_rust_cpp_binary` and `decompile_subgraph` to audit surviving `core::panicking::Location` branches and verify third-party `-sys` FFI dependencies contain zero `latent_hazards`.
   - **`forensic-vertical-specialist`:** Run `inspect_ai_model_artifact`, `inspect_packet_capture`, or `inspect_sqlite_database` to extract exact `.gguf`/`.safetensors`/`.onnx` tensor shapes, PCAP flows, or SQLite schemas in `< 50 ms` before authoring Rust structs.
3. **Final Zero-Warning Gate:**
   - Always verify with `cargo clippy --all-targets -- -D warnings` and `cargo test` before completion.

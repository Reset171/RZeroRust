# RZero: a map for agents

Native timeline-rewind mod for **Horizon** (Minecraft: Java Edition **26.3**).

## Rules

- **Generated file:** Never edit `mixins/rzero/runtime.rs` directly. Edit `mixins/rzero/runtime/*.rs`; `build.rs` regenerates `mixins/rzero/runtime.rs` on `cargo check` / `cargo build` / `cargo test`.
- **Isolation:** Keep test runs in temporary `rzero_e2e_*` worlds via `RZeroHarness` (`tools/rzero_harness.py`). Shut down the game with `quit` (`tools.horizon.game_quit()`), not `SIGKILL`.
- **Regression tests:** Every bug fix requires a reproducing E2E test in `tools/tests/test_<category>.py` registered in `tools/rzero_e2e.py`.

## Repository layout

- `mixins/rzero/horizon-mixin.toml` & `hook_*.rs` — Mixin declarations and hook helpers injected into `horizon-client` (`horizon_revision = "3d9ad5214dccdbdace554f8bc98000890b46c619"`, `NATIVE_HOOKS_API: 409`).
- `mixins/rzero/hub_ext.rs` — `Hub::rzero_reset_tracked` extension injected into `crates/horizon-client/src/lan/hub.rs`.
- `mixins/rzero/runtime/` — Modular runtime implementation:
  - `types.rs` — `RZeroState`, `AnchorMode`, `RollbackPolicy` (35 flags).
  - `snapshots.rs` — `TimelineSnapshot`, `LevelSnapshot`, `GuestSnapshot`, `ClientBlockSnapshot`, `ROLLBACK_EPOCH`.
  - `capture.rs` — `create_slot_checkpoint_on_server`, level and parked dimension snapshot capture.
  - `restore.rs` — `rollback_slot_checkpoint_on_server` (server state restoration) and `apply_client_level_rollback` (client `nearest-first` section diffing).
  - `persistence.rs` — Disk storage under `saves/<World>/rzero/` (`meta.json`, `level.dat`, `players/`, `data/`, `dimensions/`) and lazy hydration (`hydrate_slot_from_disk_on_server`).
  - `adaptive.rs` — Player safety checks, escape flood-fill, threat calculation, and interval/adaptive auto-saving.
  - `commands.rs` — `/rzero` command execution and JSON status/metrics formatting.
  - `ui.rs` — Native mod registration, settings buttons, and `config/rzero.json` loading.
  - `tracker.rs` — `LoopTracker` F3+B hitbox color comparison across loops.
  - `benchmark.rs` — Phase timers and log output (`logs/rzero.log`, `saves/<World>/rzero/benchmark.log`).
  - `hooks.rs` — `handle_lifecycle`, `handle_tick`, `intercept_guest_die`, `handle_key`.
- `src/` — Standalone `rzero` library crate (`block`, `checkpoint`, `client`, `command`, `config`, `engine`, `event`, `metrics`, `mixin`, `network`, `runtime`, `serial`, `util`).
- `tools/` — `pr_ready.py` (pre-commit check), `rzero_harness.py` (control socket client), `rzero_e2e.py` & `tools/tests/` (42 E2E tests), `rzero_bench.py` (stress benchmark), `rzero_spark.py` (CPU profiler).

## Commands

```sh
python3 tools/pr_ready.py --quick          # verify runtime.rs sync, clippy -D warnings, cargo test, python syntax
python3 tools/pr_ready.py --e2e            # same + fast smoke E2E tests
python3 tools/rzero_e2e.py --fast          # run 5 smoke tests (1, 2, 9, 14, 19)
python3 tools/rzero_e2e.py --test <N>      # run single E2E test by ID or name
python3 tools/rzero_e2e.py --group <name>  # run group (chunks, explosions, vehicles, mobs, death, slots, policies, containers, dimensions, ai, world, portals, determinism)
python3 tools/rzero_e2e.py                 # run all 42 E2E tests
```
# RZero

**[English](README.md) · [Русский](README.ru.md)**

Timeline rewind mod (`Return by Death`) for [Horizon](https://github.com/horizon-rust/horizon).
Rewinds world chunks, entities, block entities, scheduled ticks, RNG state, dimensions, and player state to the last checkpoint on death or manual trigger.

## Build

`build.rs` assembles `mixins/rzero/runtime/*.rs` into `mixins/rzero/runtime.rs`.

```sh
cargo build --locked
```

Package `.hnmod` and build into a Horizon instance:

```sh
zip -r ~/.local/share/horizon/horizon-native-mods/rzero.hnmod horizon-native.json mixins/
horizon-launcher native-build --trust-native-code
```

Or build directly from the source directory:

```sh
horizon-launcher native-build . --trust-native-code
```

## Controls and commands

| Input / Command | Description |
|---|---|
| `F6` / `/rzero set [slot]` | Save checkpoint to `slot` (default: `default`) |
| `F7` / `/rzero load [slot]` | Roll back world and player state to `slot` |
| `F8` / `/rzero anchor mode <mode>` | Set key player mode (`fixed`, `multi`, `everyone`, `rotating`) |
| `/rzero anchor cooldown <sec>` | Set rollback cooldown in seconds (`0` disables cooldown) |
| `/rzero slot <list\|save\|load\|select\|delete> [name]` | Manage named checkpoint slots |
| `/rzero policy <get\|set\|reset> [field] [bool]` | Inspect or update rollback policy flags |
| `/rzero autosave <on\|off\|interval\|adaptive> [sec]` | Configure automatic checkpoints |
| `/rzero f3 [on\|off]` | Toggle F3+B mob hitbox color comparison against the previous loop |
| `/rzero status` / `/rzero json` / `/rzero metrics` | Status and threat metrics (text or JSON) |
| `/rzero bench` | Show timing breakdown of the last rollback |

## Checks

```sh
python3 tools/pr_ready.py --quick          # clippy, unit tests, runtime.rs sync, python syntax
python3 tools/rzero_e2e.py --fast          # smoke E2E tests
python3 tools/rzero_e2e.py                 # full E2E test suite
python3 tools/rzero_bench.py               # stress benchmark
python3 tools/rzero_spark.py --record 5    # CPU call-tree profiler
```

## License

[GPL-3.0-only](LICENSE).
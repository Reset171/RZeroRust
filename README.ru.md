# RZero

**[English](README.md) · [Русский](README.ru.md)**

Мод добавляет `Возвращение через смерть` для движка [Horizon](https://github.com/horizon-rust/horizon).
Восстанавливает чанки, сущностей, блок-сущности, плановые тики, состояние ГСЧ, измерения и параметры игрока к последней точке сохранения при гибели или по ручной команде.

## Сборка

Скрипт `build.rs` собирает подмодули `mixins/rzero/runtime/*.rs` в `mixins/rzero/runtime.rs`.

```sh
cargo build --locked
```

Упаковка `.hnmod` и сборка в инстанс Horizon:

```sh
zip -r ~/.local/share/horizon/horizon-native-mods/rzero.hnmod horizon-native.json mixins/
horizon-launcher native-build --trust-native-code
```

Либо сборка напрямую из директории проекта:

```sh
horizon-launcher native-build . --trust-native-code
```

## Управление и команды

| Клавиша / Команда                                       | Описание                                                          |
| ------------------------------------------------------- | ----------------------------------------------------------------- |
| `F6` / `/rzero set [slot]`                              | Сохранить чекпоинт в слот (по умолчанию `default`)                |
| `F7` / `/rzero load [slot]`                             | Откатить мир и игрока к слоту                                     |
| `F8` / `/rzero anchor mode <mode>`                      | Режим ключевого игрока (`fixed`, `multi`, `everyone`, `rotating`) |
| `/rzero anchor cooldown <sec>`                          | Кулдаун между откатами в секундах (`0` — выключен)                |
| `/rzero slot <list\|save\|load\|select\|delete> [name]` | Управление именованными слотами                                   |
| `/rzero policy <get\|set\|reset> [field] [bool]`        | Просмотр и изменение флагов политики отката                       |
| `/rzero autosave <on\|off\|interval\|adaptive> [sec]`   | Настройка автосохранений                                          |
| `/rzero f3 [on\|off]`                                   | Подсветка хитбоксов мобов (F3+B) относительно прошлой петли       |
| `/rzero status` / `/rzero json` / `/rzero metrics`      | Статус и метрики угрозы (текст или JSON)                          |
| `/rzero bench`                                          | Пофазная сводка времени последнего отката                         |

## Проверки

```sh
python3 tools/pr_ready.py --quick          # clippy, модульные тесты, синхронизация runtime.rs, синтаксис python
python3 tools/rzero_e2e.py --fast          # быстрые smoke E2E-тесты
python3 tools/rzero_e2e.py                 # полный набор E2E-тестов
python3 tools/rzero_bench.py               # стресс-бенчмарк
python3 tools/rzero_spark.py --record 5    # профилировщик дерева вызовов CPU
```

## Лицензия

[GPL-3.0-only](LICENSE).
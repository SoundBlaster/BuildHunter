# BuildHunter

Hunt down build artifacts and development caches.

План macOS приложения: [PRD](docs/PRD.md) и [ADR](docs/adr/0001-macos-app-architecture.md).
В macOS приложении есть [анимированная диаграмма артефактов](docs/artifact-diagram.md)
в отдельном окне с обновлениями по мере сканирования.

Read-only CLI для поиска локальных артефактов сборки и кешей Swift, Rust и Python.
Rust, без внешних dependencies. Утилита ничего не удаляет.

```sh
cargo install --path .
build-hunter /Users/egor/Development/GitHub
build-hunter . --language python
build-hunter . --language rust --json > report.json
build-hunter . --include-envs
build-hunter . --apparent
build-hunter --list-filters
build-hunter . --exclude swift.build --exclude python.pytest-cache,python.ruff-cache
build-hunter . --exclude rust --json
```

Фильтры поиска описаны в [Search settings](docs/search-settings.md). Без `--exclude`
сохраняется прежний набор поиска, включая opt-in виртуальных окружений CLI.
`--exclude` принимает стабильные ID из `--list-filters`, список через запятую или
имя языка (`swift`, `rust`, `python`); параметр можно повторять. `--language`
по-прежнему выбирает язык, исключения дополнительно ограничивают результаты.

После стандартного `cargo install` executable находится в `~/.cargo/bin`.
Также можно сразу запускать `target/release/build-hunter` из папки проекта.

| Language | Обнаруживаемые пути |
|---|---|
| Swift | `.build` |
| Rust | `target` рядом с `Cargo.toml` |
| Python | `__pycache__`, `.pyc`, `.pyo`, `.egg-info`, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.pytype`, `.tox`, `.nox` |
| Python build | `build`, `dist` рядом с `pyproject.toml`, `setup.py` или `setup.cfg` |
| Python environments | `.venv`, `venv` с `pyvenv.cfg`, только с `--include-envs` |

По умолчанию корень — текущая папка. Таблица отсортирована по убыванию размера.
Вложенные совпадения отмечаются `nested` и не добавляются повторно в общий размер.
JSON содержит абсолютные пути, размеры в bytes и список ошибок.

На Unix размер вычисляется по allocated blocks, включая директории. `--apparent`
использует logical size; на других платформах он используется всегда.
Symlink не обходятся и не учитываются. `.git` и virtual environments пропускаются
вне найденных артефактов. Hard link учитываются по pathname, поэтому это оценка
размера дерева, а не гарантированного reclaimable space. APFS clones/compression
также не позволяют считать эту оценку обещанием освободить указанное место.

На macOS стандартные облачные области в `/Users/<account>/Library`
(также через `/System/Volumes/Data/Users/<account>/Library`)
(`Mobile Documents` для iCloud и `CloudStorage` для File Provider) всегда
пропускаются до обхода их содержимого, включая уже загруженные объекты.
CLI выводит предупреждение с путём (также в JSON `errors`), GUI показывает его
в `Warnings and details` и отмечает scan как incomplete. Размеры пропущенного
содержимого не входят в итог. Папки старых sync clients в произвольных местах
этим правилом не распознаются; имя `iCloud` само по себе не является признаком.

Это поиск по именам и признакам проекта: совпадения требуют проверки перед удалением.
Custom Cargo target-dir, Xcode DerivedData, глобальные pip/Cargo/Swift caches,
Python wheel files вне `dist` и произвольные имена артефактов пока не обнаруживаются.
Размеры могут меняться при параллельной сборке. Scan не является atomic snapshot.
Невалидные UTF-8 имена отображаются с replacement character.

Exit codes: `0` — scan завершён; `1` — неполный отчёт с предупреждениями (включая пропущенные облачные области);
`2` — ошибка аргументов или корневой папки.

```sh
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

Однопроходный scan исключает subprocess `du`, но ускорение не гарантируется:
большинство времени может уходить на filesystem I/O и metadata. Сравнивайте release
build на одном дереве; результаты зависят от filesystem cache и активных сборок.

## Scan performance profile

```sh
build-hunter ~/Development --profile /tmp/buildhunter-profile.json
build-hunter ~/Development --apparent --json --profile /tmp/buildhunter-profile-apparent.json
```

Profiling is opt-in. `--profile FILE` saves a separate versioned JSON report;
normal terminal/`--json` artifact output and search filters stay unchanged.
The destination must not already exist; parent folders must exist. The report
is created after scanning, so even a destination inside an artifact does not
change its measured size during that scan. Incomplete scans still save a report
and retain exit code 1. A hard process termination does not save a final report.

The report contains cumulative snapshots, elapsed microseconds, evaluated
filesystem entries/folders, discovered artifacts, warnings, pending worker tasks,
and measured bytes. Initial and terminal snapshots bracket approximately 100 ms
samples. Entries count processed files and directory listings, including the
selected root; pruned descendants and skipped cloud placeholders are excluded.
Directory processing/callback backpressure can delay samples. Idle worker waits
produce flat intervals. Elapsed time covers the Rust scan through worker join.

**Measured bytes/s is metadata size throughput, not disk read speed.** It uses
allocated sizes by default (`--apparent` uses logical sizes), counts nested
artifacts only once, and excludes unrelated files. It can legitimately reach
GB/s without reading file contents. Average rates divide cumulative counters by
elapsed scan time; peak rates are the largest adjacent-snapshot delta/time.

Schema version 1 includes `size_mode`, `status`, `sample_interval_ms`,
`average_entries_per_second`, `peak_entries_per_second`, and corresponding
`*_measured_bytes_per_second` fields. `samples` holds the latest 4,096 snapshots;
`truncated_samples` reports discarded earlier snapshots. Lifetime averages and
peaks survive truncation. Consumers can derive interval speeds from differences
between adjacent cumulative snapshots, avoiding division by zero.

The macOS main window has a collapsible **Scan profile** panel with entries/s
or measured bytes/s, current/average/peak rates, elapsed time, counters, and a
live chart of the latest 600 intervals (about one minute). The panel starts
collapsed; collecting samples does not depend on whether it is open. Stop keeps
the profile and shows current speed as zero. Rescan/replacing the folder resets
it and rejects stale events. Profiles are kept only for the current window.

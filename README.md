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

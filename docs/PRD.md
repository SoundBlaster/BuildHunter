# BuildHunter macOS PRD

Дата: 2026-10-02. Статус: требования MVP согласованы; SwiftUI skeleton и in-process Rust scanner integration реализованы. Signed sandbox runtime, Finder/clipboard actions и Store readiness остаются открытыми этапами.

BuildHunter помогает разработчику найти локальные build artifacts, кеши и environments, случайно или намеренно созданные внутри дерева проектов. Основной сценарий — обнаружить занимающие место папки после работы инструментов и агентов, оценить их размеры и перейти к ним в Finder. Приложение не определяет, что можно безопасно удалить, и не удаляет данные.

## Цель и платформа

- Native SwiftUI app для macOS 26 и новее, только Apple Silicon (`arm64`).
- Дистрибуция через Mac App Store; App Sandbox и user-selected read-only file access.
- Существующий Rust CLI сохраняется как отдельный продукт с общим scanner core.
- SpecificationCore через SPM используется для semantic policy и typed decisions согласно [ADR](adr/0001-macos-app-architecture.md).

## Согласованные требования

| ID | Требование |
|---|---|
| BH-001 | Многооконность: одна target-папка и независимый scan на окно. |
| BH-002 | Drag-and-drop папки в окно; `File → Open Folder…` и `⌘O` открывают системный выбор папки для активного окна. |
| BH-003 | Drop zone может также быть кликабельной областью открытия папки. Точная компоновка определяется дизайном. |
| BH-004 | Новый target заменяет прежний в этом окне и отменяет прежний scan. |
| BH-005 | Scan запускается автоматически после принятия target. |
| BH-006 | Stop отменяет scan, сохраняя частичные результаты. Rescan начинает новое сканирование target. |
| BH-007 | SwiftUI Table пополняется асинхронно во время обхода; UI остаётся отзывчивым. |
| BH-008 | Таблица содержит только внешние корни найденных артефактов. Вложенные совпадения не становятся отдельными строками. |
| BH-009 | Колонки: Path, Size, Language, Kind. Статус измерения строки отображается в Size. |
| BH-010 | Path отображается относительно target; Copy Path копирует абсолютный путь. |
| BH-011 | Доступны Reveal in Finder, с открытием содержащей папки и выделением находки, и Copy Path. |
| BH-012 | Нет удаления, очистки, перемещения, изменения target или его содержимого. |
| BH-013 | Ошибки доступа не прекращают обход доступной части дерева. Показывается предупреждение о неполном результате и раскрываемые подробности. |
| BH-014 | Окна, targets, результаты и разрешения доступа не восстанавливаются между запусками. |
| BH-015 | Нет пользовательского списка исключений в MVP. `.git` пропускается; symlink не обходятся. Скрытые папки и worktrees входят в обход. |
| BH-016 | Git ignore rules не исключают находки и не определяют их полезность или допустимость удаления. |
| BH-017 | Java/Kotlin и Go отложены в roadmap; Xcode DerivedData вне scope. |

Открытие нового пустого окна должно быть доступно стандартным способом через File. Несколько targets в одном окне и aggregate scan между окнами не входят в MVP.

## Обнаруживаемые артефакты

| Семейство | Признаки |
|---|---|
| Swift | Каталог `.build`. |
| Rust | Каталог `target` рядом с `Cargo.toml`. |
| Python build outputs | `build` и `dist` рядом с `pyproject.toml`, `setup.py` или `setup.cfg`. |
| Python bytecode | Корни `__pycache__`; отдельные `.pyc` и `.pyo` вне уже найденного корня. |
| Python metadata | `.egg-info`. |
| Python tool caches | `.pytest_cache`, `.mypy_cache`, `.ruff_cache`, `.pytype`. |
| Python environments | Каталог с `pyvenv.cfg`, включая `.venv` и `venv`; имя само по себе недостаточно. |
| Python test environments | `.tox` и `.nox`, отдельный Kind для test environments. |

Environments включены по умолчанию в GUI и обозначаются отдельно от build outputs и caches. Это выбранная по делегированному пользователем решению policy; она не утверждает, что окружения случайны или ненужны. Текущий CLI требует `--include-envs`; изменение его defaults не требуется этим PRD.

Имена кешей — heuristics, не доказательство содержимого. Не читать пользовательский код и не запускать инструменты проектов ради определения типа. Custom Cargo `target-dir`, глобальные caches и произвольные output names не обещаются в MVP. Правила описывают конкретные признаки, а не полноту обнаружения любого мусора.

## Жизненный цикл окна и scan

1. Пустое окно предлагает drop или выбор папки.
2. Валидный target получает session-only filesystem access; начинается scan.
3. Находки появляются и обновляются по мере обхода. Действия доступны для известного пути до окончания измерения.
4. Завершение показывает итоговый размер и количество корней. Ошибки обозначают incomplete report.
5. Stop сохраняет найденные строки и маркирует незавершённые измерения как partial/stopped.
6. Rescan очищает прежний отчёт и создаёт новую scan generation.
7. Замена target очищает прежние результаты; события отменённой generation не влияют на новый отчёт.
8. Закрытие окна отменяет его scan и освобождает доступ после остановки worker. Другие окна продолжают работу.

Если открытие папки отменено пользователем или drop невалиден, существующий target и scan сохраняются. Удаление или недоступность находки во время scan/Reveal отображается как ошибка; Copy Path остаётся возможным для известного пути.

## Размеры и streaming

Основной размер — allocated bytes с форматированием в binary units. Итог суммирует внешние корни без повторного учёта вложенных артефактов. Это измеренный размер дерева, а не обещание освобождения указанного объёма: hard links, APFS clones/compression и параллельные изменения файлов влияют на результат.

Предлагаемый UX контракт: строка появляется при обнаружении корня; Size показывает «измеряется», затем окончательный размер. Точность и частота промежуточных чисел зависят от Rust streaming API. До подтверждения API не обещать процент готовности или ETA. При Stop или ошибке частичный размер визуально отличается от полного; итог также маркируется как partial/incomplete.

Сортировка и selection не должны терять выбранную строку при обновлениях. Предлагаемый стартовый режим — Path; сортировка по Size доступна, но конкретное поведение незавершённых строк определяется при проектировании таблицы.

## Вне MVP

- Любые cleanup actions, recommendations о безопасном удалении и автоматическое обслуживание.
- DerivedData, глобальное сканирование системы и автоматическое расширение выбранного scope.
- Пользовательские исключения, сохранённые проекты, bookmarks для будущих запусков, восстановление окон.
- Background monitoring, periodic scans, telemetry, accounts и cloud sync.
- Обязательный JSON export из GUI; существующий CLI export сохраняется.
- Intel и старые версии macOS.

## Критерии приёмки

| Проверка | Ожидаемый результат |
|---|---|
| Drop, click/open, File command | Выбор папки запускает scan в правильном окне; отмена диалога ничего не заменяет. |
| Streaming fixture с несколькими корнями | Строки появляются до завершения всего обхода; окно принимает ввод во время scan. |
| Nested `.build/target/__pycache__` | Один внешний корень и одна сумма; вложенные пути отсутствуют в таблице. |
| Python fixtures | Environment требует `pyvenv.cfg`; Git ignore не скрывает находки. |
| Stop, Rescan, replace target | Частичные результаты сохраняются только при Stop; поздние события старой generation отбрасываются. |
| Два окна | Отмена, target, selection и ошибки одного окна не меняют другое. |
| Reveal и Copy Path | Finder показывает содержащую папку/находку; clipboard получает абсолютный путь. |
| Ошибка чтения | Доступная часть обходится, предупреждение и подробности доступны, итог помечен incomplete. |
| Read-only fixture | Scan и UI actions не меняют содержимое target. |
| Sandboxed signed build | Реальный drop/open даёт достаточный доступ; scope корректно освобождается; недоступные paths не обходятся. |
| Relaunch | Новое пустое окно без прежних targets или результатов. |

Автоматические tests подтверждают classification, precedence, aggregation и generation isolation. Runtime tests подтверждают sandbox access, Finder, multiwindow и responsiveness. Сборка и CI сами по себе не подтверждают прохождение App Review.

## Этапы доставки и roadmap

1. **Skeleton — implemented:** app target arm64/macOS 26, window model, Table, mock states, SpecificationCore policies и sandbox entitlements. Проверка: policy/state tests и UI screenshots.
2. **Rust integration — implemented:** общий scanner core, streaming, cancellation и FFI adapter. Проверка: Rust tests/Clippy/CLI fixtures, arm64 static-library build, Swift test по настоящему Rust FFI scanner; подпись и sandbox runtime ещё не подтверждены.
3. **Runtime UX — in progress:** настоящий drop/open запускает read-only scan; завершить Finder/clipboard actions и подтвердить доступ в signed sandboxed build на macOS 26 с несколькими окнами.
4. **Store readiness:** signing, bundle identity, icon, metadata, screenshots и release archive. Конкретные account, bundle ID и коммерческая модель ещё не выбраны. Публикация — отдельная задача.
5. **После MVP:** Java/Kotlin и Go; правила и доказательства обнаружения проектируются отдельно, без заранее обещанных directory heuristics.

Численные performance targets не согласованы. До их выбора критерии — реальные streaming updates до завершения и отзывчивый интерфейс; benchmark должен явно указывать corpus, filesystem/cache state и размерные semantics.

## Связанные документы

- [ADR 0001](adr/0001-macos-app-architecture.md): architecture, ownership и evidence gates.
- [CLI usage](../README.md): текущее поведение Rust CLI; PRD описывает будущий GUI, а не уже реализованные возможности.

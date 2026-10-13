# Hunter / Rifle / BuildHunter / Arsenal — модульные границы

Дата: 2026-10-13. Утверждены названия и продуктовое направление. Контракты ниже —
reviewable descriptive drafts; код не перенесён и новые репозитории не созданы.
Baseline реализации: `48badf9e097783505151dd47fbafe290f0af25e2`.

## Решение

| Модуль / repository | Доступ | Ответственность | Чего не знает |
|---|---|---|---|
| **Hunter** | Public | Filesystem traversal, metadata/size measurement, platform guards, streaming, cancellation, optional resource profile | Артефакты, языки, filter IDs, plugins, UI, Store |
| **Rifle** | Public | Абстрактные rule IR/schema, validation, plan compilation/evaluation; Swift authoring SDK | Файловая система, Hunter, build artifacts, Store |
| **BuildHunter** | Public | Artifact vocabulary, базовые plugins, SpecificationCore policy, Hunter/Rifle adapter, CLI, macOS GUI и FFI/session assembly | Универсальный поисковый UI, индекс, LLM |
| **Arsenal** | Private | Коммерческие плагины как данные, fixtures, пояснения и совместимость | Свой runtime, filesystem access, привилегированные hooks |

**Rifle — одна `f`.** Это окончательное выбранное имя, не прежнее предположение
`Riffle`. `BuildHunterCatalog` больше не используется как имя каталога.
Имя отдельного Store composition repository пока открыто; `BuildHunterStore`
было предложением, а не утверждённым пятым именем.

## Зависимости и владение эффектами

```text
BuildHunter CLI / macOS GUI
          │
          ├─ Artifact policy + basic plugins
          └─ Product-owned ScanAssembly adapter
                  ├─ Hunter: acquire facts / traverse / measure
                  └─ Rifle: validate rules / evaluate supplied facts

Private Store composition
          ├─ pinned public BuildHunter / Hunter / Rifle
          └─ pinned Arsenal data
```

BuildHunter зависит от Hunter и Rifle. **Hunter и Rifle не зависят друг от друга**
и не импортируют BuildHunter. Arsenal authoring использует Rifle SDK/schema,
но runtime Rifle не требует Arsenal. Это dependency/ownership contract;
Hypercode nesting показывает только композицию, не последовательность вызовов.

Hunter получает bounded acquisition/selection plan через generic seam и возвращает
facts с точной filesystem identity. Rifle вычисляет match IDs по предоставленным
facts без скрытого I/O. BuildHunter владеет преобразованием facts, artifact-specific
marker vocabulary, выбором measure/traverse/prune и интерпретацией находок.
Точные Rust traits, ABI structs и Swift API не фиксируются этим документом.

Сам по себе Rifle не может гарантировать отмену чтения диска, а Hunter не может
гарантировать правильность artifact taxonomy. Родительская ScanAssembly отвечает
за совместимые versions/digest, неизменность settings/catalog на scan, маршрутизацию
ошибок, единицы bytes, nested/outer semantics и lifetime всех borrowed данных.

Swift DSL строит portable rule data при authoring/build. Runtime evaluator — Rust.
SpecificationCore остаётся application policy engine; не создавать параллельный
Swift interpreter и не возвращать обязательный Swift callback на каждый файл.
JIT, arbitrary closures/native plugins и выполнение команд не входят в этот этап.

## Текущее состояние и цена выделения

| Сейчас | Целевая ответственность | Что нужно сделать до переноса |
|---|---|---|
| `src/lib.rs`: traversal, workers, measurement, cloud handling | Hunter | Отделить generic facts/lifecycle от artifact-specific names, codes и reporting |
| Там же: `is_policy_candidate`, `may_classify_directory`, `MARKER_*` | Rifle plan + BuildHunter vocabulary | Один catalog-derived selection/prefetch contract; заменить оба hot-path gate вместе |
| Rust CLI policy и Swift classification | BuildHunter basic plugins + Rifle execution | Golden parity, затем явно согласовать profile differences |
| Finite policy table / numeric C codes | Переходный BuildHunter adapter | Сохранить действующий ADR 0002 до версии нового ABI; не объявлять нынешнюю таблицу plugin runtime |
| Swift event bridge, access, generation | BuildHunter platform/session adapter | Сохранить ownership, queue close/unblock, current-generation-only mutation |
| AppKit table, diagram, Finder, Settings, export | BuildHunter application | Не переносить presentation или macOS effects в Hunter/Rifle |
| Коммерческий каталог и Store composition | Arsenal + private composition | Отсутствуют; потребуют отдельной реализации и private CI |

Источники и symbols привязаны в [картах](../specs/hypercode/README.md),
[BoundarySpecs](../specpm.yaml) и [claim matrix](../specs/claim-matrix.md).
Каталог фильтров уже реализован, но загрузчик plugins, schema, SDK и новые crates
не реализованы. Модульную экстракцию нельзя выполнить одним переименованием crate.

## Наблюдаемые различия и решения перед миграцией

1. **Candidate gate:** произвольный callback сейчас не увидит любое имя:
   Rust prefilter и worker prefetch знают текущие артефакты. Оба должны стать
   derived plan, иначе новый plugin будет пропущен или измерен неверно.
2. **CLI/GUI parity:** GUI распознаёт environment по `pyvenv.cfg` независимо от
   имени и включает его по умолчанию. CLI default исключает environments, а
   включённый CLI classifier распознаёт именованные `.venv`/`venv`. CLI и GUI
   также различают `.tox`/`.nox`, bytecode и metadata по-разному. Не усреднять эти
   факты в ложный единый контракт; изменения defaults требуют отдельного решения.
3. **Symlinks:** traversal/measurement не обходят symlink targets, но проверка
   symlink-маркера использует `fs::metadata`. Уточнить no-follow policy для generic
   engine/markers; не скрывать эту наблюдаемую особенность ради красивой спецификации.
4. **Exact paths:** Rust хранит raw bytes; GUI может показать lossy UTF-8 path,
   предупредить и затем построить URL из текста. Безопасность точной идентичности
   Finder/Copy для таких имён не доказана. Перед новым ABI выбрать lossless path
   handle либо явную недоступность действий; код здесь не исправляется.
5. **Memory:** ring ограничивает 4096 undelivered Swift events. Это не предел
   pending work, Rust arena, report rows и всего RSS. Приватные результаты и
   очереди требуют отдельных budgets/measurements; snapshots #61 — историческое
   evidence конкретного исправления, не общий memory contract нового Hunter.
6. **Finder:** `.opened` сейчас означает вызов host API, не проверенное завершение
   открытия Finder. Сам scan, Finder handoff и copy path не изменяют дерево файлов.
   Export и CLI `--profile` записывают выбранный файл, в том числе внутри scanned
   target: ограничения на расположение output сейчас нет. Preferences и clipboard
   также изменяются. Эти явные outputs — исключение из read-only поведения scan.

## Порядок миграции

1. Review baseline/contracts и принять решения C1–C6 из claim matrix.
2. Минимальный Rifle IR/schema и fixtures: публичное базовое и синтетическое
   «коммерческое» правило на одном runtime, без private credentials.
3. Generic Hunter facts/selection seam: candidate selection, marker acquisition
   и prefetch адаптируются совместно, сохраняя legacy golden output.
4. Подключить BuildHunter ScanAssembly/FFI; сохранить client profiles, cancellation,
   cloud behavior, invalid-path handling, outer totals и stable filter IDs.
5. Только после parity/performance validation выделять и версионировать public
   Hunter/Rifle repos, затем private Arsenal и Store composition. Репозитории
   можно подготовить раньше отдельно; это не доказывает успешную экстракцию.

Каждый slice — отдельный PR/stack с зелёным `main`. Gates: source/ABI parity,
fixtures, 200-target Release comparison, time to first finding, CPU/RAM,
cancellation и signed sandbox check. Перенос кода не даёт права изменять
поведение одновременно без отдельного acceptance.

## Поддержка карт и контрактов

При каждом изменении кода идентифицировать затронутые роли/claims и обновлять
maps, BoundarySpecs, bindings, provenance и progress record. Если контракт не
изменился, записать IDs и обоснование, а не менять формулировки для видимости.
Сохранять historical snapshots, инвалидировать affected evidence и повторять
HC/SpecPM structural checks. Требование, source observation, validation и runtime
conformance не взаимозаменяемы. Подробный workflow закреплён в корневом AGENTS.md.

Индекс, LLM и универсальное приложение под другим именем остаются отдельным
будущим продуктом; сейчас достаточно не связывать generic mechanics с артефактами.

# Сверка выделения модулей — 2026-10-22

Сверены публичные `main`, совпадающие с `origin/main` после fetch:

| Репозиторий | Ревизия |
| --- | --- |
| BuildHunter | `6dcf9ba91e67c7d5f3270369d05e03d01da1db5e` |
| Hunter | `f1c9f921fed2ea04aa8e67b84075d9fd3a504e2d` |
| Rifle | `693a284123faaa1ad3192cd88b259ba97a7b5910` |

Это текущий backlog вместо «следующих шагов» из
[аудита 20 октября](modular-refactor-audit-20261020.md), который сохранён как история.
Сверка описывает код и доказательства; она не принимает draft-требования и не
меняет runtime. Состояние приватных Arsenal/BuildHunterMAS здесь не проверялось.

## Результат этапа extraction

**Намеченный перенос общей механики из BuildHunter в Hunter выполнен по коду.**
Последние оставшиеся реализации — Darwin batch backend/prepared lookup и OS
QoS/CPU/RSS — влиты через Hunter #22/#23/#25 и BuildHunter #97/#99. Новая фабрика
или дальнейшее дробление ScanEngine для завершения этого этапа не нужны.

При этом полная целевая система не завершена: зависимости пилота ещё требуют
согласования, а default plugin migration, descriptor traversal и memory budgets
являются самостоятельными работами. Выполненный перенос не доказывает их приёмку.

| Область | Сейчас | Источники |
| --- | --- | --- |
| Traversal и task/result mechanics | В Hunter: очередь, workers, task ledger, child planning, subtree progress, coordinator и ScanEngine | `vendor/hunter-fs-core/src/{work,task,children,ledger,subtree,coordinator,processing,engine}.rs` |
| Lifecycle | В Hunter: guard/backend lifetime, scoped workers/helpers, channel, запуск двигателя и stop/close/join | `vendor/hunter-fs-core/src/{lifetime,session,assembly,launch}.rs` |
| Metadata | В Hunter: portable metadata/listing/byte primitives, prepared measurement, opt-in Darwin acquisition/checked decoder и consume-once lookup | `vendor/hunter-fs-core/src/{backend,listing,measurement,darwin_bulk,batch}.rs` |
| OS primitives | В Hunter: non-materializing guard, requested thread QoS и self CPU/RSS | `vendor/hunter-fs-core/src/{materialization,thread_qos,process_resources}.rs` |
| Scoped marker facts | В Hunter facade `hunter-fs`: Unix descriptor-bound marker observations; пилот использует отдельный старый pin | Hunter `src/scoped_metadata.rs`; `crates/buildhunter-plugin-scanner/src/scope.rs` |
| Pure rule engine | В Rifle: validated immutable programs, three-valued evaluation, generic catalog и canonical digest | Rifle `src/{lib,validate,catalog,identity}.rs`; candidate index и Swift SDK не реализованы |
| Product adapter | Намеренно в BuildHunter: классификация, filters/markers, backend choice, counters/probes, error decoration, ancestry/aggregation, warnings/status и Finished | `src/{lib,measurement,scan_session,policy_table}.rs` |
| Product profile и UI | Намеренно в BuildHunter: cadence, CPU baseline/deltas/%, RAM summaries, profile ABI/JSON, Swift bridge и представление | `src/{profile,resources,profile_report}.rs`, `macos/Sources/BuildHunter` |

`src/measurement.rs` теперь адаптирует Hunter acquisition/lookup, а не содержит
вторую реализацию decoder. `src/materialization.rs` — alias и продуктовая
интерпретация ошибок. Выбор моментов guard/QoS, статуса и публикации — обязанность
сборки приложения. Их наличие в host callbacks не означает незавершённый перенос.
SpecificationCore остаётся Swift application policy; Hunter его не импортирует.

## Оставшиеся зависимости

Default CLI/GUI используют только vendored `hunter-fs-core`:
`c5354e973b5d1d90c438c94b78c06898ffe62500` — 27 файлов, byte-equal upstream;
ревизия является предком Hunter main после merge #25.

Независимый opt-in scanner дополнительно использует Git `hunter-fs` на
`5c8f3c9a1a018039b5aea9b731f2fb744f625b5a`, а plugin-policy — Rifle на
`693a284123faaa1ad3192cd88b259ba97a7b5910`. Это реальные отдельные dependency
sources. Сейчас старый Hunter facade ещё не зависит от нового core; после
обновления pin его новый manifest добавит core-зависимость. Нельзя считать,
что одно изменение SHA автоматически оставит в graph единственную копию core.

Перед выпуском нужны согласованные pins/lockfiles и проверка dependency graph
обоих независимых пакетов. Избавляться от офлайн-vendoring без отдельного решения
не требуется. Обновление библиотечных версий остаётся последним этапом интеграции.

## Backlog с критериями завершения

Порядок ниже — рекомендация по зависимостям, а не разрешение менять поведение.

| ID | Следующая работа / владелец | Проверяемый результат |
| --- | --- | --- |
| R1 | Согласовать dependencies пилота / BuildHunter + Hunter | Точные pins и lockfiles, явная идентичность всех core sources; оба независимых пакета собираются с `--locked`; offline vendor check и marker fixtures проходят. Не заявлять один core, пока graph этого не подтверждает |
| R2 | Закрепить baseline parity / BuildHunter | Golden fixtures фиксируют нынешние target IDs, exclusions, nested/outer roots, size/error/cancellation semantics и различия CLI/GUI (C2). C3 marker-link и C4 lossless path/action decisions записаны до переключения default clients |
| R3 | Завершить безопасную fact acquisition / Hunter + host | Descriptor-bound traversal/measurement с тестами directory replacement; platform support явно описан. Scoped markers пилота не выдаются за защиту всего обхода. Windows marker backend остаётся отдельной незавершённой ветвью |
| R4 | Перевести default clients на каталог / BuildHunter + Rifle | Public basic targets выражены plugins; selection, marker vocabulary и worker prefetch согласованы (C1); CLI/FFI/Settings используют target IDs и catalog identity; golden parity и 200-target end-to-end Release gate проходят. Наличие Rifle Plan само по себе не доказывает candidate indexing |
| R5 | Ввести общие resource budgets / Hunter + BuildHunter | Отдельные лимиты pending work, response transport, ledger/progress, reports/diagnostics и profile backlog; cancellation/backpressure не создают deadlock. Свежие CPU/RSS/latency measurements с заданными пределами (C5), а не только bounded Swift ring |
| R6 | Swift authoring SDK / Rifle | Swift DSL экспортирует versioned data без arbitrary closures/JIT; результат совпадает с portable manifest и общими fixtures |
| R7 | Public/user-plugin delivery / BuildHunter | Import, enable/disable, compatibility/conflict diagnostics и guide работают end-to-end; malformed package не рушит scan; scopes/effects описаны честно (C6) |
| R8 | Commercial composition / Arsenal + BuildHunterMAS | Приватный catalog/license и fixtures, pinned app-bundle composition, signing/release и Store acceptance проверяются отдельно. Тяжёлый private CI не запускается на платных hosted runners без согласования |

R3 предшествует обещанию новой no-follow safety для default plugin path. R2/R4
можно готовить независимо от R5, но это не закрывает C5. C6 — сквозной contract
ограниченных effects: explicit export/profile writes, preferences и clipboard не
приравниваются к filesystem scan. Политика принятия этих границ остаётся за владельцем.
Индекс, LLM и универсальное приложение по-прежнему вне BuildHunter scope.

## Открытые GitHub Issues не равны списку неперенесённого кода

Состояние проверено live; Issues здесь не закрывались и не редактировались.

| Issue | Что обнаружено в коде / что ещё сверить |
| --- | --- |
| [#25](https://github.com/SoundBlaster/BuildHunter/issues/25) | Declarative plugin proposal открыт; pure runtime и opt-in pilot существуют, default delivery не готова. Performance hypotheses ведутся отдельно в `policy-performance-plan.md` |
| [#36](https://github.com/SoundBlaster/BuildHunter/issues/36) | Исторический optimization backlog; перенос в Hunter не закрывает гипотезы ускорения. Нужна отдельная сверка H-ledger, не механическое закрытие issue |
| [#49](https://github.com/SoundBlaster/BuildHunter/issues/49) | Native guard/placement и тесты реализованы; требуемую manual проверку evicted iCloud и фактического errno эта сверка не выполняла |
| [#50](https://github.com/SoundBlaster/BuildHunter/issues/50) | Код различает `CompletedWithChanges` и реальные ошибки; есть delete/create/rename и selected-root tests. Кандидат на отдельную проверку полноты критериев и закрытие, а не новый этап extraction |
| [#51](https://github.com/SoundBlaster/BuildHunter/issues/51) | Opt-in batch metadata и fixture comparison существуют. Default остаётся portable; объединённый batch listing/traversal, реальные измерения и вся приёмка issue не следуют из переноса decoder |
| [#52](https://github.com/SoundBlaster/BuildHunter/issues/52) | FSEvents tracking не найден в текущем Rust/macOS implementation; остаётся отдельной низкоприоритетной фичей |

## Доказательства и границы сверки

Hunter #25: Linux/macOS/Windows checks прошли перед merge. BuildHunter #99 на
`a5cb835b420d26abac4ca5e81e00f58388989d1a`: Rust/CLI трёх платформ,
scanner comparison двух платформ, macOS unit/UI, performance, Release и merge
gate прошли; замечание о Xcode inputs исправлено. Это результаты конкретного
переноса, а не свежие измерения производительности этого аудита.

Свежая сверка фиксирует ревизии и SHA256 выбранных исходников, dependency
manifests/lockfiles и карт. Structural checks и vendor identity проверяются
отдельно: `specs/validation/modular-audit-20261022`. Runtime, UI, home/iCloud scans
и benchmarks не запускались заново. Source observations, draft obligations,
исторические execution receipts и owner acceptance остаются раздельными.

Текущие HC/HCS уже отражают перенесённые ProcessPlatform/BatchMetadata и роли
host adapters. Нового узла ради этой сверки не нужно; их hashes перепроверены.
Подготовка к будущей регенерации из Hypercode не означает, что карты уже являются
полной исполняемой спецификацией проекта. Изменение подхода к архитектуре не входит
в эту сверку.

## R1 follow-up — 2026-10-22 (PR branch)

The dependency inventory above records the audit baseline. The next implementation
aligns the facade to `c5354e9` and uses a workspace-root Git-source patch to share
one verified vendored core. Rifle remains at the already-current pin. R1 is
implemented for these repository workspace graphs, subject to fresh CI/review;
no library versions are raised and external consumers own their own Cargo patches.
See [dependency alignment](plugin-scanner-integration.md#dependency-alignment--2026-10-22)
and `specs/validation/plugin-alignment-20261022`. R2–R8 are unaffected.

## R2 follow-up — 2026-10-22 (corpus/CLI layer)

Shared golden trees freeze reviewed current CLI/GUI rows and per-target exclusions.
Real CLI and deterministic core lifecycle witnesses are implemented; GUI bridge
execution is the upper stack layer. See [migration baseline](migration-parity-baseline.md).
C2 differences are preserved; C3/C4 baseline behavior and remaining migration
choices are explicit. No default cutover or link/path ABI policy change is included.

# ADR 0001 Архитектура BuildHunter для macOS

Дата: 2026-10-01. Статус: принято направление архитектуры по интервью; FFI и streaming contracts требуют реализации и verification. Это не свидетельство готовности macOS приложения.

## Контекст

BuildHunter уже имеет Rust CLI. Новый SwiftUI GUI предназначен для Mac App Store, macOS 26+, Apple Silicon. Он сканирует выбранную папку, показывает строки по мере обхода, поддерживает независимые окна и не удаляет данные. Пользователь требует максимально применить SpecificationCore согласно его skills. [PRD](../PRD.md) определяет продуктовый scope.

Текущий Rust scanner объединяет CLI, traversal, classification и сбор отчёта в `main.rs`. Финальный JSON не обеспечивает GUI streaming, cancellation или session isolation. Их нельзя считать уже существующими функциями.

## Решение

Использовать общий Rust scanner core и SwiftUI application, вызывающее Rust library внутри собственного sandboxed process через небольшой C ABI. CLI становится отдельным adapter к Rust core; GUI не запускает установленный executable из пользовательского PATH.

```text
CLI arguments ──> Rust CLI policy adapter ──┐
                                         │
SwiftUI window ──> scan coordinator ──> FFI ──> Rust traversal/measurement
                        │                  │
              SpecificationCore           └── streaming events
              typed policy decisions                │
                        └──────────────────> window result model
```

Приложение связывает Rust arm64 static library. Конкретная packaging/build integration определяется в implementation task. Это исключает отдельную передачу runtime sandbox grants helper-процессу; отдельно остаются проверки signed build и ABI lifecycle.

## Граница SpecificationCore

SpecificationCore — источник semantic policy для Swift application. Он получает immutable typed facts, возвращает predicates или typed outcomes; effects исполняет owning application layer.

| Ответственность | Владелец |
|---|---|
| Facts о типе узла, имени, marker files, read errors | Rust filesystem adapter |
| Допустимость target, traversal exclusions, artifact/environment classification, action availability | Swift specifications и typed decision routing |
| Чтение metadata, обход, агрегирование bytes, отмена worker | Rust scanner mechanics |
| Получение и удержание filesystem access | Swift platform adapter/scan coordinator |
| Generation isolation, start/stop/rescan и window lifetime | Swift application layer |
| Finder, clipboard и presentation | Swift platform adapters и SwiftUI |

Использовать `Specification` для predicates, `DecisionSpec`/`FirstMatchSpec` для typed ordered choices и явного no-match. Одна concrete specification — один семантический вопрос и отдельный source file. Не превращать parsing, любой `if`, сериализацию, cancellation и I/O в specifications ради количества.

Проверить exact APIs в consuming `Package.resolved` и local checkout перед кодом. Локальный источник: `~/Development/GitHub/Specification Project/SpecificationCore` (единственное число); manifest требует Swift tools 6.1 и предоставляет opt-in trait `Tracing`. GUI ещё не имеет resolved dependency. Для воспроизводимой доставки понадобится зафиксированная remote SPM dependency либо документированная воспроизводимая локальная интеграция; absolute developer path не должен быть требованием release build.

Tracing — возможный diagnostic инструмент, не обязательная функция MVP. При включении использовать стабильные rule identifiers без filesystem paths; избегать process-wide recorder, смешивающего события разных окон.

## Policy и Rust boundary

Не реализовывать авторитетные GUI classification rules второй раз в Rust. Предлагаемый seam: scanner сообщает подготовленные directory/candidate facts и получает typed traversal/classification outcome от Swift policy adapter через synchronous callback на worker thread. Swift rules на этом пути чистые, без MainActor, I/O или UI effects.

Механический prefilter Rust может уменьшать callback volume, но не должен скрывать кандидатов, допускаемых Swift policy. Pre-filter coverage и priority требуют fixture tests. Как альтернатива для реализации допустим batched facts/decisions protocol, если он сохраняет exclusions и внешний-root accounting; его нужно отдельно описать до реализации.

CLI получает собственный Rust policy adapter без зависимости на Swift. Общие fixtures и decision tables проверяют parity там, где profiles совпадают. GUI включает environments по умолчанию; CLI сохраняет текущий opt-in режим. Различие profiles явное, а не случайное расхождение.

Когда выбран внешний artifact root, worker измеряет всё его дерево, не выдавая вложенные artifacts как новые строки. Symlink не обходятся. `.git` pruning применяется при поиске; внутри принятого root measurement учитывает содержимое как часть размера. Root-level facts и priority, включая environment markers, должны быть определены в decision table до реализации.

## Streaming и cancellation

Предлагаемые события: `artifactDiscovered`, `artifactProgress` (optional), `artifactCompleted`, `scanWarning`, `scanFinished`. Каждое содержит scan generation; artifact events имеют устойчивый artifact ID. Завершение scan различает completed, cancelled и failed; наличие read warnings даёт incomplete report.

События передаются в bounded/coalescing buffer и применяются к window model на MainActor. Discovery, completion и warnings нельзя молча терять; optional progress можно объединять. UI не обновляется на каждый filesystem entry. Unknown size отличается от zero bytes; partial size не выглядит окончательным. Пользователю не обещаются ETA или проценты без соответствующих данных.

Каждое окно владеет собственными target, worker handle, generation, results и errors. Stop сохраняет partial rows. Replace/Rescan создаёт новую generation и очищает предыдущий результат. Old-generation events отбрасываются независимо от скорости завершения cancellation. Закрытие окна отменяет только принадлежащую ему работу.

FFI contract обязан определить:

- ownership handle, callback context и payload buffers;
- lifetime Swift callbacks и Rust worker, включая cancellation race;
- explicit free/release функции, thread safety и terminal event semantics;
- безопасное преобразование Rust errors; panic не пересекает C ABI;
- единицы размера и encoding paths, без предположения, что все Unix paths валидны в UTF-8;
- bounded buffering и backpressure, не создающие deadlock при Stop.

Конкретные C signatures пока не приняты. Для invalid UTF-8 paths нужно либо сохранить lossless identity отдельно от display string, либо явно объявлять unsupported entry с warning; нельзя молча использовать lossy string для Reveal/Copy как точный original path.

## Sandbox и доступ

App target использует `com.apple.security.app-sandbox` и `com.apple.security.files.user-selected.read-only`. Target URL получают через system folder selection или поддерживаемый drag-and-drop URL mechanism. URL должен сохранять предоставленный системой доступ; восстановление URL из текстового path не заменяет access grant.

Swift platform adapter удерживает session access на весь lifetime нуждающегося в нём worker и window actions. При замене/закрытии прекращает доступ после завершения соответствующей работы. Не трактовать один Bool от `startAccessingSecurityScopedResource()` как универсальное доказательство доступности или недоступности: реальные read errors остаются источником runtime результата.

Persistent bookmarks и window restoration не нужны, поскольку пользователь отказался от сохранения targets. Нет read-write grant, Full Disk Access, privileged helper, фонового daemon или сети для scan.

Read-only sandbox scan, drag/drop grants и Finder integration должны быть проверены в signed application, а не только через unsandboxed CLI или fake tests. Эти требования не гарантируют принятия App Review.

## Рассмотренные альтернативы

| Альтернатива | Причина не выбирать для первого GUI |
|---|---|
| CLI subprocess + финальный JSON | Нет текущего streaming/cancellation contract; runtime file grants дочернему процессу требуют отдельного решения. Sandbox inheritance не передаёт автоматически dynamic PowerBox rights. |
| Полностью Swift scanner | Упрощает bridge, но дублирует уже созданный Rust scanner и расходится с выбранным общим core. |
| Все rules в Rust, Swift только рисует | Не выполняет требование о существенной domain policy через SpecificationCore. |
| Specification для каждой механической ветки | Противоречит skills: parsing, aggregation, orchestration и effects остаются обычным кодом. |
| XPC worker | Возможная будущая process isolation, но требует дополнительного access/lifecycle protocol; MVP в нём не нуждается. |

## Последствия и verification gates

Положительные: один Rust measurement engine, самостоятельный CLI, именованные Swift policies, native UX и отсутствие subprocess grants в основном GUI пути.

Стоимость: FFI ownership/threading, отдельный CLI policy adapter, policy parity fixtures и сборка Rust arm64 library вместе с Xcode app. In-process Rust fault может завершить GUI; isolation не обеспечивается этим решением.

1. Decision tables/tests: classification, overlap/precedence, no-match, `.git`, symlink, environments и outer-root aggregation.
2. Bridge harness: memory lifetime, terminal events, stop/replacement races, invalid paths, bounded buffers; ни error, ни panic не пересекают ABI.
3. Window tests: независимые scans, stale events, partial results, closing/replacing target.
4. Signed sandbox runtime: настоящие drop/open grants, permissions/read errors, Finder/clipboard и отсутствие writes.
5. Release: arm64/macOS 26 archive, resolved SPM inputs, signing и App Store readiness. Store submission — отдельный gate.

## Источники и evidence boundary

- [Apple App Sandbox](https://developer.apple.com/documentation/security/protecting-user-data-with-app-sandbox): sandbox model и App Store distribution.
- [Apple file access](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox): runtime access to selected resources.
- [Apple entitlement reference](https://developer.apple.com/library/archive/documentation/Miscellaneous/Reference/EntitlementKeyReference/Chapters/EnablingAppSandbox.html): read-only selection, drag interactions и ограничения child-process inheritance. Это archived reference; проверить актуальное SDK поведение в runtime.
- [SpecificationCore](https://github.com/SoundBlaster/SpecificationCore), local manifest и skills `specification-patterns`/`specificationcore`: rules, typed outcomes и разделение policy/effects.

Apple documentation обосновывает ограничения платформы. In-process Rust, policy callback seam и buffering — архитектурные решения проекта, а не требования Apple или уже проверенные свойства implementation.

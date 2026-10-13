# ADR 0003: Declarative plugins, commercial catalog and product boundaries

Дата: 2026-10-13. Статус: направление согласовано владельцем продукта;
реализация не начата. Формат schema v1 и API SDK требуют отдельного review.

Утверждены **Hunter / Rifle / BuildHunter / Arsenal** (Rifle с одной `f`).
[Модульная архитектура](../modular-architecture.md) и
[maps/BoundarySpecs](../../specs/specification-progress.md) отделяют текущую
реализацию от целевого разделения.

## Контекст

[Issue #25](https://github.com/SoundBlaster/BuildHunter/issues/25) предлагает
единый каталог правил обнаружения вместо повторения классификации в scanner,
CLI, FFI и GUI. Настоящее решение уточняет предложение с учётом монетизации
и последующего переиспользования движка.

BuildHunter остаётся приложением для поиска локальных build artifacts, caches
и environments. Пользователь может собрать open-source версию, написать свой
плагин самостоятельно или с помощью агента и пользоваться результатом.
Платная версия из Mac App Store предоставляет готовый расширенный каталог.
Ценность покупки — покрытие, проверенные правила, удобство установки,
сопровождение и обновления без самостоятельной сборки.

Универсальный поиск станет отдельным продуктом под другим, пока не выбранным
именем. Он сможет использовать тот же движок, а позднее — индекс и LLM.
Эти возможности не входят в текущую разработку BuildHunter.

## Решение и состав продукта

| Часть | Доступ и назначение |
|---|---|
| Hunter, Rifle SDK/runtime и BuildHunter CLI/GUI | Open source; публичная сборка работает самостоятельно |
| Базовый каталог | Open source; нынешние Swift, Rust и Python проверки переводятся в плагины |
| Расширенный каталог Arsenal | Отдельный private repository; готовые пакеты популярных экосистем |
| Store distribution | Платная сборка общего приложения с расширенным каталогом в app bundle |

Не поддерживать отдельный fork GUI или scanner для платной версии.
Отсутствие private dependencies не должно ломать публичную сборку или тесты.
Store-сборка, напротив, должна завершаться ошибкой при отсутствии обязательного
каталога, а не незаметно выпускать базовую комплектацию.

Стоимость, точный состав платного набора, имя Store composition repository и
дополнительные варианты оплаты пока не выбраны. Первый целевой сценарий —
платное приложение с включённым каталогом; подписка, аккаунты и отдельные
покупки пакетов не требуются этим решением.

## Что такое плагин

Плагин — версионированный пакет правил одной экосистемы, например Python,
Gradle или Unity. Он содержит:

- stable plugin ID, версию, title, description и совместимость со schema;
- targets со stable IDs, признаками совпадения и категорией результата;
- декларацию необходимых facts и семантики вложенных совпадений;
- положительные и отрицательные filesystem fixtures с ожидаемыми результатами;
- пользовательские пояснения и необязательные подсказки восстановления.

Плагин не получает filesystem handle, сетевой доступ или права на выполнение
команд. Подсказки наподобие `cargo clean` — только текст. Удаление, изменение
файлов, следование symlinks и расширение доступа не могут включаться правилом.
Плагин не объявляет собственную произвольную executable category или UI.

Targets сохраняют существующие filter IDs (`swift.build`, `rust.target`,
`python.*`). Новая версия каталога не должна менять смысл ID или терять
пользовательские exclusions. Категории результатов остаются управляемой core
taxonomy, общей для CLI и GUI.

## Авторство на Swift, исполнение в Rust

Предпочтительный authoring interface — типизированный Swift DSL, создающий
описания правил. Переносимый формат правил доступен и без Swift: пользователь
может подготовить манифест напрямую. Предлагаемый canonical interchange —
versioned JSON IR; точные поля, encoding и schema v1 согласуются отдельно.
TOML из исходного #25 не является обязательным runtime-форматом; отдельный
TOML importer пока не требуется.

```text
Swift DSL / portable manifest
          ↓
Validation and export to versioned rule IR
          ↓
Rust catalog loader → immutable indexes / execution plan
          ↓
Traversal, matching, measurement and streaming
          ↓
CLI / macOS application policy and presentation
```

Swift DSL строит данные, а не сериализует arbitrary Swift closures.
Генератор работает при подготовке пакета/сборке. Установленному приложению
не нужны Swift compiler, Xcode или аккаунт LLM-провайдера.

Runtime правил один: Rifle на Rust интерпретирует общий IR. Hunter отвечает
за filesystem mechanics; BuildHunter — за artifact facts/policy и адаптер.
Hunter и Rifle не импортируют друг друга. Не создавать второй Swift
интерпретатор правил и не вызывать Swift callback на каждый filesystem entry
ради выполнения plugin predicates. Подготовка execution plan — компиляция
структуры данных, не машинного кода. JIT, embedded scripting runtime,
загружаемые dylibs и arbitrary native hooks в первой версии не используются.

SpecificationCore сохраняет роль semantic application policy: пользовательские
фильтры, outer-root semantics и допустимость действий над результатами. Решения
должны поступать в scanner через immutable scan plan/подходящий compiled
contract, а не автоматически возвращать per-entry FFI overhead.

Это будущая смена classification seam относительно ADR 0001. ADR 0002 и
нынешняя finite policy table остаются действующими до проверенной миграции.
Документ не утверждает, что catalog runtime или новый ABI уже существуют.

## Matching, capabilities и performance

В первом релизе plugins используют facts, уже полученные обходом:
имя, тип, own/parent markers и наличие artifact ancestor. Никакого чтения
содержимого пользовательского кода или запуска инструментов проекта.

Exact-name index выбирает небольшой набор кандидатов; suffix и marker-only
правила имеют отдельные ограниченные пути. Candidate selection, обнаружение
markers и worker prefetch выводятся из одного каталога. Не обещать O(1) для
произвольного предиката или неограниченного числа совпавших правил.

Schema/loader обязаны ограничивать сложность и размер правил, количество
targets/markers и диагностировать конфликты до начала скана. Неоднозначность
generic names (`target`, `build`, `Library`) разрешается явно, без зависимости
от порядка загрузки файлов. Точный specificity/priority contract проверяется
в schema review, включая случаи с `any`, `all` и marker-only rules.

Каталог и настройки фиксируются на начало скана. Версия и canonical catalog
digest входят в воспроизводимый отчёт. Изменение пакета не меняет выполняемый
скан. Invalid user package получает диагностику и не обрушает приложение;
оставшийся допустимый набор должен быть явно виден пользователю. Global
catalog conflicts нельзя молча разрешать случайным выбором победителя.

Для будущего общего движка различать capability tiers:

| Tier | Пример | Стоимость и статус |
|---|---|---|
| Имеющиеся facts | Имена, типы, маркеры | Первая версия BuildHunter plugins |
| Дополнительные metadata | Размер, дата изменения | Возможное расширение; требует cost contract |
| Содержимое | Текст, JSON, другие structured probes | Позднее; отдельные read budgets и отмена |

Capability — требование к данным, а не разрешение обходить sandbox. Эффекты
исполняет движок. Дорогие probes выполняются только после дешёвого отбора,
по возможности разделяются между правилами и не влияют на scans, которым
не нужны. Конкретный API этих tiers сейчас не реализуется.

Performance принимается по Release parity/benchmark gates. Сравнивать runtime
с нынешними проверками, включая загрузку каталога, time to first finding,
CPU, peak/live memory и cancellation. Корпус из 200 targets нужен до расширения
каталога. Без измерений не заявлять равенство скорости с hand-written code.

## Репозитории, сборка и лицензирование

Утверждённые имена и целевая структура (репозитории этим ADR не создаются):

| Репозиторий | Состав |
|---|---|
| `Hunter` — public | Generic traversal/metadata/measurement/lifecycle; без артефактов |
| `Rifle` — public | Abstract rule engine/schema/validation/evaluation и Swift authoring SDK; без filesystem I/O |
| `BuildHunter` — public | Artifact policy, базовые plugins, Hunter/Rifle adapter, CLI/macOS GUI и тесты |
| `Arsenal` — private | Коммерческие plugins, fixtures и пояснения |
| Store composition — private, имя открыто | Pipeline, закрепляющий public modules и Arsenal; без второго GUI fork |

Private Store pipeline фиксирует source revisions, SDK/schema versions и
catalog digest; публичный CI не нуждается в private credentials. Секреты и
коммерческие определения не попадают в публичные logs/artifacts. Интеграционные
тесты приватного набора выполняются в private pipeline. Публичные smoke tests
используют синтетические пакеты без коммерческого содержимого.

Приватность исходного каталога не гарантирует секретность правил в поставленном
приложении: ресурсы можно извлечь, binaries исследовать. Не строить модель
дохода на невозможности воспроизведения простых предикатов или на JIT/обфускации.

Уже опубликованный MIT-код остаётся доступным по выданной лицензии; удаление
файлов из public repository не делает прежние проверки эксклюзивными.
Открытый базовый набор сохраняется. Новые коммерческие пакеты разрабатываются
отдельно; их лицензия и права на third-party assets проверяются перед поставкой.

## Mac App Store

Первая Store-версия включает коммерческий каталог в app bundle. Новые версии
поставляются с обновлением приложения через Store. Downloadable commercial
plugins, remote executable code и собственный updater не входят в этот этап.

Импорт пользовательского декларативного запроса — потенциальный документ для
уже встроенного поискового движка, а не обещанный обход ограничений Apple.
Его Store UX и допустимость требуют отдельной проверки. Публичная/self-build
версия должна позволять подключение пользовательских правил; точные пути,
import UX и availability в Store edition ещё не согласованы.

Это консервативный способ поставки, а не гарантия одобрения App Review.
На дату решения проверены [App Review Guidelines](https://developer.apple.com/app-store/review/guidelines/),
в частности 2.4.5 и 2.5.2. App Sandbox и user-selected access сохраняются;
plugins не обещают доступ ко всему диску. Повторить policy review перед релизом.

## Отдельный будущий продукт

Универсальное приложение использует Hunter/Rifle, но имеет другое имя,
позиционирование и UI. В его будущем scope могут входить произвольные
пользовательские запросы, persistent index и LLM для составления запросов.
Ни сроки, ни имя, ни модель индекса, ни LLM provider сейчас не выбраны.

BuildHunter не получает индекс, LLM, универсальный query UI, accounts или
background indexing в рамках plugins. Достаточно отделить mechanics от
artifact catalog, policy и presentation. Hunter/Rifle выделяются по утверждённым
границам после parity/performance validation; преждевременная универсализация
для будущего продукта не требуется.
LLM, если появится, не должен автоматически превращать пользовательский
запрос в произвольный исполняемый код с filesystem access.

## Следствия и доставка

Преимущества: один runtime, переносимые проверяемые пакеты, самостоятельная
public build и единый код приложения для обеих комплектаций.
Цена: ограниченный DSL не выражает любую программу; сложные probes потребуют
новых core capabilities. Native hooks рассматриваются только при доказанной
потребности, с отдельным ADR об isolation и lifecycle.

Поэтапный план и критерии приёмки: [Plugin delivery plan](../plugin-delivery-plan.md).
Разработка runtime, schema, private repositories и Store submission — следующие
задачи, а не результат этого documentation-only решения.

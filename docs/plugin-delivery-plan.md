# План доставки plugins и коммерческого каталога

Дата: 2026-10-13. Implementation stages ниже **pending**. Имена Hunter/Rifle/BuildHunter/Arsenal
утверждены; [модульные контракты](modular-architecture.md) остаются draft. Согласовано архитектурное
направление в [ADR 0003](adr/0003-declarative-plugins-and-product-boundaries.md),
но Plugin SDK, schema и catalog runtime пока не реализованы.

Этот план уточняет Phases 0–5 из [#25](https://github.com/SoundBlaster/BuildHunter/issues/25).
Performance hypotheses и их доказательства остаются в
[отдельном ledger](policy-performance-plan.md). Наличие нынешнего filter catalog
не означает наличие plugins.

Каждый этап доставляется отдельным PR или небольшим стеком зависимых PRs;
после каждого merge `main` должен оставаться зелёным.

## Продуктовый контракт

- Public BuildHunter: полноценная самостоятельная сборка, открытый SDK,
  базовые Swift/Rust/Python plugins, возможность собственных правил.
- Paid Mac App Store edition: тот же application/engine плюс готовый расширенный
  private catalog. Установка и обновления без сборки или LLM-аккаунта.
- Не удалять уже открытый базовый набор ради искусственной эксклюзивности.
- Не обещать секретность правил, поставленных покупателю в app bundle.
- Универсальный поиск под другим именем, индекс и LLM — отдельный будущий
  продукт без текущего implementation scope.

## Этап 0: schema и SDK contract

Подготовлен [контракт `1-draft.1`](plugin-contract-v1.md), две JSON Schema и
примеры с wire-shape tests. Это материал для review, а не завершение этапа:
семантические тесты runtime, SDK и принятие решений ниже ещё впереди.

- [ ] Зафиксировать versioned rule IR/schema v1, stable IDs и совместимость.
- [ ] Спроектировать Swift DSL, экспортирующий данные без arbitrary closures.
- [ ] Определить linter, complexity limits, specificity/priority и diagnostics.
- [ ] Определить fixture format, canonicalization и catalog digest.
- [ ] Развести public/self-build user-plugin loading и Store delivery contract.

**Приёмка:** reviewed schema и contract tests. Синтетическое правило можно
одинаково представить через Swift DSL и portable manifest. Это данные,
которые понимает будущий Rust runtime; runtime Swift compiler не требуется.

## Этап 1: открытый базовый набор без изменения поведения

- [ ] Представить текущие filter targets как public built-in plugins.
- [ ] Перенести положительные/отрицательные fixtures и precedence cases.
- [ ] Подготовить golden comparison старого и нового представления.
- [ ] Сохранить IDs, exclusions, nested/outer-root semantics, size semantics
  и явно зафиксированные различия CLI/GUI defaults, включая environments.

**Приёмка:** golden diff пуст на фиксированных деревьях. Публичная сборка
не зависит от private repositories. Каталог не дублирует runtime-интерпретаторы.

## Этап 2: единый Rifle runtime и Hunter facts seam

- [ ] Rifle: loader, linter и immutable execution plan/indexes.
- [ ] Hunter: generic facts/measurement/lifecycle; BuildHunter: product-owned adapter.
- [ ] Ни Hunter, ни Rifle не импортируют друг друга или artifact vocabulary.
- [ ] Общие candidate selection, marker vocabulary и worker prefetch.
- [ ] CLI listing/exclusions из каталога, версия/digest в отчётах.
- [ ] Воспроизводимые Release сравнения с текущим scanner и 200-target corpus.

**Приёмка:** совпадают результаты, warnings, cancellation и partial semantics;
performance gates проходят. Отдельно измерены startup, time to first finding,
CPU и RAM; нет unbounded cache и per-entry Swift callback для plugin matching.

## Этап 3: FFI, application policy и Settings

- [ ] Согласованный ABI для target IDs, descriptors и catalog identity.
- [ ] SpecificationCore policy поверх typed facts и immutable scan settings.
- [ ] Settings строятся из plugin groups; есть load/conflict diagnostics.
- [ ] IDs сохраняют exclusions; обновление не меняет активный scan.
- [ ] Удалить старые копии классификации только после parity acceptance.

**Приёмка:** CLI и GUI классифицируют одинаковые facts одинаково; намеренные
presentation/policy различия задокументированы. Unit, UI, FFI и Release
performance checks проходят; signed sandbox runtime проверен отдельно.

## Этап 4: private catalog и Store composition

- [x] Выбрать private repository выпуска: BuildHunterMAS (репозиторий создан).
- [ ] Реализовать private Arsenal и выбрать лицензию нового каталога.
- [ ] Сделать pinned composition общего приложения и коммерческого набора.
- [ ] Обеспечить private fixture/integration CI и воспроизводимую поставку.
- [ ] Добавлять популярные экосистемы по пакетам: кандидаты Node, Gradle,
  Maven, CocoaPods, Unity и другие; Go остаётся в roadmap. Это не обязательный
  обещанный launch bundle — окончательный состав выбирается отдельно.
- [ ] На каждый target — positive/negative fixtures, объяснение и collision cases.
- [ ] Включать готовый каталог в app bundle; обновлять через Mac App Store.

**Приёмка:** public build работает без секретов. Store build требует нужную
версию private catalog и не делает silent fallback. Каталог проходит те же
semantic/performance gates. Не публикуются private definitions, secrets или
пользовательские scan traces в public CI artifacts.

## Этап 5: пользовательские пакеты и документация

- [ ] Подключение правил без пересборки public/self-build приложения.
- [ ] Enable/disable, compatibility и понятные invalid-package diagnostics.
- [ ] Authoring guide для человека и агента, Swift DSL examples, schema reference.
- [ ] Проверить Store import UX отдельно, прежде чем обещать его покупателям.

**Приёмка:** пользовательский пакет работает на том же runtime; malformed
пакет диагностируется без падения, effective catalog понятен и воспроизводим.
Нет executable hooks, JIT, скрытого I/O или расширения sandbox permissions.

## Открытые решения перед реализацией

Точные schema/API и cost limits; итоговый private plugin bundle; цена и условия
покупки; pipeline BuildHunterMAS; правила лицензирования contributions; location/import
UX пользовательских пакетов; правила разрешения конфликтов между пакетами.
Чтение содержимого файлов и global/known-location caches требуют отдельного
scope и не включаются автоматически в первую версию.

Первый implementation slice после review: один открытый и один синтетический
«private» пакет на одинаковом runtime с parity/performance evidence. Настоящие
private repositories создаются отдельно; документация не требует их наличия.

Pure adapter pilot in independent `crates/buildhunter-plugin-policy` package: [implementation scope](plugin-adapter.md). Это validation/matching slice, а не завершённый этап подключения scanner/FFI.

# Общий serial ScanEngine

ScanEngine — одноразовый caller-thread entrypoint в Hunter: run выполняет root admission
и цикл ответов, drive принимает уже подготовленные задачи. Он объединяет существующие
TraversalCoordinator, ChildTraversal::route и SelectedMeasurementPlan::resolve.
ScanHandler — callbacks приложения; их State, Branch, Selection и Context для Hunter
непрозрачны. Это composition API, которое вызывают внутри guarded ScanSession после
запуска workers. Оно само не создаёт backend или worker pool.

BuildHunter ArtifactHandler теперь реализует этот один контракт. ArtifactBranch хранит
chain/parent_len для продуктовой иерархии. Hunter PendingSelection хранит selected plan
и этот opaque context; приложение больше не собирает PendingMeasure и отдельные
routing/measurement adapters. Код классификации и накопления результата остаётся в нём.

После receipt accounting и проверки Stop host response обрабатывает SetupFailed либо
возвращает DirectoryResult. Для Listed двигатель вызывает telemetry, забирает состояние,
учитывает обещанный Measured и передаёт listing в prepare_directory. Результат подготовки:

- Abandon: не завершать ветку и не чинить deferred state; Stop/partial решает host.
- Complete: записать ignored measurement, если он обещан, и завершить ветку.
- Traverse: route children с прежними индексами; при Stop освободить префикс без finish.
  Пустую невыбранную ветку завершить сразу; непустую измерить немедленно либо отложить.

Measured забирает deferred selection один раз. Ignored result не меняет размеры/status.
Root metadata имеет приоритет над worker result, затем только отсутствующие результаты
используют fallback. Cached errors не повторяются. Checkpoint cadence и пути прежние.
finish_directory выполняется до освобождения selected/cached buffers и branch context;
общий partial finish — до освобождения ledger. Drained означает отсутствие ожидаемых
ответов, включая случай после host failure, и не означает успешный ScanStatus.

Host сохраняет cloud/access/cancel policy, markers/classification, scheduling meaning,
byte mode/aggregation, warnings/profile/status/events. Receiver освобождается до join;
итоговый Profile/Finished публикуется после join в прежнем guard scope. C ABI и GUI
контракт не меняются. Новый API не даёт snapshot, автоматического retry, I/O interruption
или RAM/CPU bound. Descriptor recursion, backend/worker factory и default plugin
migration — отдельные оставшиеся этапы. Контракты draft; receipts в
specs/validation/hunter-engine-20261019 отделяют source/local tests от hosted/live evidence.

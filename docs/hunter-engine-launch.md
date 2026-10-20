# Запуск engine в guarded session

ScanChannel — владелец прежнего unbounded mpsc-канала ответов; sender() клонирует producer.
Его создаёт приложение в исходной точке startup, перед QoS/profile capture. EngineLaunch
объединяет borrowed backend/raw root, channel, cloud preflight и idle interval. Политика
и native/batch backend selection остаются в приложении.

ScanSession::run_engine(launch, worker, start_helpers, make_host, collect) использует
существующий scoped run. Порядок: helpers → workers → drop original producer →
caller-thread host factory и begin_profile → ScanEngine → drop unread receiver →
collect(host,outcome) → helper stop → queue close → join → возврат результата.

collect выполняется до join: BuildHunter возвращает прежний tuple полей отчёта и
освобождает host-only progress в исходной точке. Возвращать весь host можно только
при осознанном выборе такого lifetime; общий API не делает этого автоматически.
Factory/engine panic пропускает collect и освобождает receiver до join. Panic в collect
также приводит к shutdown/join перед drop backend/guard. Worker panic может проявиться
только при join, уже после collect: этот callback не означает успех и не публикует
terminal status. Финальный Profile/Finished и ScanReport остаются после join.

Host/result не требуют Send; worker tasks/producers подчиняются прежним требованиям
thread scope. Изменения не добавляют threads, stop probes, I/O, retries или normalization.
Host сохраняет cooperative termination и доступ/guards, QoS, markers/classification,
byte/profile aggregation, warnings/status и внешние producer handles. Канал не ограничен
по памяти; I/O не прерывается. Это composition существующего assembly-контракта,
без нового package/capability. Descriptor recursion, default plugins, C5 budgets и
живой home/iCloud/UI прогон этим изменением не подтверждены. Свежие local receipts:
specs/validation/hunter-engine-launch-20261020; hosted CI проверяется отдельно.

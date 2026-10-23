# Engine launch validation

Baseline main 4e018c1ad0490342b7e69579916c093a101c50c3; immutable core snapshot
40c0f5384c70879f529c6069d835dafff316567a. Local Debug/Release workspace tests pass:
127 unit tests (118 portable core + 9 Unix scoped metadata) plus two compile-fail
doctests. Four new launch witnesses cover receiver-before-collect, host state release
before helper stop/queue closure/join, non-Send host/result, factory panic, collection
panic with join before backend/guard drop, and declined root without metadata/pump.
Existing helper/worker/engine panic and scoped shutdown tests remain regression gates.

Fmt, all-target clippy, Hypercode parse/validation and SpecPM pass (zero errors,
13 existing unresolved-capability/duplicate-ID warnings). Tool/source hashes, actual
command outputs and TB5 output paths are retained. Channels remain unbounded; host
still owns policy, protocol/producer termination, worker/helper effects/guards and
post-join terminal publication. No live home/cloud/UI, autonomous native/batch factory,
resource bound or general speedup is claimed. New constraints remain draft and older
receipts remain historical; hosted platform checks are separate evidence.

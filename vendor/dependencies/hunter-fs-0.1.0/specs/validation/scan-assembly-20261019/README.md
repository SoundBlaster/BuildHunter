# Guarded scan assembly validation

Runtime/core snapshot: 9f445cf8468e2f341d8e8e20782854fb391d5979, based on main
2c4d27452882957dee7c348fcd2a0e5af2599e74. Four new integrated witnesses and one
compile-fail single-use witness supplement existing lifecycle/admission tests.
Debug/Release workspace: 101 unit tests (92 core + 9 Unix), plus the compile-fail
doctest. Formatting, all-target clippy, HC parse/validation and SpecPM checked in
commands.json. Outputs/caches use the verified TB5 volume.

Boundary/map source hashes identify the final documented inputs. New contract
remains draft. Existing root admission, backend lifetime and scoped-session
contracts are composed, not broadened. No full recursive engine, throughput/RAM
improvement, live cloud, home-directory or UI acceptance follows from these tests.

# Directory task boundary review

Addresses https://github.com/SoundBlaster/Hunter/pull/4#discussion_r4279332693.
The core README and hunter.fs.directory_tasks scope now use opaque host-policy
language; no classification model belongs to Hunter. Existing capability IDs,
interfaces, constraints and map composition are unchanged. Runtime Rust sources
are byte-identical to e9b77e4; prior test receipts remain historical at
../directory-tasks-20261015. No runtime tests or performance measurements rerun
for this documentation-only change. commands.json records fresh structural
validation (SpecPM: zero errors, three pre-existing warnings) and tool paths.
source-identity.json identifies changed documentation and unchanged maps.

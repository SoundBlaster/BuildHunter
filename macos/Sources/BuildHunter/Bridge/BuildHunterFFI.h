#ifndef BUILD_HUNTER_FFI_H
#define BUILD_HUNTER_FFI_H

#include <stddef.h>
#include <stdint.h>

typedef struct BHCandidateFacts {
    const uint8_t *node_name;
    size_t node_name_len;
    uint8_t is_directory;
    uint8_t is_symbolic_link;
    uint8_t has_artifact_ancestor;
    uint32_t own_marker_files;
    uint32_t parent_marker_files;
} BHCandidateFacts;

typedef struct BHCandidateDecision {
    uint32_t action;
    uint32_t language;
    uint32_t kind;
} BHCandidateDecision;

enum { BH_POLICY_TABLE_VERSION = 1, BH_POLICY_TABLE_CELL_COUNT = 4609 };

typedef struct BHScanEvent {
    uint32_t event_type;
    uint64_t artifact_id;
    const uint8_t *path;
    size_t path_len;
    uint32_t language;
    uint32_t kind;
    uint64_t bytes;
    uint8_t partial;
    uint32_t status;
    const uint8_t *message;
    size_t message_len;
} BHScanEvent;

typedef int32_t (*BHPolicyCallback)(void *context, const BHCandidateFacts *facts,
                                    BHCandidateDecision *decision);
typedef void (*BHEventCallback)(void *context, const BHScanEvent *event);

/* The policy callback writes action 0 (traverse), 1 (prune), or 2 (classify).
 * Returning 0 or an unknown action fails the scan and stops traversal. Any
 * artifact still being measured gets a partial completion before the terminal event.
 * Event types: 1 discovered, 2 measured, 3 warning, 4 finished.
 * Terminal status / bh_scan result: 0 complete, 1 cancelled, 2 incomplete, 3 failed.
 * Callbacks run serially on the calling thread; all payload pointers are borrowed
 * until the callback returns. Keep the callback context alive until bh_scan returns.
 * bh_scan reads the file system on internal worker threads, joined before it returns;
 * event order between unrelated subtrees and artifact IDs may differ between runs.
 */
/* Canonical extensible filter catalog. Returned UTF-8 JSON is NUL-terminated,
 * immutable and valid for the lifetime of the process; callers must not free it. */
const char *bh_search_filter_catalog_json(void);

void *bh_scan_control_create(void);
/* Cancellation is thread-safe. Destroy the control only after bh_scan returns;
 * serialize destruction with cancellation so no caller uses a destroyed handle.
 */
void bh_scan_control_cancel(void *control);
void bh_scan_control_destroy(void *control);
int32_t bh_scan(void *control, const uint8_t *root, size_t root_len,
                uint8_t apparent_size, BHPolicyCallback policy_callback,
                BHEventCallback event_callback, void *context);

/* Version 1 has 18 ordered UTF-8 name classes x 256 fact flags, plus the
 * final invalid-UTF-8 cell. Cell facts borrow process-lifetime name bytes.
 * Returns 0 for an out-of-range index or null facts pointer. */
uint32_t bh_policy_table_version(void);
size_t bh_policy_table_cell_count(void);
int32_t bh_policy_table_cell_facts(size_t index, BHCandidateFacts *facts);

/* The decisions array must contain exactly bh_policy_table_cell_count()
 * readable entries. Actions are 0 (traverse), 1 (prune), or 2 (classify).
 * Invalid arguments return status 3 before traversal or callbacks. Entries are
 * copied before scanning; event payloads retain the usual callback lifetime. */
int32_t bh_scan_with_policy_table(void *control, const uint8_t *root,
                                  size_t root_len, uint8_t apparent_size,
                                  uint32_t table_version,
                                  const BHCandidateDecision *decisions,
                                  size_t decision_count,
                                  BHEventCallback event_callback, void *context);

#endif

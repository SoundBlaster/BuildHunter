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
 */
void *bh_scan_control_create(void);
/* Cancellation is thread-safe. Destroy the control only after bh_scan returns;
 * serialize destruction with cancellation so no caller uses a destroyed handle.
 */
void bh_scan_control_cancel(void *control);
void bh_scan_control_destroy(void *control);
int32_t bh_scan(void *control, const uint8_t *root, size_t root_len,
                uint8_t apparent_size, BHPolicyCallback policy_callback,
                BHEventCallback event_callback, void *context);

#endif

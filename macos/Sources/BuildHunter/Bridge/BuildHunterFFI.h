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

void *bh_scan_control_create(void);
void bh_scan_control_cancel(void *control);
void bh_scan_control_destroy(void *control);
int32_t bh_scan(void *control, const uint8_t *root, size_t root_len,
                uint8_t apparent_size, BHPolicyCallback policy_callback,
                BHEventCallback event_callback, void *context);

#endif

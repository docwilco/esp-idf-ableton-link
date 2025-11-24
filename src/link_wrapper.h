#ifndef LINK_WRAPPER_H
#define LINK_WRAPPER_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque pointer to Link instance
typedef struct LinkInstance LinkInstance;

// Link instance management
LinkInstance* link_create(double bpm);
void link_destroy(LinkInstance* link);

// Session control
void link_enable(LinkInstance* link, bool enable);
bool link_is_enabled(const LinkInstance* link);

// Tempo (BPM) control
double link_get_tempo(const LinkInstance* link);
void link_set_tempo(LinkInstance* link, double bpm);

// Beat/phase access
double link_get_beat_at_time(const LinkInstance* link, int64_t micros, double quantum);
double link_get_phase_at_time(const LinkInstance* link, int64_t micros, double quantum);

// Timing
int64_t link_clock_micros(void);

#ifdef __cplusplus
}
#endif

#endif // LINK_WRAPPER_H

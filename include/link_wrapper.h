#ifndef LINK_WRAPPER_H
#define LINK_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque pointer to Link instance
typedef struct LinkInstance LinkInstance;

// Opaque pointer to SessionState
typedef struct SessionStateInstance SessionStateInstance;

// Link instance management
LinkInstance* link_create(double bpm);
void link_destroy(LinkInstance* link);

// Session control
void link_enable(LinkInstance* link, bool enable);
bool link_is_enabled(const LinkInstance* link);
size_t link_num_peers(const LinkInstance* link);

// Session state capture/commit (application thread)
SessionStateInstance* link_capture_app_session_state(const LinkInstance* link);
void link_commit_app_session_state(LinkInstance* link, SessionStateInstance* state);

// SessionState management
void session_state_destroy(SessionStateInstance* state);

// SessionState tempo access
double session_state_tempo(const SessionStateInstance* state);
void session_state_set_tempo(SessionStateInstance* state, double bpm, int64_t at_time_micros);

// SessionState beat/phase access
double session_state_beat_at_time(const SessionStateInstance* state, int64_t micros, double quantum);
double session_state_phase_at_time(const SessionStateInstance* state, int64_t micros, double quantum);
int64_t session_state_time_at_beat(const SessionStateInstance* state, double beat, double quantum);

// SessionState beat mapping
void session_state_request_beat_at_time(SessionStateInstance* state, double beat, int64_t at_time_micros, double quantum);
void session_state_force_beat_at_time(SessionStateInstance* state, double beat, int64_t at_time_micros, double quantum);

// SessionState transport (start/stop sync)
bool session_state_is_playing(const SessionStateInstance* state);
void session_state_set_is_playing(SessionStateInstance* state, bool is_playing, int64_t at_time_micros);

// Timing
int64_t link_clock_micros(void);

#ifdef __cplusplus
}
#endif

#endif // LINK_WRAPPER_H

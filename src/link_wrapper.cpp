#include "link_wrapper.h"
#include <ableton/Link.hpp>
#include <chrono>

// C wrapper for ableton::Link
struct LinkInstance {
    ableton::Link link;
    
    LinkInstance(double bpm) : link(bpm) {}
};

// C wrapper for ableton::Link::SessionState
struct SessionStateInstance {
    ableton::Link::SessionState state;
    
    SessionStateInstance(ableton::Link::SessionState s) : state(s) {}
};

extern "C" {

LinkInstance* link_create(double bpm) {
    try {
        return new LinkInstance(bpm);
    } catch (...) {
        return nullptr;
    }
}

void link_destroy(LinkInstance* link) {
    delete link;
}

void link_enable(LinkInstance* link, bool enable) {
    if (link) {
        link->link.enable(enable);
    }
}

bool link_is_enabled(const LinkInstance* link) {
    if (link) {
        return link->link.isEnabled();
    }
    return false;
}

size_t link_num_peers(const LinkInstance* link) {
    if (link) {
        return link->link.numPeers();
    }
    return 0;
}

SessionStateInstance* link_capture_app_session_state(const LinkInstance* link) {
    if (link) {
        try {
            return new SessionStateInstance(link->link.captureAppSessionState());
        } catch (...) {
            return nullptr;
        }
    }
    return nullptr;
}

void link_commit_app_session_state(LinkInstance* link, SessionStateInstance* state) {
    if (link && state) {
        link->link.commitAppSessionState(state->state);
    }
}

void session_state_destroy(SessionStateInstance* state) {
    delete state;
}

double session_state_tempo(const SessionStateInstance* state) {
    if (state) {
        return state->state.tempo();
    }
    return 120.0;
}

void session_state_set_tempo(SessionStateInstance* state, double bpm, int64_t at_time_micros) {
    if (state) {
        state->state.setTempo(bpm, std::chrono::microseconds(at_time_micros));
    }
}

double session_state_beat_at_time(const SessionStateInstance* state, int64_t micros, double quantum) {
    if (state) {
        return state->state.beatAtTime(std::chrono::microseconds(micros), quantum);
    }
    return 0.0;
}

double session_state_phase_at_time(const SessionStateInstance* state, int64_t micros, double quantum) {
    if (state) {
        return state->state.phaseAtTime(std::chrono::microseconds(micros), quantum);
    }
    return 0.0;
}

int64_t session_state_time_at_beat(const SessionStateInstance* state, double beat, double quantum) {
    if (state) {
        return state->state.timeAtBeat(beat, quantum).count();
    }
    return 0;
}

void session_state_request_beat_at_time(SessionStateInstance* state, double beat, int64_t at_time_micros, double quantum) {
    if (state) {
        state->state.requestBeatAtTime(beat, std::chrono::microseconds(at_time_micros), quantum);
    }
}

void session_state_force_beat_at_time(SessionStateInstance* state, double beat, int64_t at_time_micros, double quantum) {
    if (state) {
        state->state.forceBeatAtTime(beat, std::chrono::microseconds(at_time_micros), quantum);
    }
}

bool session_state_is_playing(const SessionStateInstance* state) {
    if (state) {
        return state->state.isPlaying();
    }
    return false;
}

void session_state_set_is_playing(SessionStateInstance* state, bool is_playing, int64_t at_time_micros) {
    if (state) {
        state->state.setIsPlaying(is_playing, std::chrono::microseconds(at_time_micros));
    }
}

int64_t link_clock_micros(void) {
    return std::chrono::duration_cast<std::chrono::microseconds>(
        std::chrono::system_clock::now().time_since_epoch()
    ).count();
}

} // extern "C"

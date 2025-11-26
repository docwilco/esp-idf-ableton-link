#include "link_wrapper.h"
#include <ableton/Link.hpp>
#include <chrono>

// C wrapper for ableton::Link
struct LinkInstance {
    ableton::Link link;
    
    LinkInstance(double bpm) : link(bpm) {}
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

double link_get_tempo(const LinkInstance* link) {
    if (link) {
        auto sessionState = link->link.captureAppSessionState();
        return sessionState.tempo();
    }
    return 120.0;
}

void link_set_tempo(LinkInstance* link, double bpm) {
    if (link) {
        auto sessionState = link->link.captureAppSessionState();
        sessionState.setTempo(bpm, link->link.clock().micros());
        link->link.commitAppSessionState(sessionState);
    }
}

double link_get_beat_at_time(const LinkInstance* link, int64_t micros, double quantum) {
    if (link) {
        auto sessionState = link->link.captureAppSessionState();
        auto time = std::chrono::microseconds(micros);
        return sessionState.beatAtTime(time, quantum);
    }
    return 0.0;
}

double link_get_phase_at_time(const LinkInstance* link, int64_t micros, double quantum) {
    if (link) {
        auto sessionState = link->link.captureAppSessionState();
        auto time = std::chrono::microseconds(micros);
        return sessionState.phaseAtTime(time, quantum);
    }
    return 0.0;
}

int64_t link_clock_micros(void) {
    return std::chrono::duration_cast<std::chrono::microseconds>(
        std::chrono::system_clock::now().time_since_epoch()
    ).count();
}

} // extern "C"

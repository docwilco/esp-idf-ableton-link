#include "link_wrapper.h"
#include <ableton/Link.hpp>
#include <chrono>
#include <cwchar>
#include <cstdlib>
#include <cstdio>
#include <cstring>
#include <cstdarg>
#include <cerrno>

// Stub implementations for missing newlib functions
// These are required by libstdc++ locale support but not provided by ESP-IDF's newlib
extern "C" {

// Standard C functions that should be in newlib but aren't being linked properly
// These symbols exist but the linker can't find them due to link order issues
// We provide simple implementations

float strtof(const char* nptr, char** endptr) {
    return static_cast<float>(strtod(nptr, endptr));
}

double strtod(const char* nptr, char** endptr) {
    // Simple implementation using atof
    double result = atof(nptr);
    if (endptr) {
        const char* p = nptr;
        // Skip whitespace
        while (*p == ' ' || *p == '\t' || *p == '\n' || *p == '\r' || *p == '\f' || *p == '\v') p++;
        // Skip sign
        if (*p == '+' || *p == '-') p++;
        // Skip digits before decimal
        while (*p >= '0' && *p <= '9') p++;
        // Skip decimal point and digits after
        if (*p == '.') {
            p++;
            while (*p >= '0' && *p <= '9') p++;
        }
        // Skip exponent
        if (*p == 'e' || *p == 'E') {
            p++;
            if (*p == '+' || *p == '-') p++;
            while (*p >= '0' && *p <= '9') p++;
        }
        *endptr = const_cast<char*>(p);
    }
    return result;
}

int sscanf(const char* str, const char* format, ...) {
    // Minimal stub - return 0 (no items matched)
    // Full implementation is complex and not needed for Link
    (void)str;
    (void)format;
    return 0;
}

// Wide character stubs
wchar_t* wmemcpy(wchar_t* dest, const wchar_t* src, size_t n) {
    return static_cast<wchar_t*>(memcpy(dest, src, n * sizeof(wchar_t)));
}

wchar_t* wmemmove(wchar_t* dest, const wchar_t* src, size_t n) {
    return static_cast<wchar_t*>(memmove(dest, src, n * sizeof(wchar_t)));
}

wchar_t* wmemset(wchar_t* dest, wchar_t ch, size_t n) {
    for (size_t i = 0; i < n; ++i) {
        dest[i] = ch;
    }
    return dest;
}

wchar_t* wmemchr(const wchar_t* s, wchar_t c, size_t n) {
    for (size_t i = 0; i < n; ++i) {
        if (s[i] == c) {
            return const_cast<wchar_t*>(&s[i]);
        }
    }
    return nullptr;
}

size_t wcslen(const wchar_t* s) {
    size_t len = 0;
    while (*s++) ++len;
    return len;
}

int wcscoll(const wchar_t* s1, const wchar_t* s2) {
    while (*s1 && *s1 == *s2) { ++s1; ++s2; }
    return *s1 - *s2;
}

size_t wcsxfrm(wchar_t* dest, const wchar_t* src, size_t n) {
    size_t len = wcslen(src);
    if (dest && n > 0) {
        size_t copy_len = (len < n - 1) ? len : n - 1;
        wmemcpy(dest, src, copy_len);
        dest[copy_len] = L'\0';
    }
    return len;
}

size_t wcsftime(wchar_t* s, size_t maxsize, const wchar_t* format, const struct tm* timeptr) {
    // Minimal implementation - just return 0 (no characters written)
    if (s && maxsize > 0) {
        s[0] = L'\0';
    }
    return 0;
}

int wctob(wint_t c) {
    return (c >= 0 && c <= 127) ? static_cast<int>(c) : EOF;
}

wint_t btowc(int c) {
    return (c >= 0 && c <= 127) ? static_cast<wint_t>(c) : WEOF;
}

wctype_t wctype(const char* property) {
    return 0;
}

int iswctype(wint_t wc, wctype_t desc) {
    return 0;
}

wint_t towupper(wint_t wc) {
    if (wc >= L'a' && wc <= L'z') {
        return wc - L'a' + L'A';
    }
    return wc;
}

wint_t towlower(wint_t wc) {
    if (wc >= L'A' && wc <= L'Z') {
        return wc - L'A' + L'a';
    }
    return wc;
}

size_t strxfrm(char* dest, const char* src, size_t n) {
    size_t len = strlen(src);
    if (dest && n > 0) {
        size_t copy_len = (len < n - 1) ? len : n - 1;
        memcpy(dest, src, copy_len);
        dest[copy_len] = '\0';
    }
    return len;
}

size_t strftime(char* s, size_t maxsize, const char* format, const struct tm* timeptr) {
    // ESP-IDF should provide this, but in case it doesn't...
    if (s && maxsize > 0) {
        s[0] = '\0';
    }
    return 0;
}

} // extern "C"

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


#![no_std]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

// Opaque pointer to the Link instance
pub enum LinkInstance {}

extern "C" {
    // Link instance management
    pub fn link_create(bpm: f64) -> *mut LinkInstance;
    pub fn link_destroy(link: *mut LinkInstance);
    
    // Session control
    pub fn link_enable(link: *mut LinkInstance, enable: bool);
    pub fn link_is_enabled(link: *const LinkInstance) -> bool;
    
    // Tempo (BPM) control
    pub fn link_get_tempo(link: *const LinkInstance) -> f64;
    pub fn link_set_tempo(link: *mut LinkInstance, bpm: f64);
    
    // Beat/phase access
    pub fn link_get_beat_at_time(link: *const LinkInstance, micros: i64, quantum: f64) -> f64;
    pub fn link_get_phase_at_time(link: *const LinkInstance, micros: i64, quantum: f64) -> f64;
    
    // Timing
    pub fn link_clock_micros() -> i64;
}

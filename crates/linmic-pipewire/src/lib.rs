use anyhow::{ensure, Result};
use linmic_audio::{Consumer, Sample};
use linmic_common::Counters;
use std::{
    ffi::{c_char, c_void, CString},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
unsafe extern "C" {
    fn linmic_pw_create(
        name: *const c_char,
        desc: *const c_char,
        fill: unsafe extern "C" fn(*mut c_void, *mut f32, usize),
        user: *mut c_void,
    ) -> *mut c_void;
    fn linmic_pw_destroy(p: *mut c_void);
    fn linmic_pw_id(p: *mut c_void) -> u32;
    fn linmic_pw_state(p: *mut c_void) -> i32;
    fn linmic_pw_description(p: *mut c_void, desc: *const c_char);
}
struct Callback {
    consumer: Consumer<Sample>,
    stats: Arc<Counters>,
    silent: Arc<AtomicBool>,
}
unsafe extern "C" fn fill(p: *mut c_void, out: *mut f32, n: usize) {
    // The C stream owns callback scheduling; destruction stops/joins it before dropping this box.
    let c = unsafe { &mut *p.cast::<Callback>() };
    let output = unsafe { std::slice::from_raw_parts_mut(out, n) };
    let silent = c.silent.load(Ordering::Acquire);
    let mut missing = false;
    for s in output {
        let v = c.consumer.pop();
        *s = if silent {
            0.
        } else {
            v.map(|s| {
                if s.generation == c.stats.generation.load(Ordering::Acquire) {
                    s.value
                } else {
                    0.
                }
            })
            .unwrap_or_else(|_| {
                missing = true;
                0.
            })
        };
    }
    if missing && !silent {
        c.stats.underruns.fetch_add(1, Ordering::Relaxed);
    }
    c.stats.quantum.store(n as u32, Ordering::Relaxed);
    c.stats
        .output_samples
        .store(c.consumer.slots() as u32, Ordering::Relaxed);
}
pub struct Source {
    raw: *mut c_void,
    callback: Box<Callback>,
}
impl Source {
    pub fn new(
        name: &str,
        desc: &str,
        consumer: Consumer<Sample>,
        stats: Arc<Counters>,
        silent: Arc<AtomicBool>,
    ) -> Result<Self> {
        let mut callback = Box::new(Callback {
            consumer,
            stats,
            silent,
        });
        let n = CString::new(name)?;
        let d = CString::new(desc)?;
        let raw = unsafe {
            linmic_pw_create(
                n.as_ptr(),
                d.as_ptr(),
                fill,
                (&mut *callback as *mut Callback).cast(),
            )
        };
        ensure!(!raw.is_null(), "PipeWire source creation failed");
        Ok(Self { raw, callback })
    }
    pub fn poll(&self) -> bool {
        let state = unsafe { linmic_pw_state(self.raw) };
        let connected = state >= 2;
        self.callback
            .stats
            .pipewire_connected
            .store(connected, Ordering::Relaxed);
        self.callback
            .stats
            .node_id
            .store(unsafe { linmic_pw_id(self.raw) }, Ordering::Relaxed);
        state >= 0
    }
    pub fn description(&self, s: &str) {
        if let Ok(d) = CString::new(s) {
            unsafe { linmic_pw_description(self.raw, d.as_ptr()) }
        }
    }
}
impl Drop for Source {
    fn drop(&mut self) {
        unsafe { linmic_pw_destroy(self.raw) }
        self.callback
            .stats
            .pipewire_connected
            .store(false, Ordering::Relaxed);
    }
}

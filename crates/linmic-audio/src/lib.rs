pub mod jitter;
use anyhow::{ensure, Result};
pub use rtrb::{Consumer, Producer, RingBuffer};
use std::ffi::{c_int, c_void};
#[link(name = "opus")]
unsafe extern "C" {
    fn opus_decoder_create(rate: c_int, ch: c_int, error: *mut c_int) -> *mut c_void;
    fn opus_decoder_destroy(p: *mut c_void);
    fn opus_decode(
        p: *mut c_void,
        data: *const u8,
        len: c_int,
        pcm: *mut i16,
        frame: c_int,
        fec: c_int,
    ) -> c_int;
    fn opus_decoder_ctl(p: *mut c_void, request: c_int, ...) -> c_int;
    fn opus_encoder_create(
        rate: c_int,
        ch: c_int,
        application: c_int,
        error: *mut c_int,
    ) -> *mut c_void;
    fn opus_encoder_destroy(p: *mut c_void);
    fn opus_encode(
        p: *mut c_void,
        pcm: *const i16,
        frame: c_int,
        data: *mut u8,
        max: c_int,
    ) -> c_int;
    fn opus_encoder_ctl(p: *mut c_void, request: c_int, ...) -> c_int;
}
pub struct Decoder(*mut c_void);
impl Decoder {
    pub fn new() -> Result<Self> {
        let mut e = 0;
        let p = unsafe { opus_decoder_create(48000, 1, &mut e) };
        ensure!(e == 0 && !p.is_null(), "Opus decoder: {e}");
        Ok(Self(p))
    }
    pub fn reset(&mut self) {
        unsafe {
            opus_decoder_ctl(self.0, 4028);
        }
    }
    pub fn decode(&mut self, data: Option<&[u8]>, out: &mut [i16], fec: bool) -> Result<usize> {
        ensure!(!out.is_empty() && out.len() <= 5760, "invalid output size");
        let (p, n) = data.map_or((std::ptr::null(), 0), |d| (d.as_ptr(), d.len() as c_int));
        let n = unsafe {
            opus_decode(
                self.0,
                p,
                n,
                out.as_mut_ptr(),
                out.len() as c_int,
                fec as c_int,
            )
        };
        ensure!(n >= 0, "Opus decode error {n}");
        Ok(n as usize)
    }
}
impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { opus_decoder_destroy(self.0) }
    }
}
pub struct Encoder(*mut c_void);
impl Encoder {
    pub fn new(bitrate: u32, low_delay: bool) -> Result<Self> {
        let mut e = 0;
        let p =
            unsafe { opus_encoder_create(48000, 1, if low_delay { 2051 } else { 2048 }, &mut e) };
        ensure!(e == 0 && !p.is_null(), "Opus encoder {e}");
        let s = Self(p);
        unsafe {
            ensure!(opus_encoder_ctl(p, 4002, bitrate as c_int) == 0, "bitrate");
            opus_encoder_ctl(p, 4010, 5);
            opus_encoder_ctl(p, 4016, 0);
        }
        Ok(s)
    }
    pub fn encode(&mut self, pcm: &[i16], out: &mut [u8]) -> Result<usize> {
        ensure!([240, 480, 960].contains(&pcm.len()), "invalid frame");
        let n = unsafe {
            opus_encode(
                self.0,
                pcm.as_ptr(),
                pcm.len() as c_int,
                out.as_mut_ptr(),
                out.len() as c_int,
            )
        };
        ensure!(n >= 0, "Opus encode {n}");
        Ok(n as usize)
    }
    pub fn fec(&mut self, loss: i32) {
        unsafe {
            opus_encoder_ctl(self.0, 4012, (loss > 0) as c_int);
            opus_encoder_ctl(self.0, 4014, loss.clamp(0, 100));
        }
    }
}
impl Drop for Encoder {
    fn drop(&mut self) {
        unsafe { opus_encoder_destroy(self.0) }
    }
}
#[derive(Default)]
pub struct Dsp {
    smoothed_gain: f32,
    level: f32,
    limiter: f32,
    agc_gain: f32,
    fade: usize,
}
impl Dsp {
    pub fn reset(&mut self) {
        *self = Self {
            limiter: 1.,
            smoothed_gain: 1.,
            agc_gain: 1.,
            fade: 480,
            ..Default::default()
        };
    }
    pub fn process(
        &mut self,
        pcm: &[i16],
        out: &mut [f32],
        gain_db: f32,
        muted: bool,
        gate: bool,
        agc: bool,
    ) -> (f32, f32) {
        let gain = 10f32.powf(gain_db / 20.);
        let (mut sum, mut peak) = (0., 0f32);
        for (&x, y) in pcm.iter().zip(out.iter_mut()) {
            self.smoothed_gain += 0.002 * (gain - self.smoothed_gain);
            let mut s = if muted {
                0.
            } else {
                x as f32 / 32768. * self.smoothed_gain
            };
            self.level = 0.995 * self.level + 0.005 * s.abs();
            if gate && self.level < 0.006 {
                s *= self.level / 0.006;
            }
            if agc {
                let target = (0.15 / self.level.max(0.01)).clamp(0.25, 4.);
                self.agc_gain += 0.0001 * (target - self.agc_gain);
                s *= self.agc_gain;
            }
            let target = (0.891 / s.abs().max(0.891)).min(1.);
            if target < self.limiter {
                self.limiter = target
            } else {
                self.limiter += 0.00026 * (target - self.limiter)
            }
            s *= self.limiter;
            if self.fade > 0 {
                s *= 1. - self.fade as f32 / 480.;
                self.fade -= 1;
            }
            *y = s;
            sum += s * s;
            peak = peak.max(s.abs());
        }
        ((sum / pcm.len().max(1) as f32).sqrt(), peak)
    }
}
/// Streaming fractional resampler. phase and previous sample cross frame boundaries.
pub struct Drift {
    integral: f64,
    phase: f64,
    previous: f32,
    pub ratio: f64,
}
impl Default for Drift {
    fn default() -> Self {
        Self {
            integral: 0.,
            phase: 0.,
            previous: 0.,
            ratio: 1.,
        }
    }
}
impl Drift {
    pub fn update(&mut self, occupancy: usize, target: usize) {
        let e = (occupancy as f64 - target as f64) / 48000.;
        self.integral = (self.integral + e * 0.000002).clamp(-0.002, 0.002);
        self.ratio = (1. + e * 0.015 + self.integral).clamp(0.995, 1.005);
    }
    pub fn process(&mut self, input: &[f32], out: &mut [f32]) -> usize {
        if input.is_empty() {
            return 0;
        }
        let mut n = 0;
        while self.phase < input.len() as f64 && n < out.len() {
            let i = self.phase.floor() as usize;
            let frac = (self.phase - i as f64) as f32;
            let a = if i == 0 { self.previous } else { input[i - 1] };
            out[n] = a + (input[i] - a) * frac;
            n += 1;
            self.phase += self.ratio;
        }
        self.phase -= input.len() as f64;
        self.previous = *input.last().unwrap();
        n
    }
}
#[derive(Clone, Copy)]
pub struct Sample {
    pub value: f32,
    pub generation: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opus_roundtrip() {
        for size in [240, 480, 960] {
            let mut e = Encoder::new(48000, size == 240).unwrap();
            let mut d = Decoder::new().unwrap();
            let pcm: Vec<i16> = (0..size)
                .map(|i| ((i as f32 * 0.0576).sin() * 8000.) as i16)
                .collect();
            let mut b = [0; 1144];
            let n = e.encode(&pcm, &mut b).unwrap();
            let mut out = [0; 960];
            assert_eq!(
                d.decode(Some(&b[..n]), &mut out[..size], false).unwrap(),
                size
            );
            assert_eq!(d.decode(None, &mut out[..size], false).unwrap(), size);
        }
    }
    #[test]
    fn mute_limiter() {
        let mut d = Dsp::default();
        d.reset();
        let mut out = [0.; 960];
        let (_, p) = d.process(&[32767; 960], &mut out, 12., false, false, false);
        assert!(p <= 0.892);
        let (r, p) = d.process(&[32767; 960], &mut out, 12., true, false, false);
        assert_eq!((r, p), (0., 0.));
    }
    #[test]
    fn drift_bound() {
        let mut d = Drift::default();
        for _ in 0..100000 {
            d.update(48000, 960)
        }
        assert!((0.995..=1.005).contains(&d.ratio));
        let mut out = [0.; 1000];
        let n = d.process(&[0.5; 960], &mut out);
        assert!((950..=966).contains(&n));
    }
    #[test]
    fn ring_overflow() {
        let (mut p, mut c) = RingBuffer::new(2);
        p.push(1).unwrap();
        p.push(2).unwrap();
        assert!(p.push(3).is_err());
        assert_eq!(c.pop().unwrap(), 1);
    }
}

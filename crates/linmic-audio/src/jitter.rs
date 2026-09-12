use linmic_protocol::{newer, Header, MAX_PAYLOAD};
use std::time::{Duration, Instant};
const SLOTS: usize = 512;
#[derive(Clone)]
pub struct Packet {
    pub header: Header,
    pub payload: [u8; MAX_PAYLOAD],
}
impl Packet {
    pub fn new(header: Header, data: &[u8]) -> Self {
        let mut payload = [0; MAX_PAYLOAD];
        payload[..data.len()].copy_from_slice(data);
        Self { header, payload }
    }
    pub fn data(&self) -> &[u8] {
        &self.payload[..self.header.payload_len as usize]
    }
}
#[derive(Debug, PartialEq)]
pub enum Push {
    Accepted,
    Duplicate,
    Late,
    TooFar,
}
// Inline storage intentionally avoids per-packet heap allocation.
#[allow(clippy::large_enum_variant)]
pub enum Due {
    Wait,
    Packet(Packet),
    Missing(u16),
}
pub struct JitterBuffer {
    slots: Box<[Option<Packet>]>,
    next: Option<u32>,
    due: Option<Instant>,
    frame: u16,
    pub target_ms: u32,
    last_arrival: Option<(Instant, u64)>,
    pub jitter_ms: f64,
    min: u32,
    max: u32,
    adaptive: bool,
    last_adjust: Instant,
}
impl JitterBuffer {
    pub fn new(target: u32, min: u32, max: u32, adaptive: bool) -> Self {
        Self {
            slots: vec![None; SLOTS].into_boxed_slice(),
            next: None,
            due: None,
            frame: 480,
            target_ms: target,
            last_arrival: None,
            jitter_ms: 0.,
            min,
            max,
            adaptive,
            last_adjust: Instant::now(),
        }
    }
    pub fn reset(&mut self) {
        self.slots.fill(None);
        self.next = None;
        self.due = None;
        self.last_arrival = None;
        self.jitter_ms = 0.;
    }
    pub fn push(&mut self, p: Packet, now: Instant) -> Push {
        let seq = p.header.sequence;
        if let Some(n) = self.next {
            if seq != n && !newer(seq, n) {
                return Push::Late;
            }
            if seq.wrapping_sub(n) >= SLOTS as u32 {
                return Push::TooFar;
            }
        } else {
            self.next = Some(seq);
            self.frame = p.header.frame_samples;
            self.due = Some(now + Duration::from_millis(self.target_ms as u64));
        }
        let slot = seq as usize % SLOTS;
        if self.slots[slot].is_some() {
            return Push::Duplicate;
        }
        if let Some((arrival, samples)) = self.last_arrival {
            if p.header.sample_index >= samples {
                let delta = (now.duration_since(arrival).as_secs_f64() * 1000.
                    - (p.header.sample_index - samples) as f64 / 48.)
                    .abs();
                self.jitter_ms += (delta - self.jitter_ms) / 16.;
            }
        }
        if self
            .last_arrival
            .is_none_or(|(_, s)| p.header.sample_index > s)
        {
            self.last_arrival = Some((now, p.header.sample_index));
        }
        if self.adaptive && now.duration_since(self.last_adjust) >= Duration::from_secs(1) {
            let wanted = (self.min as f64 + self.jitter_ms * 3.).ceil() as u32;
            let target = wanted.clamp(self.min, self.max);
            let old = self.target_ms;
            self.target_ms = if target > old {
                target.min(old + 5)
            } else {
                old.saturating_sub(1).max(target)
            };
            if self.target_ms > old {
                self.due = self
                    .due
                    .map(|t| t + Duration::from_millis((self.target_ms - old) as u64));
            } else if self.target_ms < old {
                self.due = self.due.and_then(|t| {
                    t.checked_sub(Duration::from_millis((old - self.target_ms) as u64))
                });
            }
            self.last_adjust = now;
        }
        self.slots[slot] = Some(p);
        Push::Accepted
    }
    pub fn pop(&mut self, now: Instant) -> Due {
        let Some(due) = self.due else {
            return Due::Wait;
        };
        if now < due {
            return Due::Wait;
        }
        let seq = self.next.unwrap();
        self.next = Some(seq.wrapping_add(1));
        self.due = Some(due + Duration::from_micros(self.frame as u64 * 1_000_000 / 48000));
        match self.slots[seq as usize % SLOTS].take() {
            Some(p) => {
                self.frame = p.header.frame_samples;
                Due::Packet(p)
            }
            None => Due::Missing(self.frame),
        }
    }
    pub fn peek_next(&self) -> Option<&Packet> {
        self.next
            .and_then(|n| self.slots[n as usize % SLOTS].as_ref())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn p(seq: u32) -> Packet {
        Packet::new(
            Header {
                flags: 0,
                session: 1,
                sequence: seq,
                sample_index: seq as u64 * 480,
                sender_time_us: 0,
                frame_samples: 480,
                payload_len: 1,
            },
            &[1],
        )
    }
    #[test]
    fn reorder_loss_duplicate() {
        let t = Instant::now();
        let mut j = JitterBuffer::new(20, 10, 80, false);
        assert_eq!(j.push(p(0), t), Push::Accepted);
        assert_eq!(j.push(p(2), t), Push::Accepted);
        assert_eq!(j.push(p(2), t), Push::Duplicate);
        assert!(matches!(j.pop(t), Due::Wait));
        assert!(matches!(
            j.pop(t + Duration::from_millis(20)),
            Due::Packet(_)
        ));
        assert!(matches!(
            j.pop(t + Duration::from_millis(30)),
            Due::Missing(480)
        ));
        assert_eq!(j.push(p(1), t), Push::Late);
        assert!(matches!(
            j.pop(t + Duration::from_millis(40)),
            Due::Packet(_)
        ));
    }
    #[test]
    fn wrap() {
        let t = Instant::now();
        let mut j = JitterBuffer::new(10, 10, 80, false);
        j.push(p(u32::MAX), t);
        j.push(p(0), t);
        assert!(matches!(
            j.pop(t + Duration::from_millis(10)),
            Due::Packet(_)
        ));
        assert!(matches!(
            j.pop(t + Duration::from_millis(20)),
            Due::Packet(_)
        ));
        j.reset();
        assert!(matches!(j.pop(t), Due::Wait));
    }
}

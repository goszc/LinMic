use anyhow::{bail, ensure, Result};
use chacha20poly1305::{
    aead::{AeadInPlace, KeyInit},
    ChaCha20Poly1305, Nonce, Tag,
};
use std::io::{Read, Write};
pub const HEADER: usize = 40;
pub const MAX_DATAGRAM: usize = 1200;
pub const MAX_PAYLOAD: usize = MAX_DATAGRAM - HEADER - 16;
pub const MUTED: u16 = 1;
pub const FEC: u16 = 2;
pub const DISCONTINUITY: u16 = 4;
pub const KEEPALIVE: u16 = 8;
pub const ENCRYPTED: u16 = 16;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub flags: u16,
    pub session: u32,
    pub sequence: u32,
    pub sample_index: u64,
    pub sender_time_us: u64,
    pub frame_samples: u16,
    pub payload_len: u16,
}
impl Header {
    pub fn encode(&self, b: &mut [u8]) -> Result<()> {
        ensure!(b.len() >= HEADER, "short header");
        b[..4].copy_from_slice(b"LMIC");
        b[4] = 1;
        b[5] = 1;
        b[6..8].copy_from_slice(&self.flags.to_be_bytes());
        b[8..12].copy_from_slice(&self.session.to_be_bytes());
        b[12..16].copy_from_slice(&self.sequence.to_be_bytes());
        b[16..24].copy_from_slice(&self.sample_index.to_be_bytes());
        b[24..32].copy_from_slice(&self.sender_time_us.to_be_bytes());
        b[32..34].copy_from_slice(&self.frame_samples.to_be_bytes());
        b[34..36].copy_from_slice(&self.payload_len.to_be_bytes());
        b[36..40].fill(0);
        Ok(())
    }
    pub fn parse(b: &[u8]) -> Result<Self> {
        ensure!(
            (HEADER..=MAX_DATAGRAM).contains(&b.len()),
            "invalid datagram size"
        );
        ensure!(
            &b[..4] == b"LMIC" && b[4] == 1 && b[5] == 1,
            "invalid protocol"
        );
        let h = Self {
            flags: u16::from_be_bytes(b[6..8].try_into()?),
            session: u32::from_be_bytes(b[8..12].try_into()?),
            sequence: u32::from_be_bytes(b[12..16].try_into()?),
            sample_index: u64::from_be_bytes(b[16..24].try_into()?),
            sender_time_us: u64::from_be_bytes(b[24..32].try_into()?),
            frame_samples: u16::from_be_bytes(b[32..34].try_into()?),
            payload_len: u16::from_be_bytes(b[34..36].try_into()?),
        };
        ensure!(
            h.flags & !31 == 0 && b[36..40] == [0; 4],
            "unknown flags/reserved"
        );
        ensure!([240, 480, 960].contains(&h.frame_samples), "invalid frame");
        ensure!(h.payload_len as usize <= MAX_PAYLOAD, "oversize payload");
        ensure!(h.payload_len > 0 || h.flags & KEEPALIVE != 0, "empty audio");
        ensure!(
            b.len()
                == HEADER + h.payload_len as usize + if h.flags & ENCRYPTED != 0 { 16 } else { 0 },
            "length mismatch"
        );
        Ok(h)
    }
}
pub fn newer(a: u32, b: u32) -> bool {
    (a.wrapping_sub(b) as i32) > 0
}
fn nonce(h: &Header) -> [u8; 12] {
    let mut n = [0; 12];
    n[..4].copy_from_slice(&h.session.to_be_bytes());
    n[4..8].copy_from_slice(&h.sequence.to_be_bytes());
    n
}
pub fn seal(
    key: &[u8; 32],
    h: &Header,
    payload: &[u8],
    out: &mut [u8; MAX_DATAGRAM],
) -> Result<usize> {
    ensure!(
        h.flags & ENCRYPTED != 0
            && payload.len() == h.payload_len as usize
            && payload.len() <= MAX_PAYLOAD,
        "bad seal input"
    );
    h.encode(out)?;
    let len = payload.len();
    out[HEADER..HEADER + len].copy_from_slice(payload);
    let (aad, rest) = out.split_at_mut(HEADER);
    let tag = ChaCha20Poly1305::new(key.into())
        .encrypt_in_place_detached(Nonce::from_slice(&nonce(h)), aad, &mut rest[..len])
        .map_err(|_| anyhow::anyhow!("encrypt failed"))?;
    rest[len..len + 16].copy_from_slice(&tag);
    Ok(HEADER + len + 16)
}
pub fn open<'a>(key: &[u8; 32], b: &'a mut [u8]) -> Result<(Header, &'a [u8])> {
    let h = Header::parse(b)?;
    ensure!(h.flags & ENCRYPTED != 0, "unencrypted packet rejected");
    let (aad, rest) = b.split_at_mut(HEADER);
    let len = h.payload_len as usize;
    let tag = Tag::clone_from_slice(&rest[len..len + 16]);
    ChaCha20Poly1305::new(key.into())
        .decrypt_in_place_detached(Nonce::from_slice(&nonce(&h)), aad, &mut rest[..len], &tag)
        .map_err(|_| anyhow::anyhow!("authentication failed"))?;
    Ok((h, &rest[..len]))
}
pub fn write_json(w: &mut impl Write, v: &serde_json::Value) -> Result<()> {
    let b = serde_json::to_vec(v)?;
    ensure!(b.len() <= 16384, "control message too large");
    w.write_all(&(b.len() as u32).to_be_bytes())?;
    w.write_all(&b)?;
    w.flush()?;
    Ok(())
}
pub fn read_json(r: &mut impl Read) -> Result<serde_json::Value> {
    let mut b = [0; 4];
    r.read_exact(&mut b)?;
    let len = u32::from_be_bytes(b) as usize;
    if len == 0 || len > 16384 {
        bail!("invalid control length")
    };
    let mut b = vec![0; len];
    r.read_exact(&mut b)?;
    Ok(serde_json::from_slice(&b)?)
}
#[derive(Default)]
pub struct Replay {
    highest: Option<u32>,
    bitmap: u64,
}
impl Replay {
    pub fn accept(&mut self, seq: u32) -> bool {
        match self.highest {
            None => {
                self.highest = Some(seq);
                self.bitmap = 1;
                true
            }
            Some(h) if newer(seq, h) => {
                let d = seq.wrapping_sub(h);
                self.bitmap = if d >= 64 { 1 } else { (self.bitmap << d) | 1 };
                self.highest = Some(seq);
                true
            }
            Some(h) => {
                let d = h.wrapping_sub(seq);
                if d >= 64 || self.bitmap & (1 << d) != 0 {
                    false
                } else {
                    self.bitmap |= 1 << d;
                    true
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn h() -> Header {
        Header {
            flags: ENCRYPTED,
            session: 123,
            sequence: 0,
            sample_index: 0,
            sender_time_us: 9,
            frame_samples: 480,
            payload_len: 3,
        }
    }
    #[test]
    fn roundtrip_and_auth() {
        let mut b = [0; 1200];
        let len = seal(&[7; 32], &h(), b"abc", &mut b).unwrap();
        assert_eq!(&b[..6], b"LMIC\x01\x01");
        let orig = b;
        assert_eq!(open(&[7; 32], &mut b[..len]).unwrap().1, b"abc");
        b = orig;
        b[24] ^= 1;
        assert!(open(&[7; 32], &mut b[..len]).is_err());
        assert!(Header::parse(&b[..len - 1]).is_err());
    }
    #[test]
    fn hostile() {
        for n in 0..1300 {
            assert!(Header::parse(&vec![0; n]).is_err())
        }
        let mut b = [0; 1200];
        let n = seal(&[0; 32], &h(), b"abc", &mut b).unwrap();
        b[4] = 2;
        assert!(Header::parse(&b[..n]).is_err());
    }
    #[test]
    fn replay_wrap() {
        let mut r = Replay::default();
        assert!(r.accept(u32::MAX - 1));
        assert!(r.accept(0));
        assert!(r.accept(u32::MAX));
        assert!(!r.accept(0));
        assert!(newer(0, u32::MAX));
        assert!(!newer(u32::MAX, 0));
    }
    #[test]
    fn bounded_json() {
        assert!(read_json(&mut &u32::MAX.to_be_bytes()[..]).is_err());
    }
}

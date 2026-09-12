//! Shared, single-use PAKE for desktop and Android. The code never crosses the wire.
use curve25519_dalek::{edwards::CompressedEdwardsY, traits::IsIdentity};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use spake2::{Ed25519Group, Identity, Password, Spake2};
use zeroize::Zeroizing;
pub const MESSAGE_LEN: usize = 33;
pub struct Exchange(Spake2<Ed25519Group>);
impl Exchange {
    pub fn start(code: &str, server: bool) -> Result<(Self, Vec<u8>), &'static str> {
        if code.len() != 6 || !code.bytes().all(|c| c.is_ascii_digit()) {
            return Err("six digits required");
        }
        let a = Identity::new(b"org.linmic/pairing/spake2-v1/android");
        let b = Identity::new(b"org.linmic/pairing/spake2-v1/desktop");
        let password = Password::new(code.as_bytes());
        let (s, msg) = if server {
            Spake2::start_b(&password, &a, &b)
        } else {
            Spake2::start_a(&password, &a, &b)
        };
        Ok((Self(s), msg))
    }
    pub fn finish(self, message: &[u8]) -> Result<Zeroizing<Vec<u8>>, &'static str> {
        if message.len() != MESSAGE_LEN {
            return Err("invalid PAKE message length");
        }
        let point = CompressedEdwardsY(message[1..].try_into().map_err(|_| "invalid point")?)
            .decompress()
            .ok_or("invalid point")?;
        if point.is_identity() || !point.is_torsion_free() {
            return Err("invalid group element");
        }
        self.0
            .finish(message)
            .map(Zeroizing::new)
            .map_err(|_| "invalid PAKE exchange")
    }
}
pub fn proof(key: &[u8], certificate_sha256: &[u8], server: bool) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts all key lengths");
    mac.update(b"org.linmic/pairing/spake2-v1/tls-certificate-binding\0");
    mac.update(if server { b"server" } else { b"client" });
    mac.update(certificate_sha256);
    mac.finalize().into_bytes().into()
}
pub fn verify(key: &[u8], certificate_sha256: &[u8], server: bool, received: &[u8]) -> bool {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts all key lengths");
    mac.update(b"org.linmic/pairing/spake2-v1/tls-certificate-binding\0");
    mac.update(if server { b"server" } else { b"client" });
    mac.update(certificate_sha256);
    mac.verify_slice(received).is_ok()
}
// The C ABI is private to the JNI wrapper. Buffers have fixed lengths documented in pairing.h.
/// # Safety
/// `code` points to 6 readable bytes and `out` to 33 writable bytes.
#[no_mangle]
pub unsafe extern "C" fn linmic_pair_start(code: *const u8, out: *mut u8) -> *mut Exchange {
    if code.is_null() || out.is_null() {
        return std::ptr::null_mut();
    }
    let text = std::str::from_utf8(unsafe { std::slice::from_raw_parts(code, 6) }).unwrap_or("");
    match Exchange::start(text, false) {
        Ok((s, msg)) => {
            unsafe { std::ptr::copy_nonoverlapping(msg.as_ptr(), out, MESSAGE_LEN) };
            Box::into_raw(Box::new(s))
        }
        Err(_) => std::ptr::null_mut(),
    }
}
/// # Safety
/// `handle` is a live handle returned by start, consumed exactly once. Peer/cert/output
/// point to 33/32/64 readable/readable/writable bytes respectively.
#[no_mangle]
pub unsafe extern "C" fn linmic_pair_finish(
    handle: *mut Exchange,
    peer: *const u8,
    cert: *const u8,
    out: *mut u8,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let state = unsafe { Box::from_raw(handle) };
    if peer.is_null() || cert.is_null() || out.is_null() {
        return -1;
    }
    let peer = unsafe { std::slice::from_raw_parts(peer, 33) };
    let cert = unsafe { std::slice::from_raw_parts(cert, 32) };
    match state.finish(peer) {
        Ok(key) => {
            let client = proof(&key, cert, false);
            let server = proof(&key, cert, true);
            unsafe {
                std::ptr::copy_nonoverlapping(client.as_ptr(), out, 32);
                std::ptr::copy_nonoverlapping(server.as_ptr(), out.add(32), 32)
            };
            0
        }
        Err(_) => -1,
    }
}
/// # Safety
/// The handle is null or a live start result which has not been finished/freed.
#[no_mangle]
pub unsafe extern "C" fn linmic_pair_free(handle: *mut Exchange) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn code_authentication_and_binding() {
        let (a, am) = Exchange::start("123456", false).unwrap();
        let (b, bm) = Exchange::start("123456", true).unwrap();
        let ak = a.finish(&bm).unwrap();
        let bk = b.finish(&am).unwrap();
        assert_eq!(*ak, *bk);
        let p = proof(&ak, &[3; 32], false);
        assert!(verify(&bk, &[3; 32], false, &p));
        assert!(!verify(&bk, &[4; 32], false, &p));
        assert!(!verify(&bk, &[3; 32], true, &p));
    }
    #[test]
    fn wrong_code() {
        let (a, am) = Exchange::start("123456", false).unwrap();
        let (b, bm) = Exchange::start("654321", true).unwrap();
        let ak = a.finish(&bm).unwrap();
        let bk = b.finish(&am).unwrap();
        assert!(!verify(&bk, &[3; 32], false, &proof(&ak, &[3; 32], false)));
    }
    #[test]
    fn reflection() {
        let (a, msg) = Exchange::start("123456", false).unwrap();
        assert!(a.finish(&msg).is_err());
    }
    #[test]
    fn malformed_points() {
        for message in [vec![0; 33], vec![1; 32], vec![255; 33]] {
            let (a, _) = Exchange::start("123456", false).unwrap();
            assert!(a.finish(&message).is_err());
        }
    }
    #[test]
    fn fresh_ephemerals() {
        let (_, a) = Exchange::start("123456", false).unwrap();
        let (_, b) = Exchange::start("123456", false).unwrap();
        assert_ne!(a, b);
        assert!(Exchange::start("123", false).is_err());
    }
}

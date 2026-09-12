use anyhow::Result;
pub use linmic_protocol::{read_json, write_json};
use std::{os::unix::net::UnixStream, time::Duration};
pub fn request(v: serde_json::Value) -> Result<serde_json::Value> {
    let mut s = UnixStream::connect(linmic_common::socket_path()?)?;
    s.set_read_timeout(Some(Duration::from_secs(2)))?;
    s.set_write_timeout(Some(Duration::from_secs(2)))?;
    write_json(&mut s, &v)?;
    read_json(&mut s)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn framing() {
        let (a, mut b) = UnixStream::pair().unwrap();
        let t = std::thread::spawn(move || {
            let mut a = a;
            write_json(&mut a, &serde_json::json!({"type":"status"})).unwrap();
        });
        assert_eq!(read_json(&mut b).unwrap()["type"], "status");
        t.join().unwrap();
    }
}

use anyhow::{ensure, Context, Result};
pub use linmic_protocol::{read_json, write_json};
pub use mdns_sd;
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, ServerName},
    ClientConfig, ClientConnection, RootCertStore, ServerConfig, ServerConnection, StreamOwned,
};
use sha2::{Digest, Sha256};
use std::{fs, net::TcpStream, sync::Arc, time::Duration};
pub struct Identity {
    pub server: Arc<ServerConfig>,
    pub certificate: Vec<u8>,
    pub fingerprint: String,
}
impl Identity {
    pub fn load() -> Result<Self> {
        let dir = linmic_common::data_dir();
        linmic_common::private_dir(&dir)?;
        let cert_path = dir.join("server.der");
        let key_path = dir.join("server-key.der");
        if !cert_path.exists() && !key_path.exists() {
            let key = rcgen::KeyPair::generate()?;
            let mut params =
                rcgen::CertificateParams::new(vec!["linmic.local".into(), "localhost".into()])?;
            params
                .distinguished_name
                .push(rcgen::DnType::CommonName, "LinMic");
            let cert = params.self_signed(&key)?;
            linmic_common::private_write(&key_path, &key.serialize_der())?;
            linmic_common::private_write(&cert_path, cert.der())?;
        }
        let certificate = fs::read(cert_path)?;
        let key = PrivateKeyDer::try_from(fs::read(key_path)?).map_err(anyhow::Error::msg)?;
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let server = ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()?
            .with_no_client_auth()
            .with_single_cert(vec![CertificateDer::from(certificate.clone())], key)?;
        let fingerprint = hex::encode(Sha256::digest(&certificate));
        Ok(Self {
            server: Arc::new(server),
            certificate,
            fingerprint,
        })
    }
    pub fn accept(&self, socket: TcpStream) -> Result<StreamOwned<ServerConnection, TcpStream>> {
        socket.set_read_timeout(Some(Duration::from_secs(5)))?;
        socket.set_write_timeout(Some(Duration::from_secs(5)))?;
        socket.set_nodelay(true)?;
        Ok(StreamOwned::new(
            ServerConnection::new(self.server.clone())?,
            socket,
        ))
    }
}
pub type Client = StreamOwned<ClientConnection, TcpStream>;
pub fn connect(host: &str, certificate: &[u8]) -> Result<Client> {
    let mut roots = RootCertStore::empty();
    roots.add(CertificateDer::from(certificate.to_vec()))?;
    let config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()?
            .with_root_certificates(roots)
            .with_no_client_auth();
    let socket = TcpStream::connect(host)?;
    socket.set_read_timeout(Some(Duration::from_secs(5)))?;
    socket.set_write_timeout(Some(Duration::from_secs(5)))?;
    socket.set_nodelay(true)?;
    Ok(StreamOwned::new(
        ClientConnection::new(Arc::new(config), ServerName::try_from("linmic.local")?)?,
        socket,
    ))
}
pub fn random_hex() -> String {
    use rand::RngCore;
    let mut b = [0; 32];
    rand::rngs::OsRng.fill_bytes(&mut b);
    hex::encode(b)
}
pub fn key(s: &str) -> Result<[u8; 32]> {
    let bytes = hex::decode(s)?;
    ensure!(bytes.len() == 32, "invalid key");
    bytes.try_into().map_err(|_| anyhow::anyhow!("invalid key"))
}
pub fn announce(port: u16, name: &str) -> Result<mdns_sd::ServiceDaemon> {
    let daemon = mdns_sd::ServiceDaemon::new()?;
    daemon.disable_interface(mdns_sd::IfKind::IPv6)?;
    let hostname = format!(
        "{}.local.",
        name.replace(|c: char| !c.is_ascii_alphanumeric(), "-")
    );
    let props = [
        ("version", "1"),
        ("name", name),
        ("requires_pairing", "1"),
        ("tls", "1"),
    ];
    let info =
        mdns_sd::ServiceInfo::new("_linmic._tcp.local.", name, &hostname, "", port, &props[..])
            .context("mDNS service")?
            .enable_addr_auto();
    daemon.register(info)?;
    Ok(daemon)
}

pub fn pair_client(
    control: &mut Client,
    challenge: &serde_json::Value,
    code: &str,
    certificate: &[u8],
) -> Result<()> {
    ensure!(
        challenge["method"] == "spake2-v1",
        "Unsupported pairing method"
    );
    let (state, message) =
        linmic_pairing::Exchange::start(code, false).map_err(anyhow::Error::msg)?;
    let peer = hex::decode(challenge["message"].as_str().context("PAKE challenge")?)?;
    let key = state.finish(&peer).map_err(anyhow::Error::msg)?;
    let fingerprint = Sha256::digest(certificate);
    let proof = linmic_pairing::proof(&key, &fingerprint, false);
    write_json(
        control,
        &serde_json::json!({"type":"pair_submit","method":"spake2-v1","message":hex::encode(message),"proof":hex::encode(proof)}),
    )?;
    let result = read_json(control)?;
    ensure!(result["type"] == "pair_success", "Pairing failed");
    ensure!(
        linmic_pairing::verify(
            &key,
            &fingerprint,
            true,
            &hex::decode(result["proof"].as_str().context("server proof")?)?
        ),
        "Server authentication failed"
    );
    Ok(())
}

/// Default transport is restricted to local/private networks, including USB tethering.
pub fn local_address(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v) => v.is_private() || v.is_loopback() || v.is_link_local(),
        std::net::IpAddr::V6(v) => {
            v.is_loopback()
                || v.is_unique_local()
                || v.is_unicast_link_local()
                || v.to_ipv4_mapped().is_some_and(|v| local_address(v.into()))
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keys() {
        let a = random_hex();
        let b = random_hex();
        assert_ne!(a, b);
        assert!(key(&a).is_ok());
        assert!(key("00").is_err());
    }
}

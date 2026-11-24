//! The machine identity: one RSA-2048 key per machine, generated on first use.
//! The same plan built on a different machine signs differently, and therefore
//! compiles into a different program. It works on *my* machine.

use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding};
use rsa::{Pkcs1v15Sign, RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256, Sha512};

pub const KEY_BITS: usize = 2048;
pub const SCHEME: &str = "RSASSA-PKCS1-v1_5 / RSA-2048 / SHA-512";

fn key_path() -> Result<PathBuf> {
    if let Some(h) = std::env::var_os("WOMM_HOME") {
        return Ok(PathBuf::from(h).join("machine-key.pem"));
    }
    let dir = dirs::config_dir().context("could not find a config directory for the machine key")?;
    Ok(dir.join("womm").join("machine-key.pem"))
}

/// The machine key if this machine already has one. Never generates.
pub fn existing_key() -> Option<RsaPrivateKey> {
    let pem = fs::read_to_string(key_path().ok()?).ok()?;
    RsaPrivateKey::from_pkcs8_pem(&pem).ok()
}

/// Public key bytes (DER), used as a per-machine seed.
pub fn public_der(key: &RsaPrivateKey) -> Vec<u8> {
    key.to_public_key().to_public_key_der().map(|d| d.as_bytes().to_vec()).unwrap_or_default()
}

/// A hash of this machine's hostname. The name itself is never stored.
pub fn host_hash() -> String {
    let name = gethostname::gethostname().to_string_lossy().into_owned();
    hex::encode(&Sha256::digest(name.as_bytes())[..8])
}

pub fn fingerprint(key: &RsaPrivateKey) -> String {
    hex::encode(&Sha256::digest(public_der(key))[..8])
}

pub fn machine_key() -> Result<RsaPrivateKey> {
    let path = key_path()?;
    if path.exists() {
        let pem = fs::read_to_string(&path)?;
        return RsaPrivateKey::from_pkcs8_pem(&pem)
            .with_context(|| format!("machine key at {} is corrupt", path.display()));
    }

    println!("   Generating {KEY_BITS}-bit RSA machine identity (one time only)");
    let key = RsaPrivateKey::new(&mut rand::rngs::OsRng, KEY_BITS)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(&path, key.to_pkcs8_pem(LineEnding::LF)?.as_bytes())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }
    println!("   Machine identity stored at {}", path.display());
    Ok(key)
}

/// Sign a precomputed SHA-512 digest. PKCS#1 v1.5 is deterministic, so the
/// same digest and key always give the same 256-byte signature.
pub fn sign(key: &RsaPrivateKey, digest: &[u8]) -> Result<Vec<u8>> {
    Ok(key.sign(Pkcs1v15Sign::new::<Sha512>(), digest)?)
}

pub fn verify(public_pem: &str, digest: &[u8], signature: &[u8]) -> Result<()> {
    let key = RsaPublicKey::from_public_key_pem(public_pem).context("manifest public key is not valid")?;
    if key.verify(Pkcs1v15Sign::new::<Sha512>(), digest, signature).is_err() {
        bail!("signature does not verify against the manifest's digest and public key");
    }
    Ok(())
}

pub fn public_pem(key: &RsaPrivateKey) -> Result<String> {
    Ok(key.to_public_key().to_public_key_pem(LineEnding::LF)?)
}

pub fn fingerprint_of_pem(public_pem: &str) -> Result<String> {
    let key = RsaPublicKey::from_public_key_pem(public_pem)?;
    let der = key.to_public_key_der()?;
    Ok(hex::encode(&Sha256::digest(der.as_bytes())[..8]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_is_deterministic_and_512_hex_chars() {
        let key = RsaPrivateKey::new(&mut rand::rngs::OsRng, KEY_BITS).unwrap();
        let digest = Sha512::digest(b"kludge");
        let a = sign(&key, &digest).unwrap();
        let b = sign(&key, &digest).unwrap();
        assert_eq!(a, b);
        assert_eq!(hex::encode(&a).len(), 512);
        verify(&public_pem(&key).unwrap(), &digest, &a).unwrap();
    }
}

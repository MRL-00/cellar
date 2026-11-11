#[cfg(test)]
#[path = "known_hosts_tests.rs"]
mod tests;

use std::fs::{DirBuilder, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use cellar_core::error::{CellarError, CellarResult, SshHostKey};
use russh::keys::{
    self,
    ssh_key::{HashAlg, PublicKey},
};

fn challenge(host: &str, port: u16, key: &PublicKey) -> CellarResult<SshHostKey> {
    Ok(SshHostKey {
        host: host.to_owned(),
        port,
        algorithm: key.algorithm().to_string(),
        fingerprint: key.fingerprint(HashAlg::Sha256).to_string(),
        public_key: key
            .to_openssh()
            .map_err(|_| CellarError::Authentication("cannot encode SSH host key".into()))?,
    })
}

/// Persist only after the user has verified this fingerprint through a trusted
/// channel. This does not connect or bypass verification on the next attempt.
pub fn trust_host_key(key: &SshHostKey) -> CellarResult<()> {
    let path = dirs::home_dir()
        .ok_or_else(|| CellarError::invalid_config("cannot locate SSH known_hosts"))?
        .join(".ssh/known_hosts");
    trust_host_key_at(key, &path)
}

pub(crate) fn trust_host_key_at(key: &SshHostKey, path: &Path) -> CellarResult<()> {
    // Restrict known_hosts syntax, not just line breaks: aliases, markers,
    // wildcards and hashed patterns must never be injected via confirmation.
    if key.port == 0
        || key.host.is_empty()
        || !key
            .host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_:%".contains(&b))
    {
        return Err(CellarError::invalid_config(
            "invalid SSH trust host or port",
        ));
    }
    // ssh-key preserves comments, including embedded newlines. Reject control
    // characters before any filesystem effects and never persist comments.
    if key.public_key.chars().any(char::is_control) {
        return Err(CellarError::invalid_config("invalid SSH public host key"));
    }
    let mut public = PublicKey::from_openssh(&key.public_key)
        .map_err(|_| CellarError::invalid_config("invalid SSH public host key"))?;
    public.set_comment("");
    if key.algorithm != public.algorithm().to_string()
        || key.fingerprint != public.fingerprint(HashAlg::Sha256).to_string()
    {
        return Err(CellarError::invalid_config(
            "SSH host key algorithm or fingerprint mismatch",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| CellarError::invalid_config("invalid known_hosts path"))?;
    let mut directory = DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directory.mode(0o700);
    }
    match directory.create(parent) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error.into()),
    }
    let mut options = OpenOptions::new();
    options.read(true).append(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    // Advisory, OS-backed lock: concurrent Cellar confirmations serialize on
    // this file. Never truncate/replace it (which would invalidate inode locks).
    // External known_hosts editors must cooperate with the lock as well.
    file.lock()?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    match verify_snapshot(&key.host, key.port, &public, path, &text) {
        Ok(()) => return Ok(()),
        Err(CellarError::SshHostKeyUnknown(_)) => {}
        Err(error) => return Err(error),
    }
    let host = if key.port == 22 {
        key.host.clone()
    } else {
        format!("[{}]:{}", key.host, key.port)
    };
    let public = public
        .to_openssh()
        .map_err(|_| CellarError::invalid_config("invalid SSH public host key"))?;
    let prefix = if text.is_empty() || text.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    file.write_all(format!("{prefix}{host} {public}\n").as_bytes())?;
    file.sync_all()?;
    Ok(())
}

pub(crate) fn verify_host_key(
    host: &str,
    port: u16,
    key: &keys::ssh_key::PublicKey,
    path: &Path,
) -> CellarResult<()> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    verify_snapshot(host, port, key, path, &text)
}

fn verify_snapshot(
    host: &str,
    port: u16,
    key: &PublicKey,
    path: &Path,
    text: &str,
) -> CellarResult<()> {
    // Revocation is global: aliases and ordinary duplicate entries cannot
    // resurrect a revoked key. Malformed revocations fail closed.
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        if fields.next() != Some("@revoked") {
            continue;
        }
        let _host = fields.next();
        let algorithm = fields.next().unwrap_or_default();
        let data = fields.next().unwrap_or_default();
        let revoked = keys::ssh_key::PublicKey::from_openssh(&format!("{algorithm} {data}"))
            .map_err(|_| {
                CellarError::Authentication("invalid revoked SSH host key entry".into())
            })?;
        if revoked.key_data() == key.key_data() {
            return Err(CellarError::Authentication(
                "SSH host key is revoked".into(),
            ));
        }
    }
    // Parse the already-read snapshot so whitespace handling and revocation
    // checks cannot disagree, or race a second read of known_hosts.
    let host_port = if port == 22 {
        host.to_owned()
    } else {
        format!("[{host}]:{port}")
    };
    let mut trusted = false;
    let mut recorded_host = false;
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(patterns) = fields.next() else {
            continue;
        };
        if patterns.starts_with(['#', '@']) {
            continue;
        }
        if !matches_host_patterns(&host_port, patterns) {
            continue;
        }
        let algorithm = fields.next().unwrap_or_default();
        let data = fields.next().unwrap_or_default();
        let recorded = keys::ssh_key::PublicKey::from_openssh(&format!("{algorithm} {data}"))
            .map_err(|_| CellarError::Authentication("invalid SSH known_hosts key entry".into()))?;
        recorded_host = true;
        if recorded.algorithm() == key.algorithm() {
            if recorded.key_data() != key.key_data() {
                return Err(changed_key(host, port, path));
            }
            trusted = true;
        }
    }
    if trusted {
        Ok(())
    } else if recorded_host {
        // A new algorithm is not a new host: allowing a challenge here would
        // let an attacker bypass existing pins by changing key type. Multiple
        // explicitly provisioned algorithms remain valid when one matches.
        Err(changed_key(host, port, path))
    } else {
        Err(CellarError::SshHostKeyUnknown(challenge(host, port, key)?))
    }
}

fn changed_key(host: &str, port: u16, path: &Path) -> CellarError {
    CellarError::Authentication(format!(
        "SSH host key changed for {host}:{port}; check {}",
        path.display()
    ))
}

fn matches_host_patterns(host: &str, patterns: &str) -> bool {
    let mut matched = false;
    for pattern in patterns.split(',') {
        if let Some(excluded) = pattern.strip_prefix('!') {
            if matches_known_host(host, excluded) {
                return false;
            }
        } else if matches_known_host(host, pattern) {
            matched = true;
        }
    }
    matched
}

fn matches_known_host(host: &str, pattern: &str) -> bool {
    use hmac::Mac;
    use keys::ssh_key::known_hosts::HostPatterns;

    if !pattern.starts_with("|1|") {
        return matches_plain_host(host.as_bytes(), pattern.as_bytes());
    }
    let Ok(HostPatterns::HashedName { salt, hash }) = pattern.parse() else {
        return false;
    };
    let Ok(hmac) = hmac::Hmac::<sha1::Sha1>::new_from_slice(&salt) else {
        return false;
    };
    // ssh-keygen normally hashes canonical lowercase DNS names. Keep exact
    // matching for existing entries, but do not let configuration casing
    // bypass a canonical hashed pin.
    hmac.clone()
        .chain_update(host.as_bytes())
        .verify_slice(&hash)
        .is_ok()
        || hmac
            .chain_update(host.to_ascii_lowercase().as_bytes())
            .verify_slice(&hash)
            .is_ok()
}

// OpenSSH host patterns use only '*' and '?'; brackets in [host]:port are
// literal, not glob character classes. DNS names compare without ASCII case.
fn matches_plain_host(host: &[u8], pattern: &[u8]) -> bool {
    let (mut h, mut p) = (0, 0);
    let mut star = None;
    while h < host.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p].eq_ignore_ascii_case(&host[h])) {
            h += 1;
            p += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star = Some((p, h));
            p += 1;
        } else if let Some((position, consumed)) = star {
            h = consumed + 1;
            p = position + 1;
            star = Some((position, h));
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
}

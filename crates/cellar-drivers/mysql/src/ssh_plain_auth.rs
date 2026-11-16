//! Startup authentication relay. TLS-backed authentication is relayed unchanged,
//! including mysql_clear_password and other plugins the bridge does not know.
//! Preferred's remote-noTLS fallback converts SHA passwords to MySQL RSA-OAEP/SHA1
//! and rejects every other plugin, so no password is sent in cleartext.
//! The local leg always validates the ephemeral loopback TLS identity.
//! Independent sequence counters account for the local-only SSLRequest and
//! remote-only public-key exchanges. After authentication, commands reset the
//! sequence to zero and streams can be copied without inspecting result sets.
use super::{io_error, protocol_error, read_packet};
use cellar_core::error::{CellarError, CellarResult};
use cellar_ssh::TunnelStream;
use rsa::{pkcs8::DecodePublicKey, Oaep, RsaPublicKey};
use tokio::io::AsyncWriteExt;

#[derive(Clone, Copy, PartialEq)]
enum Plugin {
    Native,
    CachingSha2,
    Sha256,
    /// mysql_clear_password or any plugin the plaintext fallback cannot bridge.
    Other,
}

impl Plugin {
    fn parse(name: &[u8]) -> Self {
        match name {
            b"mysql_native_password" => Self::Native,
            b"caching_sha2_password" => Self::CachingSha2,
            b"sha256_password" => Self::Sha256,
            _ => Self::Other,
        }
    }

    /// A relayed plugin may send the password as-is; only TLS may carry it.
    fn check(self, remote_tls: bool) -> CellarResult<Self> {
        if self == Self::Other && !remote_tls {
            return Err(CellarError::Authentication(
                "MySQL authentication plugin requires TLS to the server through the SSH tunnel"
                    .into(),
            ));
        }
        Ok(self)
    }
}

pub(super) struct Handshake {
    plugin: Plugin,
    nonce: Vec<u8>,
}

impl Handshake {
    pub(super) fn parse(packet: &[u8], flags: usize) -> CellarResult<Self> {
        let capabilities = u16::from_le_bytes([packet[flags + 5], packet[flags + 6]]);
        // Modern plugin authentication + secure connection are necessary to
        // locate the challenge unambiguously. Do not guess legacy formats.
        if capabilities & 0x08 == 0 || packet[flags + 1] & 0x80 == 0 {
            return Err(protocol_error());
        }
        let second_len = usize::from(packet[flags + 7].saturating_sub(8)).max(13);
        let start = flags + 18;
        let second = packet
            .get(start..start + second_len)
            .ok_or_else(protocol_error)?;
        let name = packet
            .get(start + second_len..)
            .ok_or_else(protocol_error)?;
        let end = name
            .iter()
            .position(|b| *b == 0)
            .ok_or_else(protocol_error)?;
        let mut nonce = packet[flags - 9..flags - 1].to_vec();
        nonce.extend_from_slice(second.strip_suffix(&[0]).unwrap_or(second));
        if nonce.len() != 20 {
            return Err(protocol_error());
        }
        Ok(Self {
            plugin: Plugin::parse(&name[..end]),
            nonce,
        })
    }

    /// Reject before forwarding the greeting so the failure is reported as
    /// an authentication error rather than SQLx closing the socket.
    pub(super) fn check(&self, remote_tls: bool) -> CellarResult<()> {
        self.plugin.check(remote_tls).map(drop)
    }

    /// Parse an AuthSwitchRequest, rejecting it before SQLx sees it if the
    /// requested plugin cannot be carried safely over the remote leg.
    fn switch(response: &[u8], remote_tls: bool) -> CellarResult<Self> {
        // 0xfe, NUL-terminated plugin name, plugin data (optionally NUL-ended).
        // Checked accessors only: a truncated packet is a protocol error.
        let (_, body) = response.split_first().ok_or_else(protocol_error)?;
        let name_end = body
            .iter()
            .position(|b| *b == 0)
            .ok_or_else(protocol_error)?;
        let (name, rest) = body.split_at(name_end);
        let plugin = Plugin::parse(name).check(remote_tls)?;
        let (_, data) = rest.split_first().ok_or_else(protocol_error)?;
        let nonce = data.strip_suffix(&[0]).unwrap_or(data).to_vec();
        // Relayed plugins may carry no challenge (AWS Aurora clear password).
        if plugin != Plugin::Other && nonce.len() != 20 {
            return Err(protocol_error());
        }
        Ok(Self { plugin, nonce })
    }
}

struct Sequence(u8);
impl Sequence {
    async fn read<S: TunnelStream + ?Sized>(&mut self, stream: &mut S) -> CellarResult<Vec<u8>> {
        let packet = read_packet(stream).await?;
        if packet[3] != self.0 || packet.len() == 4 {
            return Err(protocol_error());
        }
        self.0 = self.0.wrapping_add(1);
        Ok(packet[4..].to_vec())
    }
    async fn write<S: TunnelStream + ?Sized>(
        &mut self,
        stream: &mut S,
        payload: &[u8],
    ) -> CellarResult<()> {
        // Classic protocol packet length is a 24-bit integer. Authentication
        // messages are single packets; never truncate or aggregate continuations.
        if payload.len() >= 0xff_ffff {
            return Err(protocol_error());
        }
        let size = (payload.len() as u32).to_le_bytes();
        stream
            .write_all(&[size[0], size[1], size[2], self.0])
            .await
            .map_err(io_error)?;
        stream.write_all(payload).await.map_err(io_error)?;
        stream.flush().await.map_err(io_error)?;
        self.0 = self.0.wrapping_add(1);
        Ok(())
    }
}

pub(super) async fn authenticate<L: TunnelStream, R: TunnelStream + ?Sized>(
    local: &mut L,
    remote: &mut R,
    mut handshake: Handshake,
    remote_tls: bool,
) -> CellarResult<()> {
    handshake.plugin.check(remote_tls)?;
    let mut ls = Sequence(2);
    let mut rs = Sequence(if remote_tls { 2 } else { 1 });
    let mut login = ls.read(local).await?;
    if login.len() < 32 {
        return Err(protocol_error());
    }
    if !remote_tls {
        // The remote never saw an SSLRequest, so clear CLIENT_SSL on its login.
        login[1] &= !0x08;
    }
    let (_, auth, _) = login_auth(&login)?;
    // With no configured password SQLx emits an empty initial response, not
    // a NUL-terminated password. Preserve it without requesting a public key.
    if handshake.plugin == Plugin::Sha256 && !remote_tls && !auth.is_empty() {
        let (prefix, auth, suffix) = login_auth(&login)?;
        let password = auth.to_vec();
        let mut request = login[..prefix].to_vec();
        request.extend_from_slice(&[1, 1]); // auth length 1, public-key request 1
        request.extend_from_slice(&login[suffix..]);
        rs.write(remote, &request).await?;
        send_encrypted(remote, &mut rs, &password, &handshake.nonce).await?;
    } else {
        rs.write(remote, &login).await?;
    }
    loop {
        let response = rs.read(remote).await?;
        let kind = *response.first().ok_or_else(protocol_error)?;
        if kind == 0xfe {
            // Validate switches before asking SQLx for any new credentials.
            handshake = Handshake::switch(&response, remote_tls)?;
        }
        ls.write(local, &response).await?;
        let fast_auth_ok = handshake.plugin == Plugin::CachingSha2 && response.as_slice() == [1, 3];
        match kind {
            0x00 | 0xff => return Ok(()),
            // Fast-auth success: the server sends OK next, the client is silent.
            0x01 if fast_auth_ok => {}
            // Both legs are TLS, so each request gets exactly one client answer,
            // relayed unchanged whatever the plugin.
            0x01 | 0xfe if remote_tls => {
                let answer = ls.read(local).await?;
                rs.write(remote, &answer).await?;
            }
            0xfe => {
                let answer = ls.read(local).await?;
                if handshake.plugin == Plugin::Sha256 {
                    rs.write(remote, &[1]).await?;
                    send_encrypted(remote, &mut rs, &answer, &handshake.nonce).await?;
                } else {
                    rs.write(remote, &answer).await?;
                }
            }
            0x01 if handshake.plugin == Plugin::CachingSha2 && response.as_slice() == [1, 4] => {
                let password = ls.read(local).await?;
                rs.write(remote, &[2]).await?;
                send_encrypted(remote, &mut rs, &password, &handshake.nonce).await?;
            }
            _ => return Err(protocol_error()),
        }
    }
}

// Return auth length field offset, auth payload, and the remaining login offset.
// SQLx uses either one-byte secure-connection length or length-encoded length.
fn login_auth(login: &[u8]) -> CellarResult<(usize, &[u8], usize)> {
    let user_end = login
        .get(32..)
        .ok_or_else(protocol_error)?
        .iter()
        .position(|b| *b == 0)
        .ok_or_else(protocol_error)?
        + 32;
    let prefix = user_end + 1;
    let marker = *login.get(prefix).ok_or_else(protocol_error)?;
    let length_encoded = login[2] & 0x20 != 0;
    let (length, width) = match (length_encoded, marker) {
        (true, 0xfc) => (le_length(login, prefix + 1, 2)?, 3),
        (true, 0xfd) => (le_length(login, prefix + 1, 3)?, 4),
        (true, 0xfe) => (le_length(login, prefix + 1, 8)?, 9),
        (true, 0xfb | 0xff) => return Err(protocol_error()),
        _ => (usize::from(marker), 1),
    };
    let end = prefix
        .checked_add(width)
        .and_then(|v| v.checked_add(length))
        .ok_or_else(protocol_error)?;
    let auth = login.get(prefix + width..end).ok_or_else(protocol_error)?;
    Ok((prefix, auth, end))
}

fn le_length(packet: &[u8], offset: usize, width: usize) -> CellarResult<usize> {
    let bytes = packet
        .get(offset..offset + width)
        .ok_or_else(protocol_error)?;
    let mut value = [0; 8];
    value[..width].copy_from_slice(bytes);
    usize::try_from(u64::from_le_bytes(value)).map_err(|_| protocol_error())
}

async fn send_encrypted<R: TunnelStream + ?Sized>(
    remote: &mut R,
    sequence: &mut Sequence,
    password: &[u8],
    nonce: &[u8],
) -> CellarResult<()> {
    // This is MySQL's documented password XOR + RSA-OAEP/SHA1 scheme, matching
    // sqlx-mysql::connection::auth::encrypt_rsa. Crypto is provided by rsa.
    if password.last() != Some(&0)
        || password[..password.len() - 1].contains(&0)
        || nonce.len() != 20
    {
        return Err(protocol_error());
    }
    let response = sequence.read(remote).await?;
    let Some((&1, pem)) = response.split_first() else {
        return Err(protocol_error());
    };
    let pem = std::str::from_utf8(pem).map_err(|_| protocol_error())?;
    let key = RsaPublicKey::from_public_key_pem(pem).map_err(|_| protocol_error())?;
    let encoded: Vec<_> = password
        .iter()
        .enumerate()
        .map(|(i, byte)| byte ^ nonce[i % nonce.len()])
        .collect();
    let encrypted = key
        .encrypt(&mut rand::thread_rng(), Oaep::new::<sha1::Sha1>(), &encoded)
        .map_err(|_| CellarError::Authentication("MySQL RSA password encryption failed".into()))?;
    sequence.write(remote, &encrypted).await
}

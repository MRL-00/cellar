use super::*;
use russh::keys::ssh_key::{private::Ed25519Keypair, Algorithm, EcdsaCurve, PrivateKey};

fn public(seed: u8) -> PublicKey {
    PrivateKey::from(Ed25519Keypair::from_seed(&[seed; 32]))
        .public_key()
        .clone()
}

fn unknown(path: &Path, host: &str, port: u16, public: &PublicKey) -> SshHostKey {
    match verify_host_key(host, port, public, path).unwrap_err() {
        CellarError::SshHostKeyUnknown(key) => key,
        error => panic!("expected typed host challenge, got {error}"),
    }
}

#[test]
fn missing_file_challenges_and_confirmation_creates_secure_idempotent_trust() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(".ssh/known_hosts");
    let public = public(1);
    let key = unknown(&path, "db", 2222, &public);
    assert_eq!(key.host, "db");
    assert_eq!(key.port, 2222);
    assert_eq!(key.algorithm, "ssh-ed25519");
    assert_eq!(
        key.fingerprint,
        public.fingerprint(HashAlg::Sha256).to_string()
    );
    assert!(key.fingerprint.starts_with("SHA256:"));
    assert_eq!(key.public_key, public.to_openssh().unwrap());
    trust_host_key_at(&key, &path).unwrap();
    let content = std::fs::read(&path).unwrap();
    assert_eq!(
        content,
        format!("[db]:2222 {}\n", key.public_key).as_bytes()
    );
    trust_host_key_at(&key, &path).unwrap();
    assert_eq!(content, std::fs::read(&path).unwrap());
    verify_host_key("db", 2222, &public, &path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(path.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
}

#[test]
fn append_preserves_exact_content_and_handles_absent_terminal_newline() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("known_hosts");
    for prefix in ["# existing\r\n\t\n", "# no newline"] {
        std::fs::write(&path, prefix).unwrap();
        let key = unknown(&path, "::1", 22, &public(1));
        trust_host_key_at(&key, &path).unwrap();
        let separator = if prefix.ends_with('\n') { "" } else { "\n" };
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            format!("{prefix}{separator}::1 {}\n", key.public_key)
        );
    }
}

#[test]
fn confirmation_rechecks_changed_revoked_and_different_algorithm_pins() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("known_hosts");
    let public = public(1);
    let key = unknown(&path, "db", 22, &public);
    let different = PrivateKey::random(
        &mut rand::rng(),
        Algorithm::Ecdsa {
            curve: EcdsaCurve::NistP256,
        },
    )
    .unwrap();
    for entry in [
        format!("db {}\n", self::public(2).to_openssh().unwrap()),
        format!("@revoked alias {}\n", key.public_key),
        format!("db {}\n", different.public_key().to_openssh().unwrap()),
    ] {
        std::fs::write(&path, &entry).unwrap();
        assert!(matches!(
            verify_host_key("db", 22, &public, &path),
            Err(CellarError::Authentication(_))
        ));
        assert!(matches!(
            trust_host_key_at(&key, &path),
            Err(CellarError::Authentication(_))
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), entry);
    }
    // Existing, explicitly provisioned multiple algorithms are not new trust.
    let entries = format!(
        "db {}\ndb {}\n",
        different.public_key().to_openssh().unwrap(),
        key.public_key
    );
    std::fs::write(&path, &entries).unwrap();
    verify_host_key("db", 22, &public, &path).unwrap();
    trust_host_key_at(&key, &path).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), entries);
}

#[test]
fn host_patterns_pin_wildcards_case_and_ports_without_overriding_negation() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("known_hosts");
    let pinned = public(1);
    let changed = public(2);
    for (patterns, host, port) in [
        ("*.example.com", "db.example.com", 22),
        ("db.example.com", "DB.EXAMPLE.COM", 22),
        ("DB.EXAMPLE.COM", "db.example.com", 22),
        ("db?.example.com", "db1.example.com", 22),
        ("db*.example.com", "db.example.com", 22),
        ("*.example.com,!excluded.example.com", "DB.EXAMPLE.COM", 22),
        ("[*.example.com]:2222", "DB.EXAMPLE.COM", 2222),
    ] {
        let entry = format!("{patterns} {}\n", pinned.to_openssh().unwrap());
        std::fs::write(&path, &entry).unwrap();
        verify_host_key(host, port, &pinned, &path).unwrap();
        assert!(
            matches!(
                verify_host_key(host, port, &changed, &path),
                Err(CellarError::Authentication(_))
            ),
            "pattern {patterns}"
        );
        let confirmation = challenge(host, port, &changed).unwrap();
        assert!(
            matches!(
                trust_host_key_at(&confirmation, &path),
                Err(CellarError::Authentication(_))
            ),
            "pattern {patterns}"
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), entry);
    }
    for (patterns, host, port) in [
        (
            "*.example.com,!excluded.example.com",
            "EXCLUDED.EXAMPLE.COM",
            22,
        ),
        (
            "!excluded.example.com,*.example.com",
            "excluded.example.com",
            22,
        ),
        ("*.example.com,!db?.example.com", "db1.example.com", 22),
        ("!*.example.com", "db.example.com", 22),
        ("db?.example.com", "db.example.com", 22),
        ("[*.example.com]:2222", "db.example.com", 22),
        ("*.example.com", "db.other.com", 22),
    ] {
        std::fs::write(
            &path,
            format!("{patterns} {}\n", pinned.to_openssh().unwrap()),
        )
        .unwrap();
        let _challenge = unknown(&path, host, port, &changed);
    }
}

#[test]
fn canonical_hashed_pin_protects_uppercase_configured_hostname() {
    use hmac::Mac;
    use keys::ssh_key::known_hosts::HostPatterns;

    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("known_hosts");
    let pinned = public(1);
    let changed = public(2);
    for (canonical, port) in [("db.example.com", 22), ("[db.example.com]:2222", 2222)] {
        let salt = b"canonical-host-test-salt".to_vec();
        let hash = hmac::Hmac::<sha1::Sha1>::new_from_slice(&salt)
            .unwrap()
            .chain_update(canonical.as_bytes())
            .finalize()
            .into_bytes()
            .into();
        let pattern = HostPatterns::HashedName { salt, hash }.to_string();
        let entry = format!("{pattern} {}\n", pinned.to_openssh().unwrap());
        std::fs::write(&path, &entry).unwrap();
        verify_host_key("DB.EXAMPLE.COM", port, &pinned, &path).unwrap();
        assert!(matches!(
            verify_host_key("DB.EXAMPLE.COM", port, &changed, &path),
            Err(CellarError::Authentication(_))
        ));
        let confirmation = challenge("DB.EXAMPLE.COM", port, &changed).unwrap();
        assert!(matches!(
            trust_host_key_at(&confirmation, &path),
            Err(CellarError::Authentication(_))
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), entry);
    }
}

#[test]
fn public_key_comment_cannot_inject_an_unconfirmed_pin() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("known_hosts");
    let original = "# existing content without terminal newline";
    std::fs::write(&path, original).unwrap();
    let key = unknown(&path, "db.example.com", 22, &public(1));
    let other = public(2).to_openssh().unwrap();
    for separator in ["\n", "\r\n", "\r", "\0"] {
        let mut injected = key.clone();
        injected.public_key = format!("{} comment{separator}other-host {other}", key.public_key);
        assert!(matches!(
            trust_host_key_at(&injected, &path),
            Err(CellarError::InvalidConfig(_))
        ));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    }
    // Ordinary comments may be supplied but must not survive serialization.
    let mut commented = key.clone();
    commented.public_key.push_str(" human comment");
    trust_host_key_at(&commented, &path).unwrap();
    let expected = format!("{original}\ndb.example.com {}\n", key.public_key);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), expected);
    let _challenge = unknown(&path, "other-host", 22, &public(2));
}

#[test]
fn concurrent_confirmations_serialize_and_never_replace_a_conflicting_pin() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("known_hosts");
    let first = unknown(&path, "db", 22, &public(1));
    let second = unknown(&path, "db", 22, &public(2));
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            barrier.wait();
            trust_host_key_at(&first, &path)
        });
        let b = scope.spawn(|| {
            barrier.wait();
            trust_host_key_at(&second, &path)
        });
        [a.join().unwrap(), b.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert!(results
        .iter()
        .any(|r| matches!(r, Err(CellarError::Authentication(_)))));
    assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 1);
    let winner = if results[0].is_ok() { first } else { second };
    std::thread::scope(|scope| {
        let a = scope.spawn(|| trust_host_key_at(&winner, &path));
        let b = scope.spawn(|| trust_host_key_at(&winner, &path));
        a.join().unwrap().unwrap();
        b.join().unwrap().unwrap();
    });
    assert_eq!(std::fs::read_to_string(&path).unwrap().lines().count(), 1);
}

#[test]
fn confirmation_rejects_incoherent_or_injectable_payload_before_writing() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(".ssh/known_hosts");
    let key = unknown(&path, "db", 22, &public(1));
    let mut bad = key.clone();
    bad.fingerprint.push('x');
    let mut algorithm = key.clone();
    algorithm.algorithm = "ssh-rsa".into();
    let mut invalid = key.clone();
    invalid.public_key = "not a public key".into();
    let mut port = key.clone();
    port.port = 0;
    for key in [bad, algorithm, invalid, port] {
        assert!(matches!(
            trust_host_key_at(&key, &path),
            Err(CellarError::InvalidConfig(_))
        ));
    }
    for host in [
        "",
        "db\nother",
        "db other",
        "db,alias",
        "*.example",
        "!db",
        "|1|abc",
        "@revoked",
        "[db]:22",
    ] {
        let mut key = key.clone();
        key.host = host.into();
        assert!(matches!(
            trust_host_key_at(&key, &path),
            Err(CellarError::InvalidConfig(_))
        ));
    }
    assert!(!path.exists());
    // Non-missing read failures must never become trust challenges.
    assert!(!matches!(
        verify_host_key("db", 22, &public(1), temp.path()),
        Err(CellarError::SshHostKeyUnknown(_))
    ));
}

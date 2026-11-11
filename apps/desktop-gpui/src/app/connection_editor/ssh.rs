use cellar_core::driver::{SshAuthMethod, SshHostConfig, SshTunnelConfig};
use cellar_runtime::ConnectionSecrets;
use gpui::{AppContext, Context, Entity, Window};
use gpui_component::input::InputState;

use super::{optional_text, text, CellarApp, ConnectionEditor};

const PASSWORD_PLACEHOLDER: &str = "Blank keeps the saved password";
const PASSPHRASE_PLACEHOLDER: &str = "Optional; blank keeps the saved passphrase";
const CLEARED_PLACEHOLDER: &str = "Saved value will be cleared on save";

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SshSecretField {
    Password,
    Passphrase,
}

pub(crate) struct SshHostEditor {
    pub(crate) host: Entity<InputState>,
    pub(crate) port: Entity<InputState>,
    pub(crate) user: Entity<InputState>,
    pub(crate) auth_method: SshAuthMethod,
    pub(crate) private_key_path: Entity<InputState>,
    pub(crate) password: Entity<InputState>,
    pub(crate) passphrase: Entity<InputState>,
    /// Auth method of this hop in the saved connection. A keychain value can
    /// only exist for it, so the clear controls are offered only while the
    /// editor still uses that method.
    pub(crate) saved_auth_method: Option<SshAuthMethod>,
    pub(crate) clear_password: bool,
    pub(crate) clear_passphrase: bool,
}

impl SshHostEditor {
    pub(crate) fn new(
        config: Option<&SshHostConfig>,
        window: &mut Window,
        cx: &mut Context<CellarApp>,
    ) -> Self {
        let mut input = |value: String,
                         placeholder: &'static str,
                         masked: bool,
                         cx: &mut Context<CellarApp>| {
            cx.new(|cx| {
                InputState::new(window, cx)
                    .default_value(value)
                    .placeholder(placeholder)
                    .masked(masked)
            })
        };
        Self {
            host: input(
                config.map(|c| c.host.clone()).unwrap_or_default(),
                "ssh.example.com",
                false,
                cx,
            ),
            port: input(
                config.map(|c| c.port).unwrap_or(22).to_string(),
                "22",
                false,
                cx,
            ),
            user: input(
                config.map(|c| c.user.clone()).unwrap_or_default(),
                "SSH user",
                false,
                cx,
            ),
            auth_method: config
                .map(|c| c.auth_method)
                .unwrap_or(SshAuthMethod::Agent),
            private_key_path: input(
                config
                    .and_then(|c| c.private_key_path.clone())
                    .unwrap_or_default(),
                "~/.ssh/id_ed25519",
                false,
                cx,
            ),
            password: input(String::new(), PASSWORD_PLACEHOLDER, true, cx),
            passphrase: input(String::new(), PASSPHRASE_PLACEHOLDER, true, cx),
            saved_auth_method: config.map(|c| c.auth_method),
            clear_password: false,
            clear_passphrase: false,
        }
    }

    /// Whether the clear control for `field` applies to a saved keychain value.
    pub(crate) fn can_clear(&self, field: SshSecretField) -> bool {
        self.saved_auth_method == Some(self.auth_method) && field.auth_method() == self.auth_method
    }

    pub(crate) fn clear_requested(&self, field: SshSecretField) -> bool {
        match field {
            SshSecretField::Password => self.clear_password,
            SshSecretField::Passphrase => self.clear_passphrase,
        }
    }

    /// A duplicate gets a new ID without saved SSH secrets.
    pub(crate) fn forget_saved_secrets(&mut self) {
        self.saved_auth_method = None;
        self.clear_password = false;
        self.clear_passphrase = false;
    }

    fn set_clear(
        &mut self,
        field: SshSecretField,
        clear: bool,
        window: &mut Window,
        cx: &mut Context<CellarApp>,
    ) {
        let (flag, state, placeholder) = match field {
            SshSecretField::Password => (
                &mut self.clear_password,
                &self.password,
                PASSWORD_PLACEHOLDER,
            ),
            SshSecretField::Passphrase => (
                &mut self.clear_passphrase,
                &self.passphrase,
                PASSPHRASE_PLACEHOLDER,
            ),
        };
        *flag = clear;
        state.update(cx, |state, cx| {
            if clear {
                state.set_value("", window, cx);
            }
            state.set_placeholder(
                if clear {
                    CLEARED_PLACEHOLDER
                } else {
                    placeholder
                },
                window,
                cx,
            );
        });
    }

    fn hop_secrets(&self, active: bool, cx: &Context<CellarApp>) -> HopSecrets {
        hop_secrets(
            active,
            self.auth_method,
            SecretInput {
                value: &text_value(&self.password, cx),
                clear: self.clear_password,
            },
            SecretInput {
                value: &text_value(&self.passphrase, cx),
                clear: self.clear_passphrase,
            },
        )
    }

    fn config(&self, cx: &Context<CellarApp>) -> Result<SshHostConfig, String> {
        let host = text(&self.host, cx);
        let user = text(&self.user, cx);
        if host.is_empty() || user.is_empty() {
            return Err("SSH host and user are required for each enabled hop".into());
        }
        let port = text(&self.port, cx)
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
            .ok_or("SSH port must be between 1 and 65535")?;
        let private_key_path = optional_text(&self.private_key_path, cx);
        if self.auth_method == SshAuthMethod::PrivateKey && private_key_path.is_none() {
            return Err("Choose a private key path for key authentication".into());
        }
        Ok(SshHostConfig {
            host,
            port,
            user,
            auth_method: self.auth_method,
            private_key_path,
        })
    }
}

impl ConnectionEditor {
    pub(crate) fn ssh_config(
        &self,
        cx: &Context<CellarApp>,
    ) -> Result<Option<SshTunnelConfig>, String> {
        if !self.ssh {
            return Ok(None);
        }
        if !self.engine.supports_ssh() {
            return Err(
                "SSH tunneling requires a PostgreSQL, MySQL, or SQL Server connection".into(),
            );
        }
        Ok(Some(SshTunnelConfig {
            server: self.ssh_server.config(cx)?,
            jump_host: self
                .ssh_jump_enabled
                .then(|| self.ssh_jump.config(cx))
                .transpose()?,
        }))
    }

    /// SSH secrets follow the runtime save contract: `None` keeps the saved
    /// value, `Some("")` clears it, and any other value replaces it. Values
    /// typed into hidden fields (inactive hop or other auth method) are never
    /// sent; the runtime deletes secret kinds the saved config no longer uses.
    pub(crate) fn secrets(&self, cx: &Context<CellarApp>) -> ConnectionSecrets {
        let server = self.ssh_server.hop_secrets(self.ssh, cx);
        let jump = self
            .ssh_jump
            .hop_secrets(self.ssh && self.ssh_jump_enabled, cx);
        ConnectionSecrets {
            password: Some(text_value(&self.password, cx)).filter(|value| !value.is_empty()),
            ssh_password: server.password,
            ssh_key_passphrase: server.passphrase,
            jump_password: jump.password,
            jump_key_passphrase: jump.passphrase,
        }
    }
}

impl CellarApp {
    fn edited_ssh_host(&mut self, jump: bool) -> Option<&mut SshHostEditor> {
        let editor = self.connection_editor.as_mut()?;
        editor.message = None;
        Some(if jump {
            &mut editor.ssh_jump
        } else {
            &mut editor.ssh_server
        })
    }

    pub(crate) fn select_ssh_auth(
        &mut self,
        jump: bool,
        method: SshAuthMethod,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(host) = self.edited_ssh_host(jump) {
            if host.auth_method != method {
                host.auth_method = method;
                // A pending clear belongs to the previously selected method.
                host.set_clear(SshSecretField::Password, false, window, cx);
                host.set_clear(SshSecretField::Passphrase, false, window, cx);
            }
        }
        cx.notify();
    }

    pub(crate) fn toggle_ssh_secret_clear(
        &mut self,
        jump: bool,
        field: SshSecretField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(host) = self.edited_ssh_host(jump) {
            if host.can_clear(field) {
                let clear = !host.clear_requested(field);
                host.set_clear(field, clear, window, cx);
            }
        }
        cx.notify();
    }
}

impl SshSecretField {
    fn auth_method(self) -> SshAuthMethod {
        match self {
            Self::Password => SshAuthMethod::Password,
            Self::Passphrase => SshAuthMethod::PrivateKey,
        }
    }
}

fn text_value(state: &Entity<InputState>, cx: &Context<CellarApp>) -> String {
    state.read(cx).value().to_string()
}

struct SecretInput<'a> {
    value: &'a str,
    clear: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct HopSecrets {
    password: Option<String>,
    passphrase: Option<String>,
}

fn hop_secrets(
    active: bool,
    auth_method: SshAuthMethod,
    password: SecretInput<'_>,
    passphrase: SecretInput<'_>,
) -> HopSecrets {
    let field = |input: SecretInput<'_>, field: SshSecretField| {
        (active && auth_method == field.auth_method())
            .then(|| secret_update(input))
            .flatten()
    };
    HopSecrets {
        password: field(password, SshSecretField::Password),
        passphrase: field(passphrase, SshSecretField::Passphrase),
    }
}

/// A typed value wins over a pending clear; a blank field alone keeps the
/// saved value.
fn secret_update(input: SecretInput<'_>) -> Option<String> {
    if !input.value.is_empty() {
        Some(input.value.to_owned())
    } else if input.clear {
        Some(String::new())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(value: &str, clear: bool) -> SecretInput<'_> {
        SecretInput { value, clear }
    }

    fn secrets(password: Option<&str>, passphrase: Option<&str>) -> HopSecrets {
        HopSecrets {
            password: password.map(str::to_owned),
            passphrase: passphrase.map(str::to_owned),
        }
    }

    #[test]
    fn hidden_fields_are_never_sent() {
        for method in [
            SshAuthMethod::Agent,
            SshAuthMethod::PrivateKey,
            SshAuthMethod::Password,
        ] {
            assert_eq!(
                hop_secrets(false, method, input("pw", true), input("pp", true)),
                secrets(None, None),
            );
        }
        assert_eq!(
            hop_secrets(
                true,
                SshAuthMethod::Agent,
                input("pw", true),
                input("pp", true)
            ),
            secrets(None, None),
        );
        assert_eq!(
            hop_secrets(
                true,
                SshAuthMethod::Password,
                input("pw", false),
                input("pp", true)
            ),
            secrets(Some("pw"), None),
        );
        assert_eq!(
            hop_secrets(
                true,
                SshAuthMethod::PrivateKey,
                input("pw", true),
                input("pp", false)
            ),
            secrets(None, Some("pp")),
        );
    }

    #[test]
    fn blank_keeps_and_clear_flag_clears() {
        assert_eq!(
            hop_secrets(
                true,
                SshAuthMethod::Password,
                input("", false),
                input("", false)
            ),
            secrets(None, None),
        );
        assert_eq!(
            hop_secrets(
                true,
                SshAuthMethod::Password,
                input("", true),
                input("", false)
            ),
            secrets(Some(""), None),
        );
        assert_eq!(
            hop_secrets(
                true,
                SshAuthMethod::PrivateKey,
                input("", false),
                input("", true)
            ),
            secrets(None, Some("")),
        );
    }

    #[test]
    fn typed_value_overrides_clear() {
        assert_eq!(
            hop_secrets(
                true,
                SshAuthMethod::Password,
                input("new", true),
                input("", false)
            ),
            secrets(Some("new"), None),
        );
    }
}

use super::*;
use crate::app::connection_editor::{SshHostEditor, SshSecretField};
use cellar_core::driver::SshAuthMethod;

pub(super) fn ssh_panel(editor: &ConnectionEditor, cx: &mut Context<CellarApp>) -> AnyElement {
    if !editor.engine.supports_ssh() {
        return div()
            .flex().flex_col().gap_2()
            .text_size(px(12.))
            .text_color(FG_MUTED)
            .child("SSH tunneling supports PostgreSQL (Supabase/Neon), MySQL (PlanetScale), and SQL Server. Azure SQL is not supported.")
            .when(editor.ssh, |element| element.child(
                action_button("remove-unsupported-ssh", "", "Remove saved tunnel", false, true)
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_connection_ssh(cx)))
            ))
            .into_any_element();
    }
    div().flex().flex_col().gap(px(10.))
        .child(form_row("Use SSH tunnel", None,
            toggle("connection-ssh-toggle", editor.ssh)
                .on_click(cx.listener(|this, _, _, cx| this.toggle_connection_ssh(cx)))
                .into_any_element()))
        .when(editor.ssh, |element| element
            .child(host_panel(&editor.ssh_server, false, cx))
            .child(form_row("Use jump host", None,
                toggle("connection-ssh-jump-toggle", editor.ssh_jump_enabled)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(editor) = &mut this.connection_editor {
                            editor.ssh_jump_enabled = !editor.ssh_jump_enabled;
                            editor.message = None;
                        }
                        cx.notify();
                    })).into_any_element()))
            .when(editor.ssh_jump_enabled, |element| element.child(host_panel(&editor.ssh_jump, true, cx)))
            .child(div().text_size(px(11.)).text_color(FG_MUTED)
                .child("Database host and port are resolved from the SSH server. Unknown SSH host keys are shown for fingerprint verification before they are trusted; changed or revoked keys are blocked. Passwords and key passphrases are saved in the OS keychain.")))
        .into_any_element()
}

fn host_panel(host: &SshHostEditor, jump: bool, cx: &mut Context<CellarApp>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(12.))
                .text_color(FG_SECONDARY)
                .child(if jump { "Jump host" } else { "SSH server" }),
        )
        .child(form_row(
            "Host",
            None,
            div()
                .flex()
                .flex_1()
                .gap(px(6.))
                .child(input_box(&host.host, true, None))
                .child(input_box(&host.port, true, Some(70.)))
                .into_any_element(),
        ))
        .child(form_row("User", None, input_box(&host.user, true, None)))
        .child(form_row(
            "Authentication",
            None,
            div()
                .flex()
                .gap(px(4.))
                .children(
                    [
                        (SshAuthMethod::Agent, "SSH agent"),
                        (SshAuthMethod::PrivateKey, "Private key"),
                        (SshAuthMethod::Password, "Password"),
                    ]
                    .map(|(method, label)| {
                        segment(label, host.auth_method == method)
                            .id(SharedString::from(format!("ssh-auth:{jump}:{label}")))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.select_ssh_auth(jump, method, window, cx)
                            }))
                    }),
                )
                .into_any_element(),
        ))
        .when(host.auth_method == SshAuthMethod::Password, |element| {
            element.child(form_row(
                "Password",
                None,
                secret_input(host, jump, SshSecretField::Password, cx),
            ))
        })
        .when(host.auth_method == SshAuthMethod::PrivateKey, |element| {
            element
                .child(form_row(
                    "Private key",
                    Some("Path to a local private key"),
                    input_box(&host.private_key_path, true, None),
                ))
                .child(form_row(
                    "Passphrase",
                    Some("For encrypted private keys"),
                    secret_input(host, jump, SshSecretField::Passphrase, cx),
                ))
        })
        .into_any_element()
}

/// Masked secret field plus, for a saved value, an explicit clear toggle.
/// Blank fields keep the saved value, so clearing needs its own control.
fn secret_input(
    host: &SshHostEditor,
    jump: bool,
    field: SshSecretField,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    let state = match field {
        SshSecretField::Password => &host.password,
        SshSecretField::Passphrase => &host.passphrase,
    };
    let clearing = host.clear_requested(field);
    let (id, label) = match (jump, field, clearing) {
        (false, SshSecretField::Password, false) => ("ssh-clear-password", "Clear saved password"),
        (false, SshSecretField::Password, true) => ("ssh-clear-password", "Keep saved password"),
        (false, SshSecretField::Passphrase, false) => {
            ("ssh-clear-passphrase", "Clear saved passphrase")
        }
        (false, SshSecretField::Passphrase, true) => {
            ("ssh-clear-passphrase", "Keep saved passphrase")
        }
        (true, SshSecretField::Password, false) => ("jump-clear-password", "Clear saved password"),
        (true, SshSecretField::Password, true) => ("jump-clear-password", "Keep saved password"),
        (true, SshSecretField::Passphrase, false) => {
            ("jump-clear-passphrase", "Clear saved passphrase")
        }
        (true, SshSecretField::Passphrase, true) => {
            ("jump-clear-passphrase", "Keep saved passphrase")
        }
    };
    div()
        .flex()
        .flex_1()
        .min_w_0()
        .gap(px(6.))
        .child(input_box(state, true, None))
        .when(host.can_clear(field), |element| {
            element.child(
                action_button(id, "", label, false, true).on_click(cx.listener(
                    move |this, _, window, cx| {
                        this.toggle_ssh_secret_clear(jump, field, window, cx)
                    },
                )),
            )
        })
        .into_any_element()
}

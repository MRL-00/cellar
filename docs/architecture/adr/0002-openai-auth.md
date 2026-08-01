# ADR 0002: OpenAI API-key and ChatGPT authentication

- **Status:** Accepted
- **Date:** 2026-08-01

## Context

Cellar's provider model is local-first and bring-your-own-credential. OpenAI
has two distinct supported access paths:

- Platform API keys for usage-based Responses API billing.
- ChatGPT sign-in for subscription-backed Codex access.

A ChatGPT OAuth token is not an interchangeable replacement for an OpenAI
Platform API key. OpenAI exposes the supported browser and device-code flows
through Codex app-server, which also owns token refresh and account state.

Returning either credential to the React webview would enlarge the trusted
surface and make accidental disclosure through frontend logs or extensions
more likely.

## Decision

OpenAI is a privileged first-party provider implemented behind typed Tauri
IPC.

API-key mode loads `ai:openai` from `cellar-secrets` in Rust and calls the
OpenAI Responses API directly. The key is never returned to the renderer.
Requests set `store: false` and do not enable provider tools.

ChatGPT mode starts `codex app-server` over stdio with a Cellar-specific
`CODEX_HOME`. App-server performs browser or device-code login, refreshes its
own tokens, lists the account's available models, and runs conversation turns.
Credential caching is explicitly configured to use the OS keychain.

ChatGPT turns use an empty Cellar-owned working directory, a read-only
sandbox, and `approvalPolicy: never`. Cellar instructs the model not to use
tools and interrupts any command, file, MCP, web-search, image, or collaboration
item that still starts. Only the final assistant text and token counts cross
back to React.

The first OAuth slice requires a current `codex` executable in `PATH`. Bundling
and signing a compatible sidecar is deferred until the protocol integration is
stable across macOS, Windows, and Linux.

## Consequences

- OpenAI credentials never enter the webview.
- Cellar still sends requests directly from the user's machine and does not
  introduce a hosted Cellar proxy.
- API-key usage follows OpenAI Platform billing and policy; ChatGPT usage
  follows the signed-in workspace's subscription and controls.
- ChatGPT sign-in is unavailable until the Codex CLI is installed. The UI must
  report that prerequisite rather than presenting a non-working control.
- Other providers can keep their existing frontend adapters until their threat
  model or authentication flow warrants a backend transport.

## Follow-up

- Bundle, sign, and version-pin the Codex app-server sidecar for release builds.
- Stream response deltas over Tauri events and expose turn cancellation.
- Add end-to-end coverage for browser callback and device-code login.

# ADR 0003: DeepSeek backend provider

- **Status:** Accepted
- **Date:** 2026-08-01

## Context

Cellar supports bring-your-own AI credentials and sends requests directly to
the selected provider. DeepSeek exposes model discovery and generation through
an OpenAI-compatible Chat Completions API, but it has provider-specific options
such as thinking mode.

Loading the key into the React webview would expand the credential trust
boundary. Building a one-off DeepSeek command surface would also make planned
compatible providers repeat the same model, message, usage, and error mapping.

## Decision

DeepSeek is a fixed first-party profile on a provider-neutral Rust backend
transport. Typed IPC accepts a closed provider enum, so the renderer cannot
choose an arbitrary destination. The service loads `ai:deepseek` from
`cellar-secrets`, discovers models from `https://api.deepseek.com/models`, and
sends full conversation history to `https://api.deepseek.com/chat/completions`.

Thinking mode is persisted as a DeepSeek setting and sent explicitly on each
generation request. Model identifiers are never maintained as an allowlist;
the provider's model endpoint remains authoritative.

## Consequences

- The DeepSeek key never enters the webview or plain-text settings.
- Cellar does not proxy or store DeepSeek requests.
- Model changes do not require a Cellar release.
- The shared transport can support additional fixed Chat Completions profiles
  without allowing arbitrary network destinations.
- OpenAI keeps its separate Responses API and ChatGPT OAuth implementations.

## Follow-up

- Add user-defined compatible endpoints when Cellar has capability and URL
  validation suitable for custom network destinations.
- Extend provider capabilities if future profiles need additional controls.
- Add streaming and cancellation across backend AI providers.

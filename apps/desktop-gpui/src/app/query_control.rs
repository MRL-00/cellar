use std::sync::Arc;

use cellar_core::driver::Engine;
use gpui::Context;

use cellar_desktop_gpui::model::{QueryState, TabKind};

use super::CellarApp;

pub(super) fn required_confirmations(production: bool, destructive: bool) -> u8 {
    u8::from(production) + u8::from(destructive)
}

pub(super) fn required_analyze_confirmations(_production: bool, _destructive: bool) -> u8 {
    1
}

impl CellarApp {
    pub(crate) fn cancel_active_query(&mut self, cx: &mut Context<Self>) {
        if let Some(tab_id) = self.model.active_tab().map(|tab| tab.id) {
            self.cancel_query(tab_id, cx);
        }
    }

    pub(super) fn cancel_query(&mut self, tab_id: u64, cx: &mut Context<Self>) {
        let Some((connection_id, engine)) = self.model.tabs().iter().find_map(|tab| {
            if tab.id != tab_id {
                return None;
            }
            match &tab.kind {
                TabKind::Query {
                    target,
                    state: QueryState::Running { .. },
                } => self
                    .model
                    .connections()
                    .iter()
                    .find(|config| config.id == target.connection_id)
                    .map(|config| (target.connection_id.clone(), config.engine)),
                _ => None,
            }
        }) else {
            return;
        };
        if engine.family() != Engine::Postgres {
            return;
        }
        let registry = Arc::clone(&self.registry);
        self.runtime.spawn(async move {
            let _ = registry
                .cancel_query(&connection_id, &format!("gpui-{tab_id}"))
                .await;
        });
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::{required_analyze_confirmations, required_confirmations};

    #[test]
    fn production_adds_a_confirmation_to_the_destructive_gate() {
        assert_eq!(required_confirmations(false, false), 0);
        assert_eq!(required_confirmations(false, true), 1);
        assert_eq!(required_confirmations(true, false), 1);
        assert_eq!(required_confirmations(true, true), 2);
    }

    #[test]
    fn analyze_always_uses_the_canonical_single_confirmation() {
        assert_eq!(required_analyze_confirmations(false, false), 1);
        assert_eq!(required_analyze_confirmations(false, true), 1);
        assert_eq!(required_analyze_confirmations(true, false), 1);
        assert_eq!(required_analyze_confirmations(true, true), 1);
    }
}

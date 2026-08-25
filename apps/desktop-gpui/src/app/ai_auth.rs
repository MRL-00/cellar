use std::{sync::Arc, time::Duration};

use gpui::Context;

use super::CellarApp;

impl CellarApp {
    pub(super) fn start_openai_auth_poll(&mut self, cx: &mut Context<Self>) {
        self.ai_auth_poll = None;
        let service = Arc::clone(&self.ai.openai);
        let runtime = Arc::clone(&self.runtime);
        self.ai_auth_poll = Some(cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(1500))
                .await;
            let service = Arc::clone(&service);
            let result = runtime
                .spawn(async move {
                    service
                        .oauth_status()
                        .await
                        .map_err(|error| error.to_string())
                })
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result);
            let finished = this
                .update(cx, |this, cx| match result {
                    Ok(status) => {
                        let signed_in = status.signed_in;
                        this.ai.oauth_status = Some(status);
                        this.ai.configured = signed_in;
                        if signed_in {
                            this.ai.login = None;
                            this.refresh_ai_models(cx);
                        }
                        cx.notify();
                        signed_in
                    }
                    Err(_) => false,
                })
                .unwrap_or(true);
            if finished {
                break;
            }
        }));
    }
}

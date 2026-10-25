use astra_plugin_sdk::prelude::*;

#[derive(Default)]
struct AstraPlay;

#[astra::plugin]
impl AstraPlay {
    /// Run this action from a command. Shown in the command editor.
    ///
    /// DECLARED plane: the label is a key and the DAEMON resolves it, per
    /// request, in whatever language the user is running. `#[action]` takes a
    /// string literal, so the `$` is written out rather than built with
    /// `i18n::key(..)` — same string either way. It must exist in
    /// `locales/en.json` or the editor shows the key; `astra-plugin check`
    /// says so before a user ever sees it.
    ///
    /// `icon` is the glyph it wears there. Give it a `viewBox` and paint with
    /// `currentColor`, with no width or height — Astra sizes and tints it.
    /// Drop the argument and the step falls back to a generic plugin mark.
    #[action(label = "$action.do_something.label", icon = "<svg viewBox='0 0 24 24'><path d='M13 2 4 14h6l-1 8 9-12h-6z' fill='none' stroke='currentColor' stroke-width='2' stroke-linejoin='round'/></svg>")]
    async fn do_something(&self, ctx: &PluginContext) -> Result<String, ActionError> {
        ctx.host().log_info("do_something ran").await?;
        // RUNTIME plane, unlike the label above: the plugin produces this
        // string, so the plugin translates it — here, now, with a count no
        // daemon can know.
        let n = 1;
        Ok(ctx.i18n().tn("msg.done", n, &[("n", &n.to_string())]))
    }

    /// The trigger types a user can attach a command to. Fire one with
    /// `ctx.host().fire_trigger("something_happened", payload_json)`; check
    /// `ctx.active_triggers().contains(..)` first if the payload is costly.
    #[hook]
    async fn trigger_types(&self) -> Vec<TriggerTypeDef> {
        vec![TriggerTypeDef {
            r#type: "something_happened".into(),
            // DECLARED plane, resolved by the daemon. `key` just writes the
            // `$`; the key has to be in `locales/en.json`.
            label: key("trigger.something_happened.label"),
            // The glyph it wears in the command editor: a `viewBox` and
            // `currentColor`, no width or height — Astra sizes and tints it.
            icon_svg: "<svg viewBox='0 0 24 24'><circle cx='12' cy='12' r='3' fill='currentColor'/><path d='M6 6a8.5 8.5 0 0 0 0 12M18 18a8.5 8.5 0 0 0 0-12' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round'/></svg>".into(),
            ..Default::default()
        }]
    }
}

astra::main!(AstraPlay::default());

#[cfg(test)]
mod tests {
    use super::*;
    use astra_plugin_sdk::testing::Harness;

    /// `cargo test`. The harness runs the hooks in process against a recording
    /// host: no daemon, no socket, no Astra installed.
    ///
    /// `h.host().fired_triggers()` / `.logs()` / `.variables()` say what the
    /// plugin told Astra; `h.host().deny("fire_trigger")` stages the refusal a
    /// user's `[permissions]` would produce. For the wire — registration, the
    /// session token, streaming audio — there is
    /// `astra_plugin_sdk::testing::WireHarness`.
    #[tokio::test]
    async fn it_starts_and_answers() {
        let h = Harness::new(AstraPlay::default())
            .with_config(json!({}))
            .start()
            .await
            .expect("the plugin started");

        assert!(h.health().await.0);
    }
}

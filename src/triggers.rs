use astra_plugin_sdk::prelude::*;

pub const MATCH_FOUND: &str = "match_found";
pub const MATCH_ACCEPTED: &str = "match_accepted";

const FOUND_ICON: &str = "<svg viewBox='0 0 24 24'><circle cx='12' cy='12' r='8' fill='none' stroke='currentColor' stroke-width='2'/><path d='M12 7v5l3 2' fill='none' stroke='currentColor' stroke-width='2' stroke-linecap='round'/></svg>";
const ACCEPTED_ICON: &str = "<svg viewBox='0 0 24 24'><path d='M5 12.5 10 17l9-10' fill='none' stroke='currentColor' stroke-width='2.4' stroke-linecap='round' stroke-linejoin='round'/></svg>";

pub fn definitions() -> Vec<TriggerTypeDef> {
    vec![
        TriggerTypeDef::new(MATCH_FOUND, key("trigger.match_found.label")).with_icon_svg(FOUND_ICON),
        TriggerTypeDef::new(MATCH_ACCEPTED, key("trigger.match_accepted.label"))
            .with_icon_svg(ACCEPTED_ICON),
    ]
}

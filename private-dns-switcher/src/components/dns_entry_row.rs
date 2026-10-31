use crate::models::DnsEntry;
use crate::services::ping::PingStatus;
use dioxus::prelude::*;

#[component]
pub fn DnsEntryRow(
    entry: DnsEntry,
    selected: bool,
    ping: Option<PingStatus>,
    on_select: EventHandler<u64>,
    on_edit: EventHandler<DnsEntry>,
    on_delete: EventHandler<u64>,
    on_ping: EventHandler<u64>,
) -> Element {
    let id = entry.id;
    let entry_for_edit = entry.clone();

    rsx! {
        div {
            class: if selected { "dns-row dns-row--selected" } else { "dns-row" },
            onclick: move |_| on_select.call(id),
            div { class: "dns-radio",
                if selected {
                    div { class: "dns-radio-dot" }
                }
            }
            div { class: "dns-info",
                div { class: "dns-name", "{entry.name}" }
                div { class: "dns-host", "{entry.hostname}" }
            }
            button {
                class: match ping {
                    Some(PingStatus::Testing) => "badge-ping badge-ping--testing",
                    Some(PingStatus::Latency(_)) => "badge-ping badge-ping--ok",
                    Some(PingStatus::Timeout) => "badge-ping badge-ping--fail",
                    None => "badge-ping",
                },
                title: "Test latency",
                onclick: move |evt| {
                    evt.stop_propagation();
                    on_ping.call(id);
                },
                match ping {
                    Some(PingStatus::Testing) => "⟳ ...".to_string(),
                    Some(PingStatus::Latency(ms)) => format!("{ms}ms"),
                    Some(PingStatus::Timeout) => "timeout".to_string(),
                    None => "⚡ ping".to_string(),
                }
            }
            div { class: "dns-actions",
                button {
                    class: "btn-mini",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        on_edit.call(entry_for_edit.clone());
                    },
                    "✎"
                }
                button {
                    class: "btn-mini",
                    onclick: move |evt| {
                        evt.stop_propagation();
                        on_delete.call(id);
                    },
                    "🗑"
                }
            }
        }
    }
}

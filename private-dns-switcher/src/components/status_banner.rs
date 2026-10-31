use crate::services::dns::{DnsMode, DnsStatus};
use dioxus::prelude::*;

#[component]
pub fn StatusBanner(status: DnsStatus) -> Element {
    if !status.permitted {
        return rsx! {
            div { class: "banner banner--denied",
                div { class: "banner-title", "⚠ Permission Required" }
                div { class: "banner-host", "WRITE_SECURE_SETTINGS is not granted." }
            }
        };
    }

    match status.mode {
        DnsMode::Hostname => rsx! {
            div { class: "banner banner--active",
                div { class: "banner-title", "● DNS Active" }
                div { class: "banner-host", "{status.hostname}" }
            }
        },
        DnsMode::Off => rsx! {
            div { class: "banner banner--disabled",
                div { class: "banner-title", "○ DNS Off" }
                div { class: "banner-host", "System default DNS is in use" }
            }
        },
        DnsMode::Unknown => rsx! {
            div { class: "banner banner--disabled",
                div { class: "banner-title", "? Status Unknown" }
            }
        },
    }
}

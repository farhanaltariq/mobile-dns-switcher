use crate::services::dns::{DnsMode, DnsStatus};
use dioxus::prelude::*;

#[component]
pub fn StatusBanner(status: DnsStatus) -> Element {
    if status.strict_private_dns {
        return rsx! {
            div { class: "banner banner--denied",
                div { class: "banner-title", "⚠ System Private DNS Conflict" }
                div { class: "banner-host", "Android Settings has Private DNS set to Strict mode. Change it to Automatic or Off for DNS Switcher to route queries." }
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

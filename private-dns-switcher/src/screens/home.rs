use std::collections::HashMap;

use dioxus::prelude::*;

use crate::components::{DnsEntryRow, StatusBanner};
use crate::models::DnsEntry;
use crate::services::ping::{self, PingStatus};
use crate::services::{dns, dns_store};

#[component]
pub fn HomeScreen() -> Element {
    let mut status = use_signal(dns::get_status);
    let mut entries = use_signal(dns_store::load);
    let mut pings = use_signal(HashMap::<u64, PingStatus>::new);
    let mut is_benchmarking = use_signal(|| false);

    // Initial selected ID based on active hostname or first entry
    let mut selected_id = use_signal(|| {
        let current_host = dns::get_status().hostname;
        let loaded = dns_store::load();
        loaded
            .iter()
            .find(|e| e.hostname == current_host)
            .map(|e| e.id)
            .or_else(|| loaded.first().map(|e| e.id))
    });

    // Dialog state: None if closed, Some(entry) for Add (id == 0) or Edit (id > 0)
    let mut editing_entry = use_signal(|| None::<DnsEntry>);
    let mut form_name = use_signal(String::new);
    let mut form_host = use_signal(String::new);
    let mut error_msg = use_signal(|| None::<String>);

    let current_status = status.read().clone();
    let is_permitted = current_status.permitted;

    let selected_entry = entries
        .read()
        .iter()
        .find(|e| Some(e.id) == *selected_id.read())
        .cloned();

    // Ping single DNS entry
    let on_ping_single = move |id: u64| {
        if let Some(entry) = entries.read().iter().find(|e| e.id == id).cloned() {
            pings.write().insert(id, PingStatus::Testing);
            spawn(async move {
                let res = match ping::ping_dot(&entry.hostname) {
                    Some(ms) => PingStatus::Latency(ms),
                    None => PingStatus::Timeout,
                };
                pings.write().insert(id, res);
            });
        }
    };

    // Sequential benchmark of all entries; auto-selects lowest latency
    let on_benchmark_all = move |_| {
        if *is_benchmarking.read() {
            return;
        }
        is_benchmarking.set(true);
        let list = entries.read().clone();
        spawn(async move {
            let mut best: Option<(u64, u128)> = None;
            for entry in list {
                pings.write().insert(entry.id, PingStatus::Testing);
                let res = match ping::ping_dot(&entry.hostname) {
                    Some(ms) => {
                        if best.map_or(true, |(_, min_ms)| ms < min_ms) {
                            best = Some((entry.id, ms));
                        }
                        PingStatus::Latency(ms)
                    }
                    None => PingStatus::Timeout,
                };
                pings.write().insert(entry.id, res);
            }
            if let Some((best_id, _)) = best {
                selected_id.set(Some(best_id));
            }
            is_benchmarking.set(false);
        });
    };

    rsx! {
        div { class: "app",
            // Header
            header { class: "header",
                h1 { "Private DNS" }
                button {
                    class: "btn-icon",
                    title: "Add DNS Provider",
                    onclick: move |_| {
                        form_name.set(String::new());
                        form_host.set(String::new());
                        editing_entry.set(Some(DnsEntry::new(0, "", "")));
                    },
                    "+"
                }
            }

            // Current Status Banner
            StatusBanner { status: current_status }

            // Permission Warning Card if WRITE_SECURE_SETTINGS not granted
            if !is_permitted {
                div { class: "permission-box",
                    strong { "Setup Command Required" }
                    span { "Run this via ADB on your computer to grant permission:" }
                    code { class: "permission-cmd",
                        "adb shell pm grant com.sicloud.dnsswitcher android.permission.WRITE_SECURE_SETTINGS"
                    }
                }
            }

            // DNS Providers List Section Header
            div { class: "section-header",
                div { class: "section-label", "Configured Providers" }
                button {
                    class: if *is_benchmarking.read() { "btn-bench btn-disabled" } else { "btn-bench" },
                    disabled: *is_benchmarking.read(),
                    onclick: on_benchmark_all,
                    if *is_benchmarking.read() { "Testing..." } else { "⚡ Test All (Find Best)" }
                }
            }

            div { class: "dns-list",
                for entry in entries.read().iter().cloned() {
                    {
                        let is_sel = *selected_id.read() == Some(entry.id);
                        let ping_res = pings.read().get(&entry.id).copied();
                        rsx! {
                            DnsEntryRow {
                                key: "{entry.id}",
                                entry: entry.clone(),
                                selected: is_sel,
                                ping: ping_res,
                                on_select: move |id| selected_id.set(Some(id)),
                                on_ping: on_ping_single,
                                on_edit: move |e: DnsEntry| {
                                    form_name.set(e.name.clone());
                                    form_host.set(e.hostname.clone());
                                    editing_entry.set(Some(e));
                                },
                                on_delete: move |id| {
                                    entries.write().retain(|e| e.id != id);
                                    let _ = dns_store::save(&entries.read());
                                    pings.write().remove(&id);
                                    if *selected_id.read() == Some(id) {
                                        let next = entries.read().first().map(|e| e.id);
                                        selected_id.set(next);
                                    }
                                },
                            }
                        }
                    }
                }
            }

            // Action Buttons
            div { class: "actions",
                if let Some(ref err) = *error_msg.read() {
                    div { class: "banner banner--denied", "{err}" }
                }

                button {
                    class: if selected_entry.is_some() { "btn btn-primary" } else { "btn btn-primary btn-disabled" },
                    onclick: move |_| {
                        if let Some(target) = selected_entry.clone() {
                            match dns::set_private_dns(Some(&target.hostname)) {
                                Ok(()) => {
                                    error_msg.set(None);
                                    status.set(dns::get_status());
                                }
                                Err(e) => {
                                    error_msg.set(Some(format!("Failed to activate: {e}")));
                                    status.set(dns::get_status());
                                }
                            }
                        }
                    },
                    "Enable Private DNS"
                }

                button {
                    class: "btn btn-secondary",
                    onclick: move |_| {
                        match dns::set_private_dns(None) {
                            Ok(()) => {
                                error_msg.set(None);
                                status.set(dns::get_status());
                            }
                            Err(e) => {
                                error_msg.set(Some(format!("Failed to turn off: {e}")));
                                status.set(dns::get_status());
                            }
                        }
                    },
                    "Turn Off Private DNS"
                }
            }

            // Add / Edit Modal Dialog
            if let Some(entry) = editing_entry.read().clone() {
                div {
                    class: "dialog-backdrop",
                    onclick: move |_| editing_entry.set(None),
                    div {
                        class: "dialog",
                        onclick: move |evt| evt.stop_propagation(),
                        h2 { if entry.id == 0 { "Add DNS Provider" } else { "Edit DNS Provider" } }
                        div { class: "field",
                            label { "Display Name" }
                            input {
                                r#type: "text",
                                placeholder: "e.g. Quad9 DNS",
                                value: "{form_name}",
                                oninput: move |evt| form_name.set(evt.value()),
                            }
                        }
                        div { class: "field",
                            label { "TLS Hostname (DoT)" }
                            input {
                                r#type: "text",
                                placeholder: "e.g. dns.quad9.net",
                                value: "{form_host}",
                                oninput: move |evt| form_host.set(evt.value()),
                            }
                        }
                        div { class: "dialog-buttons",
                            button {
                                class: "btn btn-secondary",
                                onclick: move |_| editing_entry.set(None),
                                "Cancel"
                            }
                            button {
                                class: "btn btn-primary",
                                onclick: move |_| {
                                    let name = form_name.read().trim().to_string();
                                    let host = form_host.read().trim().to_string();
                                    if name.is_empty() || host.is_empty() {
                                        return;
                                    }

                                    if entry.id == 0 {
                                        let new_id = dns_store::next_id(&entries.read());
                                        let new_item = DnsEntry::new(new_id, name, host);
                                        entries.write().push(new_item);
                                        let _ = dns_store::save(&entries.read());
                                        selected_id.set(Some(new_id));
                                    } else {
                                        let mut list = entries.write();
                                        if let Some(item) = list.iter_mut().find(|e| e.id == entry.id) {
                                            item.name = name;
                                            item.hostname = host;
                                        }
                                        let _ = dns_store::save(&list);
                                    }
                                    editing_entry.set(None);
                                },
                                "Save"
                            }
                        }
                    }
                }
            }
        }
    }
}

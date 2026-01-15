use dioxus::prelude::*;
use crate::api;
use crate::models::StorageStats;
use crate::components::common::{LoadingSpinner, ErrorMessage, Card};

#[component]
pub fn StorageDashboard() -> Element {
    let mut stats = use_signal(|| None::<StorageStats>);
    let mut is_loading = use_signal(|| true);
    let mut error = use_signal(|| None::<String>);

    // Fetch storage stats on component mount
    use_effect(move || {
        spawn(async move {
            is_loading.set(true);
            error.set(None);

            match api::recordings::get_storage_stats().await {
                Ok(storage_stats) => {
                    stats.set(Some(storage_stats));
                    is_loading.set(false);
                }
                Err(e) => {
                    error.set(Some(format!("Failed to load storage stats: {}", e)));
                    is_loading.set(false);
                }
            }
        });
    });

    rsx! {
        div { class: "space-y-6",
            h2 { class: "text-xl font-bold mb-4", "Storage Dashboard" }

            if *is_loading.read() {
                LoadingSpinner {}
            } else if let Some(err) = error.read().as_ref() {
                ErrorMessage { message: err.clone() }
            } else if let Some(storage_stats) = stats.read().as_ref() {
                // Alert banner when storage >80% full
                if storage_stats.quota_percentage >= 80.0 {
                    div {
                        class: "bg-yellow-50 border-l-4 border-yellow-400 p-4 mb-4",
                        div { class: "flex items-center",
                            span { class: "text-2xl mr-3", "\u{26A0}" }
                            div {
                                p { class: "font-medium text-yellow-800",
                                    "Storage Warning"
                                }
                                p { class: "text-sm text-yellow-700",
                                    "Storage is at {storage_stats.quota_percentage:.1}% capacity. Consider increasing quota or cleaning up old recordings."
                                }
                            }
                        }
                    }
                }

                // Stats grid
                div { class: "grid gap-6 md:grid-cols-3 mb-6",
                    StatCard {
                        title: "Total Recordings",
                        value: storage_stats.total_files.to_string(),
                        icon: "\u{1F4C1}",
                        color: "blue",
                    }
                    StatCard {
                        title: "Storage Used",
                        value: format!("{:.2} GB", storage_stats.total_size_gb),
                        icon: "\u{1F4BE}",
                        color: "purple",
                    }
                    StatCard {
                        title: "Storage Quota",
                        value: format!("{:.0} GB", storage_stats.quota_gb),
                        icon: "\u{1F4CA}",
                        color: "green",
                    }
                }

                // Storage quota bar
                Card {
                    h3 { class: "font-semibold mb-4", "Storage Usage" }
                    div { class: "space-y-2",
                        div { class: "flex justify-between text-sm mb-1",
                            span { class: "text-gray-600", "Used: {storage_stats.total_size_gb:.2} GB" }
                            span { class: "text-gray-600", "Quota: {storage_stats.quota_gb:.0} GB" }
                        }
                        div { class: "w-full bg-gray-200 rounded-full h-4 overflow-hidden",
                            div {
                                class: if storage_stats.quota_percentage >= 90.0 {
                                    "h-full bg-red-600 transition-all duration-300"
                                } else if storage_stats.quota_percentage >= 80.0 {
                                    "h-full bg-yellow-500 transition-all duration-300"
                                } else {
                                    "h-full bg-green-600 transition-all duration-300"
                                },
                                style: "width: {storage_stats.quota_percentage.min(100.0)}%",
                            }
                        }
                        div { class: "flex justify-between text-sm mt-1",
                            span { class: "text-gray-500", "{storage_stats.quota_percentage:.1}% used" }
                            span { class: "text-gray-500",
                                "{(storage_stats.quota_gb - storage_stats.total_size_gb).max(0.0):.2} GB remaining"
                            }
                        }
                    }
                }

                // Daily usage chart
                Card { class: "mt-6",
                    h3 { class: "font-semibold mb-4", "Daily Usage (Last 30 Days)" }

                    if storage_stats.daily_usage.is_empty() {
                        div { class: "text-center text-gray-500 py-8",
                            "No usage data available"
                        }
                    } else {
                        div { class: "space-y-4",
                            // Chart area
                            div { class: "flex items-end justify-between gap-1 h-48 border-b border-gray-200 pb-2",
                                for usage in storage_stats.daily_usage.iter().rev().take(30).rev() {
                                    DailyUsageBar {
                                        key: "{usage.date}",
                                        date: usage.date.to_string(),
                                        total_size_gb: usage.total_size_bytes as f64 / 1_073_741_824.0,
                                        max_size_gb: storage_stats.daily_usage.iter()
                                            .map(|u| u.total_size_bytes as f64 / 1_073_741_824.0)
                                            .fold(0.0, f64::max),
                                        recordings_added: usage.recordings_added,
                                        recordings_deleted: usage.recordings_deleted,
                                    }
                                }
                            }

                            // Legend
                            div { class: "flex gap-4 text-sm text-gray-600 justify-center",
                                div { class: "flex items-center gap-2",
                                    div { class: "w-3 h-3 bg-blue-500 rounded" }
                                    span { "Storage Size" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StatCard(title: String, value: String, icon: String, color: String) -> Element {
    let bg_color = match color.as_str() {
        "blue" => "bg-blue-100 text-blue-600",
        "green" => "bg-green-100 text-green-600",
        "purple" => "bg-purple-100 text-purple-600",
        "yellow" => "bg-yellow-100 text-yellow-600",
        "red" => "bg-red-100 text-red-600",
        _ => "bg-gray-100 text-gray-600",
    };

    rsx! {
        Card {
            div { class: "flex items-center justify-between",
                div {
                    p { class: "text-sm text-gray-500", "{title}" }
                    p { class: "text-2xl font-bold", "{value}" }
                }
                div { class: "w-12 h-12 rounded-full flex items-center justify-center text-2xl {bg_color}",
                    "{icon}"
                }
            }
        }
    }
}

#[component]
fn DailyUsageBar(
    date: String,
    total_size_gb: f64,
    max_size_gb: f64,
    recordings_added: i32,
    recordings_deleted: i32,
) -> Element {
    // Calculate height percentage (minimum 2% for visibility)
    let height_percent = if max_size_gb > 0.0 {
        ((total_size_gb / max_size_gb) * 100.0).max(2.0)
    } else {
        2.0
    };

    // Format date to show only day
    let day = date.split('-').last().unwrap_or(&date);

    rsx! {
        div {
            class: "flex-1 flex flex-col items-center group relative",
            title: "Date: {date}\nSize: {total_size_gb:.2} GB\nAdded: {recordings_added}\nDeleted: {recordings_deleted}",

            // Bar
            div { class: "w-full flex items-end justify-center",
                div {
                    class: "w-full bg-blue-500 rounded-t hover:bg-blue-600 transition-colors cursor-pointer",
                    style: "height: {height_percent}%",
                }
            }

            // Date label (show every 5 days to avoid clutter)
            if day.parse::<i32>().unwrap_or(0) % 5 == 0 {
                div { class: "text-xs text-gray-500 mt-1 transform -rotate-45 origin-top-left",
                    "{day}"
                }
            }

            // Tooltip on hover
            div {
                class: "hidden group-hover:block absolute bottom-full mb-2 z-10 bg-gray-800 text-white text-xs rounded py-2 px-3 whitespace-nowrap",
                div { "{date}" }
                div { "{total_size_gb:.2} GB" }
                div { "+{recordings_added} / -{recordings_deleted}" }
            }
        }
    }
}

use dioxus::prelude::*;
use crate::api;

#[component]
pub fn AudioPlayer(
    recording_id: i64,
    #[props(default = true)]
    show_controls: bool,
) -> Element {
    let mut is_playing = use_signal(|| false);
    let mut current_time = use_signal(|| 0.0);
    let mut duration = use_signal(|| 0.0);
    let mut volume = use_signal(|| 100);
    let mut playback_rate = use_signal(|| 1.0);
    let mut is_loading = use_signal(|| true);

    let stream_url = api::recordings::get_stream_url(recording_id);
    let download_url = api::recordings::get_download_url(recording_id);

    // Audio element ID for JavaScript access
    let audio_id = format!("audio-player-{}", recording_id);

    // Helper function to get audio element
    #[cfg(target_arch = "wasm32")]
    let get_audio_element = move || -> Option<web_sys::HtmlAudioElement> {
        use wasm_bindgen::JsCast;
        web_sys::window()?
            .document()?
            .get_element_by_id(&audio_id)?
            .dyn_into::<web_sys::HtmlAudioElement>()
            .ok()
    };

    let toggle_play = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(audio) = get_audio_element() {
                if *is_playing.read() {
                    let _ = audio.pause();
                    is_playing.set(false);
                } else {
                    let _ = audio.play();
                    is_playing.set(true);
                }
            }
        }
    };

    let on_timeupdate = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(audio) = get_audio_element() {
                current_time.set(audio.current_time());
            }
        }
    };

    let on_loadedmetadata = move |_| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(audio) = get_audio_element() {
                duration.set(audio.duration());
                is_loading.set(false);
            }
        }
    };

    let on_ended = move |_| {
        is_playing.set(false);
    };

    let handle_seek = move |event: Event<FormEvent>| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(audio) = get_audio_element() {
                if let Ok(value) = event.value().parse::<f64>() {
                    audio.set_current_time(value);
                    current_time.set(value);
                }
            }
        }
    };

    let handle_volume = move |event: Event<FormEvent>| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(audio) = get_audio_element() {
                if let Ok(value) = event.value().parse::<i32>() {
                    let vol = value as f64 / 100.0;
                    audio.set_volume(vol);
                    volume.set(value);
                }
            }
        }
    };

    let set_speed = move |speed: f64| {
        move |_| {
            #[cfg(target_arch = "wasm32")]
            {
                if let Some(audio) = get_audio_element() {
                    audio.set_playback_rate(speed);
                    playback_rate.set(speed);
                }
            }
        }
    };

    let handle_progress_click = move |event: Event<MouseData>| {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(audio) = get_audio_element() {
                use wasm_bindgen::JsCast;
                if let Some(target) = event.target() {
                    if let Ok(element) = target.dyn_into::<web_sys::HtmlElement>() {
                        let rect = element.get_bounding_client_rect();
                        let x = event.client_coordinates().x as f64 - rect.left();
                        let percent = x / rect.width();
                        let time = percent * *duration.read();
                        audio.set_current_time(time);
                        current_time.set(time);
                    }
                }
            }
        }
    };

    // Format time in mm:ss format
    let format_time = |seconds: f64| -> String {
        if seconds.is_nan() || seconds.is_infinite() {
            return "0:00".to_string();
        }
        let mins = (seconds / 60.0).floor() as i32;
        let secs = (seconds % 60.0).floor() as i32;
        format!("{}:{:02}", mins, secs)
    };

    let current_time_str = format_time(*current_time.read());
    let duration_str = format_time(*duration.read());
    let progress_percent = if *duration.read() > 0.0 {
        (*current_time.read() / *duration.read()) * 100.0
    } else {
        0.0
    };

    rsx! {
        div { class: "bg-white rounded-lg shadow-md p-4",
            // Audio element
            audio {
                id: "{audio_id}",
                src: "{stream_url}",
                preload: "metadata",
                ontimeupdate: on_timeupdate,
                onloadedmetadata: on_loadedmetadata,
                onended: on_ended,
                style: "display: none;",
            }

            if show_controls {
                div { class: "space-y-4",
                    // Progress bar
                    div { class: "space-y-1",
                        div {
                            class: "relative h-2 bg-gray-200 rounded-full overflow-hidden cursor-pointer",
                            onclick: handle_progress_click,
                            div {
                                class: "absolute top-0 left-0 h-full bg-blue-600 transition-all",
                                style: "width: {progress_percent}%",
                            }
                        }
                        div { class: "flex justify-between text-xs text-gray-500",
                            span { "{current_time_str}" }
                            span { "{duration_str}" }
                        }
                    }

                    // Main controls
                    div { class: "flex items-center gap-4",
                        // Play/Pause button
                        button {
                            class: "flex items-center justify-center w-12 h-12 bg-blue-600 hover:bg-blue-700 text-white rounded-full transition-colors disabled:opacity-50",
                            disabled: *is_loading.read(),
                            onclick: toggle_play,
                            title: if *is_playing.read() { "Pause" } else { "Play" },
                            if *is_loading.read() {
                                span { class: "animate-spin", "\u{21BB}" }
                            } else if *is_playing.read() {
                                span { class: "text-xl", "\u{23F8}" }
                            } else {
                                span { class: "text-xl", "\u{25B6}" }
                            }
                        }

                        // Volume control
                        div { class: "flex items-center gap-2 flex-1",
                            span { class: "text-gray-600", "\u{1F50A}" }
                            input {
                                r#type: "range",
                                class: "flex-1 h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer",
                                min: "0",
                                max: "100",
                                value: "{volume.read()}",
                                oninput: handle_volume,
                            }
                            span { class: "text-xs text-gray-500 w-8 text-right",
                                "{volume.read()}%"
                            }
                        }

                        // Playback speed controls
                        div { class: "flex items-center gap-2",
                            span { class: "text-sm text-gray-600", "Speed:" }
                            for speed in [0.5, 1.0, 1.5, 2.0] {
                                button {
                                    key: "{speed}",
                                    class: if *playback_rate.read() == speed {
                                        "px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 transition-colors"
                                    } else {
                                        "px-3 py-1 text-sm bg-gray-200 text-gray-700 rounded hover:bg-gray-300 transition-colors"
                                    },
                                    onclick: set_speed(speed),
                                    "{speed}x"
                                }
                            }
                        }

                        // Download button
                        a {
                            href: "{download_url}",
                            download: true,
                            class: "flex items-center gap-1 px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded transition-colors",
                            title: "Download recording",
                            span { "\u{2B07}" }
                            span { class: "text-sm", "Download" }
                        }
                    }
                }
            } else {
                // Minimal player - just the audio element
                div { class: "text-center text-gray-500",
                    "Audio player controls hidden"
                }
            }
        }
    }
}

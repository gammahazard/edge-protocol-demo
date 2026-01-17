//! Tab navigation component

use leptos::prelude::*;

#[component]
pub fn TabNav(
    active_tab: ReadSignal<usize>,
    set_active_tab: WriteSignal<usize>,
) -> impl IntoView {
    view! {
        <div class="tabs">
            <button
                class=move || if active_tab.get() == 0 { "tab active" } else { "tab" }
                on:click=move |_| set_active_tab.set(0)
            >
                "🔗 URL Shortener"
            </button>
            <button
                class=move || if active_tab.get() == 1 { "tab active" } else { "tab" }
                on:click=move |_| set_active_tab.set(1)
            >
                "⏱️ Rate Limiter"
            </button>
            <button
                class=move || if active_tab.get() == 2 { "tab active" } else { "tab" }
                on:click=move |_| set_active_tab.set(2)
            >
                "🔒 Capabilities"
            </button>
        </div>
    }
}

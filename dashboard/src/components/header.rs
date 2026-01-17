//! Header component

use leptos::prelude::*;

#[component]
pub fn Header() -> impl IntoView {
    view! {
        <header class="header">
            <div>
                <h1>"Edge Protocol Demo"</h1>
                <p class="subtitle">"Cloudflare Workers + Rust WASM"</p>
            </div>
            <span class="badge">"Live on Edge"</span>
        </header>
    }
}

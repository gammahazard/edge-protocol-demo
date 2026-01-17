//! ==============================================================================
//! lib.rs - Edge Protocol Demo Dashboard
//! ==============================================================================
//!
//! purpose:
//!     leptos wasm dashboard for interacting with cloudflare workers.
//!     provides visual interface to test url shortener, rate limiter,
//!     and capability explorer.
//!
//! architecture:
//!     - leptos csr (client-side rendering)
//!     - compiled to wasm, runs in browser
//!     - calls worker apis via fetch
//!     - hosted on cloudflare pages
//!
//! ==============================================================================

use leptos::prelude::*;
use wasm_bindgen::prelude::*;

mod api;
mod components;

use components::{Header, TabNav, UrlShortenerTab, RateLimiterTab, CapabilityTab};

// ==============================================================================
// main entry point
// ==============================================================================

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}

// ==============================================================================
// app component
// ==============================================================================

#[component]
fn App() -> impl IntoView {
    // track active tab
    let (active_tab, set_active_tab) = signal(0usize);
    
    view! {
        <Header />
        <div class="container">
            <TabNav active_tab=active_tab set_active_tab=set_active_tab />
            
            <Show when=move || active_tab.get() == 0>
                <UrlShortenerTab />
            </Show>
            
            <Show when=move || active_tab.get() == 1>
                <RateLimiterTab />
            </Show>
            
            <Show when=move || active_tab.get() == 2>
                <CapabilityTab />
            </Show>
        </div>
    }
}

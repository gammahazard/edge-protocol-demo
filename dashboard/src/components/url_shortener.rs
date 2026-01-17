//! URL Shortener tab component

use leptos::prelude::*;
use crate::api;

#[component]
pub fn UrlShortenerTab() -> impl IntoView {
    let (url_input, set_url_input) = signal(String::new());
    let (result, set_result) = signal::<Option<Result<api::ShortenResponse, String>>>(None);
    let (loading, set_loading) = signal(false);
    
    // shorten action
    let shorten = move |_| {
        let url = url_input.get();
        if url.is_empty() {
            return;
        }
        
        set_loading.set(true);
        set_result.set(None);
        
        leptos::task::spawn_local(async move {
            let res = api::shorten_url(&url).await;
            set_result.set(Some(res));
            set_loading.set(false);
        });
    };
    
    view! {
        <div class="card">
            <h2>"🔗 URL Shortener"</h2>
            <p style="color: var(--text-secondary); margin-bottom: 1rem; font-size: 0.875rem;">
                "Create short URLs using Workers KV storage at the edge."
            </p>
            
            <div class="input-group">
                <input
                    type="text"
                    placeholder="https://example.com/very/long/url"
                    prop:value=move || url_input.get()
                    on:input=move |ev| set_url_input.set(event_target_value(&ev))
                />
                <button 
                    on:click=shorten
                    disabled=move || loading.get() || url_input.get().is_empty()
                >
                    {move || if loading.get() {
                        view! { <span class="spinner"></span> " Shortening..." }.into_any()
                    } else {
                        view! { "Shorten" }.into_any()
                    }}
                </button>
            </div>
            
            // result display
            {move || result.get().map(|r| match r {
                Ok(resp) => {
                    let short_url = resp.short_url.clone();
                    let short_url_display = resp.short_url.clone();
                    let code = resp.code.clone();
                    
                    view! {
                        <div class="result success">
                            <div class="result-label">"Short URL"</div>
                            <div class="result-value">
                                <a href=short_url target="_blank" style="color: var(--accent-primary);">
                                    {short_url_display}
                                </a>
                            </div>
                            <div class="stats-row" style="margin-top: 1rem;">
                                <div class="stat">
                                    <div class="stat-value">{code}</div>
                                    <div class="stat-label">"Code"</div>
                                </div>
                                <div class="stat">
                                    <div class="stat-value">"0"</div>
                                    <div class="stat-label">"Clicks"</div>
                                </div>
                                <div class="stat">
                                    <div class="stat-value">"Just now"</div>
                                    <div class="stat-label">"Created"</div>
                                </div>
                            </div>
                        </div>
                    }.into_any()
                },
                Err(e) => view! {
                    <div class="result error">
                        <div class="result-label">"Error"</div>
                        <div class="result-value">{e}</div>
                    </div>
                }.into_any(),
            })}
        </div>
        
        <div class="card">
            <h2>"How It Works"</h2>
            <p style="color: var(--text-secondary); font-size: 0.875rem; line-height: 1.8;">
                "This worker uses "<strong>"Workers KV"</strong>" to store URL mappings at the edge. "
                "When you create a short URL, it's instantly available from 300+ locations worldwide. "
                "Click tracking is updated on each redirect."
            </p>
        </div>
    }
}

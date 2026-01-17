//! URL Shortener tab component

use leptos::prelude::*;
use crate::api;
use serde::{Deserialize, Serialize};

const STORAGE_KEY: &str = "edge-demo-shortened-urls";

/// Stored URL entry for localStorage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredUrl {
    code: String,
    short_url: String,
    original_url: String,
    created_at: u64,
    clicks: u64,
}

/// Load URLs from localStorage
fn load_stored_urls() -> Vec<StoredUrl> {
    let window = web_sys::window().unwrap();
    let storage = window.local_storage().ok().flatten();
    
    if let Some(storage) = storage {
        if let Ok(Some(json)) = storage.get_item(STORAGE_KEY) {
            if let Ok(urls) = serde_json::from_str::<Vec<StoredUrl>>(&json) {
                return urls;
            }
        }
    }
    Vec::new()
}

/// Save URLs to localStorage
fn save_stored_urls(urls: &[StoredUrl]) {
    let window = web_sys::window().unwrap();
    if let Some(storage) = window.local_storage().ok().flatten() {
        if let Ok(json) = serde_json::to_string(urls) {
            let _ = storage.set_item(STORAGE_KEY, &json);
        }
    }
}

/// Get current timestamp in seconds
fn now_timestamp() -> u64 {
    (js_sys::Date::now() / 1000.0) as u64
}

/// Format relative time
fn format_relative_time(timestamp: u64) -> String {
    let now = now_timestamp();
    let diff = now.saturating_sub(timestamp);
    
    if diff < 60 {
        "Just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

#[component]
pub fn UrlShortenerTab() -> impl IntoView {
    let (url_input, set_url_input) = signal(String::new());
    let (result, set_result) = signal::<Option<Result<api::ShortenResponse, String>>>(None);
    let (loading, set_loading) = signal(false);
    let (stored_urls, set_stored_urls) = signal(load_stored_urls());
    
    // refresh stats for all stored URLs
    let refresh_stats = move || {
        let urls = stored_urls.get();
        for url in urls.iter() {
            let code = url.code.clone();
            leptos::task::spawn_local({
                let set_stored_urls = set_stored_urls.clone();
                async move {
                    if let Ok(stats) = api::get_url_stats(&code).await {
                        set_stored_urls.update(|urls| {
                            if let Some(url) = urls.iter_mut().find(|u| u.code == code) {
                                url.clicks = stats.clicks;
                            }
                            save_stored_urls(urls);
                        });
                    }
                }
            });
        }
    };
    
    // refresh stats on mount
    Effect::new(move || {
        refresh_stats();
    });
    
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
            
            // if successful, add to stored URLs
            if let Ok(ref resp) = res {
                let new_url = StoredUrl {
                    code: resp.code.clone(),
                    short_url: resp.short_url.clone(),
                    original_url: resp.original_url.clone(),
                    created_at: now_timestamp(),
                    clicks: 0,
                };
                
                set_stored_urls.update(|urls| {
                    // avoid duplicates
                    if !urls.iter().any(|u| u.code == new_url.code) {
                        urls.insert(0, new_url);
                        save_stored_urls(urls);
                    }
                });
            }
            
            set_result.set(Some(res));
            set_loading.set(false);
        });
    };
    
    // delete a stored URL
    let delete_url = move |code: String| {
        set_stored_urls.update(|urls| {
            urls.retain(|u| u.code != code);
            save_stored_urls(urls);
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
            
            // result display - simple toast since table shows details
            {move || result.get().map(|r| match r {
                Ok(resp) => {
                    let short_url = resp.short_url.clone();
                    let short_url_display = short_url.clone();
                    
                    view! {
                        <div class="result success" style="display: flex; align-items: center; gap: 0.75rem;">
                            <span style="font-size: 1.25rem;">"✓"</span>
                            <div>
                                <div style="font-weight: 600;">"URL shortened!"</div>
                                <a href=short_url target="_blank" style="color: var(--accent-primary); font-size: 0.875rem;">
                                    {short_url_display}
                                </a>
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
        
        // URL History table
        {move || {
            let urls = stored_urls.get();
            if urls.is_empty() {
                None
            } else {
                Some(view! {
                    <div class="card">
                        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem;">
                            <h2 style="margin: 0;">"📊 Your URLs"</h2>
                            <button 
                                class="secondary" 
                                style="padding: 0.5rem 1rem; font-size: 0.75rem;"
                                on:click=move |_| refresh_stats()
                            >
                                "↻ Refresh Stats"
                            </button>
                        </div>
                        
                        <div class="url-history-table">
                            <table>
                                <thead>
                                    <tr>
                                        <th>"Short URL"</th>
                                        <th>"Original"</th>
                                        <th>"Clicks"</th>
                                        <th>"Created"</th>
                                        <th></th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {urls.into_iter().map(|url| {
                                        let short_url = url.short_url.clone();
                                        let short_display = url.short_url.clone();
                                        let original = if url.original_url.len() > 40 {
                                            format!("{}...", &url.original_url[..40])
                                        } else {
                                            url.original_url.clone()
                                        };
                                        let clicks = url.clicks;
                                        let created = format_relative_time(url.created_at);
                                        let code_for_delete = url.code.clone();
                                        
                                        view! {
                                            <tr>
                                                <td>
                                                    <a href=short_url target="_blank" style="color: var(--accent-primary);">
                                                        {short_display}
                                                    </a>
                                                </td>
                                                <td style="color: var(--text-secondary); font-size: 0.8rem;" title=url.original_url.clone()>
                                                    {original}
                                                </td>
                                                <td>
                                                    <span class="click-badge">{clicks}</span>
                                                </td>
                                                <td style="color: var(--text-secondary); font-size: 0.8rem;">
                                                    {created}
                                                </td>
                                                <td>
                                                    <button 
                                                        class="delete-btn"
                                                        on:click=move |_| delete_url(code_for_delete.clone())
                                                        title="Remove from history"
                                                    >
                                                        "✕"
                                                    </button>
                                                </td>
                                            </tr>
                                        }
                                    }).collect::<Vec<_>>()}
                                </tbody>
                            </table>
                        </div>
                        
                        <p class="storage-disclaimer">
                            "💾 URLs are stored locally in your browser. Clearing browser data will remove this history."
                        </p>
                    </div>
                })
            }
        }}
        
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

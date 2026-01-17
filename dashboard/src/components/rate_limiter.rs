//! Rate Limiter tab component

use leptos::prelude::*;
use crate::api;

#[component]
pub fn RateLimiterTab() -> impl IntoView {
    let (status, set_status) = signal::<Option<api::RateLimitStatus>>(None);
    let (last_response, set_last_response) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(false);
    let (rate_limited, set_rate_limited) = signal(false);
    
    // fetch status on mount
    Effect::new(move || {
        leptos::task::spawn_local(async move {
            if let Ok(s) = api::get_rate_status().await {
                set_status.set(Some(s));
            }
        });
    });
    
    // test request action
    let test_request = move |_| {
        set_loading.set(true);
        
        leptos::task::spawn_local(async move {
            match api::test_rate_limit().await {
                Ok(resp) => {
                    set_last_response.set(Some(format!("✅ {} (from {})", resp.message, resp.edge_location)));
                    set_rate_limited.set(false);
                }
                Err(e) => {
                    set_last_response.set(Some(format!("❌ {}", e)));
                    set_rate_limited.set(true);
                }
            }
            
            // refresh status
            if let Ok(s) = api::get_rate_status().await {
                set_status.set(Some(s));
            }
            
            set_loading.set(false);
        });
    };
    
    view! {
        <div class="card">
            <h2>"⏱️ Rate Limiter"</h2>
            <p style="color: var(--text-secondary); margin-bottom: 1rem; font-size: 0.875rem;">
                "Test edge-based rate limiting. This worker allows 10 requests per minute."
            </p>
            
            <button 
                on:click=test_request
                disabled=move || loading.get()
                class=move || if rate_limited.get() { "danger" } else { "" }
            >
                {move || if loading.get() {
                    view! { <span class="spinner"></span> " Testing..." }.into_any()
                } else {
                    view! { "Send Request" }.into_any()
                }}
            </button>
            
            // progress bar
            {move || status.get().map(|s| {
                let percent = (s.requests_remaining as f32 / s.limit as f32 * 100.0) as u32;
                let is_low = percent < 30;
                
                view! {
                    <div class="progress-container">
                        <div class="progress-bar">
                            <div 
                                class=if is_low { "progress-fill warning" } else { "progress-fill" }
                                style=format!("width: {}%", percent)
                            ></div>
                        </div>
                        <div class="progress-info">
                            <span>{s.requests_remaining}" / "{s.limit}" remaining"</span>
                            <span>"Reset in "{s.reset_in_seconds}"s"</span>
                        </div>
                    </div>
                }
            })}
            
            // last response
            {move || last_response.get().map(|resp| view! {
                <div class=if rate_limited.get() { "result error" } else { "result success" }>
                    <div class="result-value">{resp}</div>
                </div>
            })}
        </div>
        
        <div class="card">
            <h2>"Response Headers"</h2>
            <p style="color: var(--text-secondary); font-size: 0.875rem; line-height: 1.8;">
                "The rate limiter sets standard rate limit headers:"<br/>
                <code>"X-RateLimit-Limit"</code>" - Maximum requests per window"<br/>
                <code>"X-RateLimit-Remaining"</code>" - Requests left"<br/>
                <code>"X-RateLimit-Reset"</code>" - Seconds until reset"<br/>
                <code>"Retry-After"</code>" - Seconds to wait (when rate limited)"
            </p>
        </div>
    }
}

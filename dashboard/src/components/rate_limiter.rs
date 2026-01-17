//! Rate Limiter tab component

use leptos::prelude::*;
use crate::api;

#[component]
pub fn RateLimiterTab() -> impl IntoView {
    let (status, set_status) = signal::<Option<api::RateLimitStatus>>(None);
    let (last_response, set_last_response) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(false);
    let (rate_limited, set_rate_limited) = signal(false);
    
    // client-side countdown timer
    let (countdown, set_countdown) = signal::<u64>(0);
    // track if user has an active rate limit window
    let (has_active_window, set_has_active_window) = signal(false);
    
    // Note: We don't fetch status on mount to save API requests.
    // Status will populate on first "Send Request" click.
    
    // live countdown timer - uses try_get to safely handle disposed signals
    Effect::new(move || {
        use wasm_bindgen::prelude::*;
        use wasm_bindgen::JsCast;
        
        let window = web_sys::window().unwrap();
        
        let closure = Closure::<dyn Fn()>::new(move || {
            // use try_get_untracked to safely handle disposed signals (e.g. during hot reload)
            let Some(is_active) = has_active_window.try_get_untracked() else {
                return; // signal disposed, component unmounted
            };
            
            if !is_active {
                return;
            }
            
            let Some(current) = countdown.try_get_untracked() else {
                return; // signal disposed
            };
            
            if current > 1 {
                set_countdown.set(current - 1);
            } else if current == 1 {
                // countdown finished - immediately reset UI locally
                // set window inactive FIRST so view shows "Ready" immediately
                set_has_active_window.set(false);
                set_countdown.set(0);
                
                // update the status to show 10/10 immediately
                if let Some(s) = status.try_get_untracked().flatten() {
                    set_status.set(Some(api::RateLimitStatus {
                        client_id: s.client_id.clone(),
                        requests_made: 0,
                        requests_remaining: s.limit,
                        limit: s.limit,
                        reset_in_seconds: 0,
                    }));
                }
            }
            // when current == 0, we're already reset - do nothing
        });
        
        let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
            closure.as_ref().unchecked_ref(),
            1000,
        );
        
        // keep closure alive
        closure.forget();
    });
    
    // test request action
    let test_request = move |_| {
        set_loading.set(true);
        
        leptos::task::spawn_local(async move {
            match api::test_rate_limit().await {
                Ok(resp) => {
                    set_last_response.set(Some(format!("✅ {}", resp.message)));
                    set_rate_limited.set(false);
                }
                Err(e) => {
                    let error_lower = e.to_lowercase();
                    if error_lower.contains("failed to fetch") || error_lower.contains("load failed") || error_lower.contains("network") {
                        set_last_response.set(Some("🌐 Service unavailable - daily request limit may be exceeded. Try again tomorrow!".to_string()));
                    } else {
                        set_last_response.set(Some(format!("❌ {}", e)));
                    }
                    set_rate_limited.set(true);
                }
            }
            
            // refresh status and activate countdown
            if let Ok(s) = api::get_rate_status().await {
                set_countdown.set(s.reset_in_seconds);
                set_has_active_window.set(true);
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
            
            // progress bar with live countdown
            {move || status.get().map(|s| {
                let percent = (s.requests_remaining as f32 / s.limit as f32 * 100.0) as u32;
                let is_low = percent < 30;
                let current_countdown = countdown.get();
                let window_active = has_active_window.get();
                
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
                            {if window_active {
                                view! {
                                    <span class=if current_countdown <= 10 { "countdown-urgent" } else { "" }>
                                        "Reset in "{current_countdown}"s"
                                    </span>
                                }.into_any()
                            } else {
                                view! { <span class="countdown-ready">"Ready"</span> }.into_any()
                            }}
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

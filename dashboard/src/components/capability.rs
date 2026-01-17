//! Capability Explorer tab component

use leptos::prelude::*;
use crate::api;

#[component]
pub fn CapabilityTab() -> impl IntoView {
    let (results, set_results) = signal::<Vec<(String, Option<api::CapabilityResult>)>>(vec![
        ("fetch".to_string(), None),
        ("kv".to_string(), None),
        ("filesystem".to_string(), None),
        ("sockets".to_string(), None),
        ("subprocess".to_string(), None),
    ]);
    let (selected, set_selected) = signal::<Option<api::CapabilityResult>>(None);
    let (loading, set_loading) = signal::<Option<String>>(None);
    
    // test a capability
    let test = move |capability: String| {
        set_loading.set(Some(capability.clone()));
        
        let cap = capability.clone();
        leptos::task::spawn_local(async move {
            if let Ok(result) = api::test_capability(&cap).await {
                set_selected.set(Some(result.clone()));
                
                // update results list
                set_results.update(|list| {
                    for item in list.iter_mut() {
                        if item.0 == cap {
                            item.1 = Some(result.clone());
                        }
                    }
                });
            }
            set_loading.set(None);
        });
    };
    
    view! {
        <div class="card">
            <h2>"🔒 Capability Explorer"</h2>
            <p style="color: var(--text-secondary); margin-bottom: 1rem; font-size: 0.875rem;">
                "Test what Cloudflare Workers can and cannot do. Click a capability to test it live."
            </p>
            
            <div class="capability-grid">
                {move || results.get().into_iter().map(|(cap, result)| {
                    let cap_clone = cap.clone();
                    let is_loading = loading.get().as_ref() == Some(&cap);
                    let class = match &result {
                        Some(r) if r.allowed => "capability-btn allowed",
                        Some(_) => "capability-btn blocked",
                        None => "capability-btn",
                    };
                    
                    view! {
                        <button 
                            class=class
                            on:click=move |_| test(cap_clone.clone())
                            disabled=is_loading
                        >
                            {if is_loading {
                                view! { <span class="spinner"></span> }.into_any()
                            } else {
                                match &result {
                                    Some(r) if r.allowed => view! { "✅ " }.into_any(),
                                    Some(_) => view! { "❌ " }.into_any(),
                                    None => view! { "◯ " }.into_any(),
                                }
                            }}
                            {cap.clone()}
                        </button>
                    }
                }).collect::<Vec<_>>()}
            </div>
            
            // selected result detail
            {move || selected.get().map(|result| view! {
                <div class=if result.allowed { "result success" } else { "result error" }>
                    <div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.5rem;">
                        <span class=if result.allowed { "status allowed" } else { "status blocked" }>
                            {if result.allowed { "✅ ALLOWED" } else { "❌ BLOCKED" }}
                        </span>
                        <strong>{result.capability}</strong>
                    </div>
                    <div class="result-value" style="color: var(--text-secondary);">
                        {result.message}
                    </div>
                </div>
            })}
        </div>
        
        <div class="card">
            <h2>"Security Model"</h2>
            <p style="color: var(--text-secondary); font-size: 0.875rem; line-height: 1.8;">
                "Cloudflare Workers use the same "<strong>"capability-based security"</strong>" model as WASI. "
                "Code only gets access to what the runtime explicitly grants:"
            </p>
            <ul style="color: var(--text-secondary); font-size: 0.875rem; margin-top: 0.5rem; padding-left: 1.5rem;">
                <li><strong>"fetch()"</strong>" - HTTP requests ✅"</li>
                <li><strong>"KV Storage"</strong>" - When bound in config ✅"</li>
                <li><strong>"Filesystem"</strong>" - No access ❌"</li>
                <li><strong>"Raw Sockets"</strong>" - Only via fetch() ❌"</li>
                <li><strong>"Subprocess"</strong>" - No shell access ❌"</li>
            </ul>
        </div>
    }
}

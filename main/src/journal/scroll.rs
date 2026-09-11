//! Scroll-triggered reveal animations.
//!
//! Any element marked `data-reveal` in `components.rs` fades/slides into
//! place the first time it scrolls into the viewport. Implemented with a
//! native `IntersectionObserver` — no extra crates, same "web-sys + js-sys"
//! approach already used elsewhere in this app (e.g. `util::today` uses
//! `js_sys::Date` directly). The actual transition (opacity/transform) is
//! plain CSS in `index.html`; this module only flips the `is-visible` class.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{IntersectionObserver, IntersectionObserverInit};

/// Finds every `[data-reveal]` element currently in the DOM and observes it.
/// The first time one crosses into the viewport, `is-visible` is added to
/// its class list (which triggers the CSS transition) and that element is
/// then unobserved — a one-shot reveal, not a repeat-on-every-scroll effect.
///
/// Call once after the real data has rendered (see `Page`'s `use_effect` in
/// `mod.rs`). Elements that don't exist yet at call time are simply never
/// observed, so this is safe to call exactly once per full page render.
pub fn init_scroll_reveal() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Ok(elements) = document.query_selector_all("[data-reveal]") else {
        return;
    };
    if elements.length() == 0 {
        return;
    }

    let callback = Closure::wrap(Box::new(
        move |entries: js_sys::Array, observer: IntersectionObserver| {
            for entry in entries.iter() {
                let Ok(entry) = entry.dyn_into::<web_sys::IntersectionObserverEntry>() else {
                    continue;
                };
                if entry.is_intersecting() {
                    let target = entry.target();
                    let _ = target.class_list().add_1("is-visible");
                    observer.unobserve(&target);
                }
            }
        },
    ) as Box<dyn FnMut(js_sys::Array, IntersectionObserver)>);

    // Trigger a little before the element's bottom edge fully reaches the
    // viewport's bottom edge, and once ~10% of it is visible — feels timed
    // to the scroll instead of popping in right at the viewport's edge.
    let options = IntersectionObserverInit::new();
    options.set_root_margin("0px 0px -10% 0px");
    options.set_threshold(&JsValue::from_f64(0.1));

    let Ok(observer) = IntersectionObserver::new_with_options(callback.as_ref().unchecked_ref(), &options)
    else {
        return;
    };

    for idx in 0..elements.length() {
        if let Some(node) = elements.item(idx) {
            if let Ok(el) = node.dyn_into::<web_sys::Element>() {
                observer.observe(&el);
            }
        }
    }

    // Leak the closure so it stays alive for the observer's lifetime. There's
    // no natural teardown point for a page-wide scroll effect in this
    // single-page CSR app, so this is a deliberate, bounded (one closure)
    // leak rather than a bug.
    callback.forget();
}

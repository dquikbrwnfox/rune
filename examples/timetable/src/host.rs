//! The few things the browser provides that Rune doesn't: localStorage, the URL hash and file
//! downloads. Native builds keep nothing and write downloads into the working directory.

#[cfg(target_arch = "wasm32")]
mod imp {
    use wasm_bindgen::JsCast;

    fn storage() -> Option<web_sys::Storage> { web_sys::window()?.local_storage().ok()? }

    pub fn storage_get(key: &str) -> Option<String> { storage()?.get_item(key).ok()? }
    pub fn storage_set(key: &str, value: &str) { if let Some(st) = storage() { let _ = st.set_item(key, value); } }

    /// The URL hash without the leading `#`.
    pub fn hash() -> String {
        web_sys::window().and_then(|w| w.location().hash().ok()).unwrap_or_default().trim_start_matches('#').to_owned()
    }

    /// The page address without its hash.
    pub fn page_url() -> String {
        let href = web_sys::window().and_then(|w| w.location().href().ok()).unwrap_or_default();
        href.split('#').next().unwrap_or_default().to_owned()
    }

    pub fn clear_hash() {
        let Some(w) = web_sys::window() else { return };
        let loc = w.location();
        let path = format!("{}{}", loc.pathname().unwrap_or_default(), loc.search().unwrap_or_default());
        if let Ok(h) = w.history() { let _ = h.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&path)); }
    }

    pub fn download(name: &str, mime: &str, text: &str) {
        let go = || -> Option<()> {
            let parts = js_sys::Array::new();
            parts.push(&text.into());
            let opts = web_sys::BlobPropertyBag::new();
            opts.set_type(mime);
            let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &opts).ok()?;
            let url = web_sys::Url::create_object_url_with_blob(&blob).ok()?;
            let doc = web_sys::window()?.document()?;
            let a: web_sys::HtmlAnchorElement = doc.create_element("a").ok()?.dyn_into().ok()?;
            a.set_href(&url);
            a.set_download(name);
            let body = doc.body()?;
            body.append_child(&a).ok()?;
            a.click();
            body.remove_child(&a).ok()?;
            let _ = web_sys::Url::revoke_object_url(&url);
            Some(())
        };
        let _ = go();
    }

    pub fn log(msg: &str) { web_sys::console::log_1(&msg.into()); }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    pub fn storage_get(_: &str) -> Option<String> { None }
    pub fn storage_set(_: &str, _: &str) {}
    pub fn hash() -> String { std::env::var("TIMETABLE_HASH").unwrap_or_default().trim_start_matches('#').to_owned() }
    pub fn page_url() -> String { "https://timetable.example/".to_owned() }
    pub fn clear_hash() {}
    pub fn download(name: &str, _: &str, text: &str) { let _ = std::fs::write(name, text); }
    pub fn log(msg: &str) { eprintln!("{msg}"); }
}

pub use imp::*;

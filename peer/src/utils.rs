use cfg_if::cfg_if;
use wasm_bindgen::prelude::wasm_bindgen;

cfg_if! {
    // https://github.com/rustwasm/console_error_panic_hook#readme
    if #[cfg(feature = "console_error_panic_hook")] {
        pub(crate) use console_error_panic_hook::set_once as set_panic_hook;
    } else {
        #[inline]
        pub(crate) fn set_panic_hook() {}
    }
}

#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => ($crate::utils::log(&format_args!($($t)*).to_string()))
}
#[macro_export]
macro_rules! console_error {
    ($($t:tt)*) => ($crate::utils::error(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn log(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn error(s: &str);
}

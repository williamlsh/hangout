use cfg_if::cfg_if;

cfg_if! {
    // https://github.com/rustwasm/console_error_panic_hook#readme
    if #[cfg(feature = "console_error_panic_hook")] {
        pub(crate) use console_error_panic_hook::set_once as set_panic_hook;
    } else {
        #[inline]
        pub(crate) fn set_panic_hook() {}
    }
}

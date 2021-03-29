cfg_if::cfg_if! {
    if #[cfg(any(unix))] {
        mod unix;
        pub(crate) use self::unix::*;
    }
}

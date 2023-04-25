#[cfg(feature = "serde")]
mod serde;

#[macro_export]
macro_rules! dbg {
    () => {{
        #[cfg(feature = "std")]
        {
            ::std::dbg!()
        }
    }};
    ($val:expr $(,)?) => {{
        #[cfg(feature = "std")]
        {
            ::std::dbg!($val)
        }
    }};
    ($($val:expr),+ $(,)?) => {{
        #[cfg(feature = "std")]
        ::std::dbg!($($val),+)
    }};
}

#[macro_export]
macro_rules! println {
    () => {{
        #[cfg(feature = "std")]
        {
            ::std::prinltln!()
        }
    }};
    ($($arg:tt)*) => {{
        #[cfg(feature = "std")]
        ::std::println!($($arg)*)
    }};
}

use std::env;

// to enable debug output, run the program with DEBUG_MODE=true cargo run
pub fn is_debug_mode() -> bool {
    env::var("DEBUG_MODE").unwrap_or_else(|_| "false".to_string()) == "true"
}

#[macro_export]
macro_rules! debug_print {
    ($($arg:tt)*) => {
        if crate::is_debug_mode() {
            println!($($arg)*);
        }
    };
}
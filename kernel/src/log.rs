#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        println!("\x1b[32m[INFO] {}\x1b[0m", format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_warning {
    ($($arg:tt)*) => {
        println!("\x1b[33m[WARNING] {}\x1b[0m", format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        println!("\x1b[31m[ERROR] {}\x1b[0m", format_args!($($arg)*));
    };
}

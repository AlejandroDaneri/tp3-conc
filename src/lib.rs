#[macro_use]
#[allow(unused_macros)]
pub mod macros {
    macro_rules! print_fmt {
        ($suffix:expr, $fmt:expr) => (
            println!(concat!($suffix, " ]\x1B[0m(", file!(), ":", line!(), ")\x1B[0m ", $fmt))
        );
        ($suffix:expr, $fmt:expr, $($arg:tt)*) => (
            println!(concat!($suffix, " ]\x1B[0m(", file!(), ":", line!(), ")\x1B[0m ", $fmt), $($arg)*)
        );
    }
    macro_rules! debug {
        ($fmt:expr) => (print_fmt!("\x1B[2m[DEBUG ", $fmt));
        ($fmt:expr, $($arg:tt)*) => (print_fmt!("\x1B[2m[DEBUG ", $fmt, $($arg)*));
    }
    macro_rules! info {
        ($fmt:expr) => (print_fmt!("\x1B[92m[INFO ", $fmt));
        ($fmt:expr, $($arg:tt)*) => (print_fmt!("\x1B[92m[INFO ", $fmt, $($arg)*));
    }
    macro_rules! warn {
        ($fmt:expr) => (print_fmt!("\x1B[93m[WARN ", $fmt));
        ($fmt:expr, $($arg:tt)*) => (print_fmt!("\x1B[93m[WARN ", $fmt, $($arg)*));
}
    macro_rules! error {
        ($fmt:expr) => (print_fmt!("\x1B[91m[ERROR", $fmt));
        ($fmt:expr, $($arg:tt)*) => (print_fmt!("\x1B[91m[ERROR", $fmt, $($arg)*));
}
}

pub mod blockchain;
pub mod communication;
pub mod handler;

#[macro_use]
#[allow(unused_macros)]
pub mod macros {
    macro_rules! debug {
        ($fmt:expr) => (println!(concat!("\x1B[2m[DEBUG]\x1B[0m ", file!(), ":", line!(), " ", $fmt)));
        ($fmt:expr, $($arg:tt)*) => (println!(concat!("\x1B[2m[DEBUG]\x1B[0m ", file!(), ":", line!(), " ", $fmt), $($arg)*));
    }
    macro_rules! info {
        ($fmt:expr) => (println!(concat!("\x1B[92m[INFO ]\x1B[0m ", file!(), ":", line!(), " ", $fmt)));
        ($fmt:expr, $($arg:tt)*) => (println!(concat!("\x1B[92m[INFO ]\x1B[0m ", file!(), ":", line!(), " ", $fmt), $($arg)*));
    }
    macro_rules! warn {
        ($fmt:expr) => (println!(concat!("\x1B[93m[WARN ]\x1B[0m ", file!(), ":", line!(), " ", $fmt)));
        ($fmt:expr, $($arg:tt)*) => (println!(concat!("\x1B[93m[WARN ]\x1B[0m ", file!(), ":", line!(), " ", $fmt), $($arg)*));
    }
    macro_rules! error {
        ($fmt:expr) => (println!(concat!("\x1B[91m[ERROR]\x1B[0m ", file!(), ":", line!(), " ", $fmt)));
        ($fmt:expr, $($arg:tt)*) => (println!(concat!("\x1B[91m[ERROR]\x1B[0m ", file!(), ":", line!(), " ", $fmt), $($arg)*));
    }
}

pub mod blockchain;
pub mod communication;
pub mod handler;

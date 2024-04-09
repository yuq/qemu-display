#[macro_export]
macro_rules! cast {
    ($value:expr) => {
        match $value.try_into() {
            Ok(val) => val,
            Err(err) => {
                eprintln!("Error casting value: {}", err);
                return;
            }
        }
    };
}

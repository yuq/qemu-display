use ironrdp::server::PixelFormat;

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

pub(crate) struct PixmanFormat(pub u32);

#[cfg(target_endian = "little")]
impl TryFrom<PixmanFormat> for PixelFormat {
    type Error = ();

    fn try_from(value: PixmanFormat) -> Result<Self, Self::Error> {
        use pixman_sys::*;

        #[allow(non_upper_case_globals)]
        match value.0 {
            pixman_format_code_t_PIXMAN_x8r8g8b8 => Ok(PixelFormat::BgrX32),
            _ => Err(()),
        }
    }
}

#[cfg(target_endian = "big")]
impl TryFrom<PixmanFormat> for PixelFormat {
    type Error = ();

    fn try_from(value: PixmanFormat) -> Result<Self, Self::Error> {
        use pixman_sys::*;

        #[allow(non_upper_case_globals)]
        match value.0 {
            pixman_format_code_t_PIXMAN_x8r8g8b8 => Ok(PixelFormat::XRgb32),
            _ => Err(()),
        }
    }
}

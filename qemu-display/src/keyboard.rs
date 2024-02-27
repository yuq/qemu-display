use enumflags2::{bitflags, BitFlags};
use serde::{Deserialize, Serialize};
use zbus::zvariant::Type;

#[bitflags]
#[repr(u32)]
#[derive(Type, Debug, PartialEq, Copy, Clone, Eq, Serialize, Deserialize)]
pub enum KeyboardModifiers {
    Scroll = 0x1,
    Num = 0x2,
    Caps = 0x4,
}

#[zbus::proxy(default_service = "org.qemu", interface = "org.qemu.Display1.Keyboard")]
pub trait Keyboard {
    /// Press method
    fn press(&self, keycode: u32) -> zbus::Result<()>;

    /// Release method
    fn release(&self, keycode: u32) -> zbus::Result<()>;

    #[zbus(property)]
    fn modifiers(&self) -> zbus::Result<BitFlags<KeyboardModifiers>>;
}

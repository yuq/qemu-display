use anyhow::Result;
use ironrdp::{connector::DesktopSize, server::RdpServerDisplayUpdates};
use qemu_display::{zbus, Console, ConsoleListenerHandler, Cursor, MouseSet, Scanout, Update};

use ironrdp::server::{BitmapUpdate, DisplayUpdate, PixelFormat, PixelOrder, RdpServerDisplay};

use crate::cast;

pub struct DisplayHandler {
    console: Console,
}

struct DisplayUpdates {
    receiver: tokio::sync::mpsc::Receiver<DisplayUpdate>,
}

impl DisplayHandler {
    pub async fn connect(dbus: zbus::Connection) -> Result<Self> {
        let console = Console::new(&dbus, 0).await?;

        Ok(Self { console })
    }

    async fn listen(&self) -> Result<DisplayUpdates> {
        let (sender, receiver) = tokio::sync::mpsc::channel::<DisplayUpdate>(32);
        let (width, height) = (
            self.console.width().await? as _,
            self.console.height().await? as _,
        );
        let desktop_size = DesktopSize { width, height };
        let listener = Listener::new(sender, desktop_size);
        self.console.unregister_listener();
        self.console.register_listener(listener).await?;

        Ok(DisplayUpdates { receiver })
    }
}

#[async_trait::async_trait]
impl RdpServerDisplayUpdates for DisplayUpdates {
    async fn next_update(&mut self) -> Option<DisplayUpdate> {
        self.receiver.recv().await
    }
}

#[async_trait::async_trait]
impl RdpServerDisplay for DisplayHandler {
    async fn size(&mut self) -> DesktopSize {
        let width = self.console.proxy.width().await.unwrap() as u16;
        let height = self.console.proxy.height().await.unwrap() as u16;
        DesktopSize { height, width }
    }

    async fn updates(&mut self) -> Result<Box<dyn RdpServerDisplayUpdates>> {
        Ok(Box::new(self.listen().await?))
    }
}

struct Listener {
    sender: tokio::sync::mpsc::Sender<DisplayUpdate>,
}

impl Listener {
    fn new(sender: tokio::sync::mpsc::Sender<DisplayUpdate>, _desktop_size: DesktopSize) -> Self {
        Self { sender }
    }

    async fn send(&mut self, update: DisplayUpdate) {
        if let Err(e) = self.sender.send(update).await {
            println!("{:?}", e);
        };
    }
}

#[async_trait::async_trait]
impl ConsoleListenerHandler for Listener {
    async fn scanout(&mut self, scanout: Scanout) {
        let format = match scanout.format {
            537_004_168 => PixelFormat::BgrA32,
            _ => PixelFormat::RgbA32,
        };

        let width: u16 = cast!(scanout.width);
        let height: u16 = cast!(scanout.height);
        let bitmap = DisplayUpdate::Bitmap(BitmapUpdate {
            top: 0,
            left: 0,
            width: width.try_into().unwrap(),
            height: height.try_into().unwrap(),
            format,
            order: PixelOrder::TopToBottom,
            data: scanout.data,
        });

        self.send(bitmap).await;
    }

    async fn update(&mut self, update: Update) {
        let format = match update.format {
            537_004_168 => PixelFormat::BgrA32,
            _ => PixelFormat::RgbA32,
        };

        let width: u16 = cast!(update.w);
        let height: u16 = cast!(update.h);
        let bitmap = DisplayUpdate::Bitmap(BitmapUpdate {
            // TODO: fix scary conversion
            top: cast!(update.y),
            left: cast!(update.x),
            width: width.try_into().unwrap(),
            height: height.try_into().unwrap(),
            format,
            order: PixelOrder::TopToBottom,
            data: update.data,
        });

        self.send(bitmap).await;
    }

    #[cfg(unix)]
    async fn scanout_dmabuf(&mut self, _scanout: qemu_display::ScanoutDMABUF) {}

    #[cfg(unix)]
    async fn update_dmabuf(&mut self, _update: qemu_display::UpdateDMABUF) {}

    async fn disable(&mut self) {}
    async fn mouse_set(&mut self, _set: MouseSet) {}
    async fn cursor_define(&mut self, _cursor: Cursor) {}
    fn disconnected(&mut self) {}
    fn interfaces(&self) -> Vec<String> {
        vec![]
    }
}

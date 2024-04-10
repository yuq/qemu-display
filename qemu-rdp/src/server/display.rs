use anyhow::Result;
use ironrdp::{connector::DesktopSize, server::RdpServerDisplayUpdates};
use qemu_display::{zbus, Console, ConsoleListenerHandler, Cursor, MouseSet, Scanout, Update};

use ironrdp::server::{BitmapUpdate, DisplayUpdate, PixelOrder, RdpServerDisplay};

use crate::{cast, util::PixmanFormat};

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
    _desktop_size: DesktopSize,
}

impl Listener {
    fn new(sender: tokio::sync::mpsc::Sender<DisplayUpdate>, _desktop_size: DesktopSize) -> Self {
        Self {
            sender,
            _desktop_size,
        }
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
        let desktop_size = DesktopSize {
            width: cast!(scanout.width),
            height: cast!(scanout.height),
        };

        tracing::debug!(?desktop_size);

        // if desktop_size != self.desktop_size {
        //     self.desktop_size = desktop_size;
        //     self.send(DisplayUpdate::Resize(desktop_size)).await;
        // }

        self.update(Update {
            x: 0,
            y: 0,
            w: cast!(scanout.width),
            h: cast!(scanout.height),
            stride: scanout.stride,
            format: scanout.format,
            data: scanout.data,
        })
        .await
    }

    async fn update(&mut self, update: Update) {
        let Ok(format) = PixmanFormat(update.format).try_into() else {
            println!("Unhandled format {}", update.format);
            return;
        };

        let width: u16 = cast!(update.w);
        let height: u16 = cast!(update.h);

        let bitmap = DisplayUpdate::Bitmap(BitmapUpdate {
            // TODO: fix scary conversion
            left: cast!(update.x),
            top: cast!(update.y),
            width: cast!(width),
            height: cast!(height),
            format,
            order: PixelOrder::TopToBottom,
            data: update.data,
        });

        self.send(bitmap).await;
    }

    #[cfg(unix)]
    async fn scanout_dmabuf(&mut self, scanout: qemu_display::ScanoutDMABUF) {
        tracing::debug!(?scanout);
    }

    #[cfg(unix)]
    async fn update_dmabuf(&mut self, update: qemu_display::UpdateDMABUF) {
        tracing::debug!(?update);
    }

    async fn disable(&mut self) {
        tracing::debug!("disable");
    }

    fn disconnected(&mut self) {
        tracing::debug!("console listener disconnected");
    }

    async fn mouse_set(&mut self, _set: MouseSet) {}

    async fn cursor_define(&mut self, _cursor: Cursor) {}

    fn interfaces(&self) -> Vec<String> {
        vec![]
    }
}

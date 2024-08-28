use std::thread;

use anyhow::Result;
use qemu_display::{Console, ConsoleListenerHandler, Cursor, Display, MouseSet, Scanout, Update};

use ironrdp::{
    connector::DesktopSize,
    displaycontrol::pdu::DisplayControlMonitorLayout,
    server::{
        BitmapUpdate, DisplayUpdate, PixelOrder, RGBAPointer, RdpServerDisplay,
        RdpServerDisplayUpdates,
    },
};

use crate::{cast, utils::PixmanFormat};

mod queue;
use queue::DisplayQueue;

pub struct DisplayHandler {
    console: Console,
}

struct DisplayUpdates {
    receiver: queue::Receiver,
}

impl DisplayHandler {
    pub async fn connect(display: &Display<'_>) -> Result<Self> {
        let console = Console::new(display.connection(), 0).await?;

        Ok(Self { console })
    }

    async fn listen(&self) -> Result<DisplayUpdates> {
        let (sender, receiver) = DisplayQueue::new();
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

    fn request_layout(&mut self, layout: DisplayControlMonitorLayout) {
        let console = self.console.proxy.clone();
        // TODO: use a queue and a dedicated task/thread for requests, to preserve order
        thread::spawn(move || {
            // TODO: multi-monitor
            let Some(monitor) = layout.monitors().first() else {
                return;
            };
            let (xoff, yoff) = monitor.position().unwrap_or_default();
            let (width, height) = monitor.dimensions();
            let (width_mm, height_mm) = monitor.physical_dimensions().unwrap_or_default();
            crate::utils::block_on(async move {
                let _ = console
                    .set_ui_info(width_mm as u16, height_mm as u16, xoff, yoff, width, height)
                    .await;
            });
        });
    }
}

struct Listener {
    sender: queue::Sender,
    desktop_size: DesktopSize,
    cursor_hot: (i32, i32),
}

impl Listener {
    fn new(sender: queue::Sender, desktop_size: DesktopSize) -> Self {
        Self {
            sender,
            desktop_size,
            cursor_hot: (0, 0),
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

        if desktop_size != self.desktop_size {
            self.desktop_size = desktop_size;
            self.send(DisplayUpdate::Resize(desktop_size)).await;
        }

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
            stride: cast!(update.stride),
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

    async fn mouse_set(&mut self, set: MouseSet) {
        tracing::debug!(?set);

        // FIXME: this create weird effects on the client
        //
        // self.send(DisplayUpdate::PointerPosition(ironrdp_pdu::PointerPositionAttribute {
        //     x: cast!(set.x + self.cursor_hot.0),
        //     y: cast!(set.y + self.cursor_hot.1),
        // }))
        // .await;
        //

        if set.on == 0 {
            self.send(DisplayUpdate::HidePointer).await;
        }
    }

    async fn cursor_define(&mut self, cursor: Cursor) {
        tracing::debug!(?cursor);
        self.cursor_hot = (cursor.hot_x, cursor.hot_y);

        fn flip_vertically(image: Vec<u8>, width: usize, height: usize) -> Vec<u8> {
            let row_length = width * 4; // 4 bytes per pixel
            let mut flipped_image = vec![0; image.len()]; // Initialize a vector for the flipped image

            for y in 0..height {
                let source_row_start = y * row_length;
                let dest_row_start = (height - 1 - y) * row_length;

                // Copy the whole row from the source position to the destination position
                flipped_image[dest_row_start..dest_row_start + row_length]
                    .copy_from_slice(&image[source_row_start..source_row_start + row_length]);
            }

            flipped_image
        }

        let data = flip_vertically(cursor.data, cast!(cursor.width), cast!(cursor.height));

        self.send(DisplayUpdate::RGBAPointer(RGBAPointer {
            width: cast!(cursor.width),
            height: cast!(cursor.height),
            hot_x: cast!(cursor.hot_x),
            hot_y: cast!(cursor.hot_y),
            data,
        }))
        .await;
    }

    fn disconnected(&mut self) {
        tracing::debug!("console listener disconnected");
    }

    fn interfaces(&self) -> Vec<String> {
        vec![]
    }
}

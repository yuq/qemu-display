mod clipboard;
mod display;
mod input;
mod sound;

use anyhow::Error;
use enumflags2::BitFlags;
use ironrdp::server::{Credentials, ServerEvent, TlsIdentityCtx};

use qemu_display::{zbus, Display};
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

use ironrdp::server::RdpServer;

use crate::args::ServerArgs;

use clipboard::ClipboardHandler;
use display::DisplayHandler;
use input::InputHandler;
use sound::SoundHandler;

pub struct Server {
    dbus: zbus::Connection,
    args: ServerArgs,
}

struct DBusCtrl {
    ev: UnboundedSender<ServerEvent>,
}

impl Server {
    pub fn new(dbus: zbus::Connection, args: ServerArgs) -> Self {
        Self { dbus, args }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        let dbus_display = Display::new::<()>(&self.dbus, None).await?;

        let handler = InputHandler::connect(&dbus_display).await?;
        let display = DisplayHandler::connect(&dbus_display).await?;
        let clipboard = ClipboardHandler::connect(&dbus_display).await?;
        let sound = match SoundHandler::connect(&dbus_display).await {
            Ok(h) => Some(h),
            Err(e) => {
                debug!("Can't connect audio: {}", e);
                None
            }
        };

        let tls =
            TlsIdentityCtx::init_from_paths(self.args.cert.as_path(), self.args.key.as_path())?;
        let mut server = RdpServer::builder()
            .with_addr(self.args.bind_addr)
            .with_hybrid(tls.make_acceptor()?, tls.pub_key)
            .with_input_handler(handler)
            .with_display_handler(display)
            .with_cliprdr_factory(Some(Box::new(clipboard)))
            .with_sound_factory(sound.map(|h| Box::new(h) as _))
            .with_remote_fx(self.args.remotefx.enabled())
            .build();

        let ev = server.event_sender().clone();
        let proxy = dbus_display.inner_proxy().clone();
        tokio::spawn(async move {
            use futures_util::StreamExt;

            let mut owner_changed = proxy.receive_owner_changed().await.unwrap();
            let _ = owner_changed.next().await;
            ev.send(ServerEvent::Quit("org.qemu is gone".to_owned()))
                .unwrap();
        });

        let ev = server.event_sender().clone();
        self.dbus
            .object_server()
            .at("/org/qemu_display/rdp", DBusCtrl { ev })
            .await?;
        self.dbus
            .request_name_with_flags("org.QemuDisplay", BitFlags::EMPTY)
            .await?;

        server.run().await
    }
}

#[zbus::interface(name = "org.QemuDisplay.RDP")]
impl DBusCtrl {
    async fn set_credentials(&self, username: &str, password: &str, domain: &str) {
        self.ev
            .send(ServerEvent::SetCredentials(Credentials {
                username: username.into(),
                password: password.into(),
                domain: if domain.is_empty() {
                    None
                } else {
                    Some(domain.into())
                },
            }))
            .unwrap();
    }
}

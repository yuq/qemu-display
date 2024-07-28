mod clipboard;
mod display;
mod input;
mod sound;

use anyhow::{anyhow, Context, Error};
use ironrdp::server::{
    tokio_rustls::{rustls, TlsAcceptor},
    ServerEvent,
};

use qemu_display::{zbus, Display};
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::{fs::File, io::BufReader, sync::Arc};
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

impl Server {
    pub fn new(dbus: zbus::Connection, args: ServerArgs) -> Self {
        Self { dbus, args }
    }

    pub async fn run(&mut self) -> Result<(), Error> {
        let tls = self
            .args
            .cert
            .as_ref()
            .zip(self.args.key.as_ref())
            .map(|(cert, key)| acceptor(cert, key).unwrap())
            .ok_or_else(|| anyhow!("Failed to setup TLS"))?;

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

        let mut server = RdpServer::builder()
            .with_addr((self.args.address, self.args.port))
            .with_tls(tls)
            .with_input_handler(handler)
            .with_display_handler(display)
            .with_cliprdr_factory(Some(Box::new(clipboard)))
            .with_sound_factory(sound.map(|h| Box::new(h) as _))
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
        server.run().await
    }
}

fn acceptor(cert_path: &str, key_path: &str) -> Result<TlsAcceptor, Error> {
    let cert = certs(&mut BufReader::new(File::open(cert_path)?))
        .next()
        .context("no certificate")??;
    let key = pkcs8_private_keys(&mut BufReader::new(File::open(key_path)?))
        .next()
        .context("no private key")?
        .map(rustls::pki_types::PrivateKeyDer::from)?;

    let mut server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .expect("bad certificate/key");

    // This adds support for the SSLKEYLOGFILE env variable (https://wiki.wireshark.org/TLS#using-the-pre-master-secret)
    server_config.key_log = Arc::new(rustls::KeyLogFile::new());

    Ok(TlsAcceptor::from(Arc::new(server_config)))
}

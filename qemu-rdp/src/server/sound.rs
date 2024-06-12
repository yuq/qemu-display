use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use anyhow::Result;
// use tracing::{debug, error, warn};
use ironrdp::{
    rdpsnd::pdu,
    server::{
        RdpsndServerHandler, RdpsndServerMessage, ServerEvent, ServerEventSender,
        SoundServerFactory,
    },
};
use tokio::sync::mpsc;

use qemu_display::{zbus, Audio, AudioOutHandler, PCMInfo, Volume};
use tracing::{debug, warn};

#[derive(Debug, PartialEq)]
enum State {
    Init,
    Ready(u64, pdu::AudioFormat),
}

#[derive(Debug)]
pub struct Inner {
    audio: Audio,
    state: State,
    start_time: Instant,
    ev_sender: Option<mpsc::UnboundedSender<ServerEvent>>,
    rdp_started: bool,
}

#[derive(Debug)]
pub(crate) struct DBusHandler {
    inner: Arc<Mutex<Inner>>,
}

fn wave_format_from(info: &PCMInfo) -> Result<pdu::AudioFormat, &str> {
    if info.be {
        return Err("unsupported big-endian audio format");
    }

    if info.is_float {
        return Err("unsupported float audio format");
    }

    let n_block_align = match info.bytes_per_frame.try_into() {
        Ok(bpf) => bpf,
        Err(_) => {
            return Err("unsupported audio bytes_per_frame");
        }
    };

    Ok(pdu::AudioFormat {
        format: pdu::WaveFormat::PCM,
        n_channels: info.nchannels.into(),
        n_samples_per_sec: info.freq,
        n_avg_bytes_per_sec: info.bytes_per_second,
        n_block_align,
        bits_per_sample: info.bits.into(),
        data: None,
    })
}

#[async_trait::async_trait]
impl AudioOutHandler for DBusHandler {
    async fn init(&mut self, id: u64, info: PCMInfo) {
        let mut inner = self.inner.lock().unwrap();

        if inner.state != State::Init {
            warn!("audio already initialized, no mixing yet");
            return;
        }
        let format = match wave_format_from(&info) {
            Ok(fmt) => fmt,
            Err(err) => {
                warn!("{}", err);
                return;
            }
        };

        inner.state = State::Ready(id, format);
        debug!(?inner.state);
    }

    async fn fini(&mut self, id: u64) {
        let mut inner = self.inner.lock().unwrap();

        if matches!(inner.state, State::Ready(iid, _) if iid == id) {
            inner.state = State::Init;
            debug!(?inner.state);
        }
    }

    async fn set_enabled(&mut self, id: u64, enabled: bool) {
        debug!(?id, ?enabled)
    }

    async fn set_volume(&mut self, id: u64, volume: Volume) {
        debug!(?id, ?volume)
    }

    async fn write(&mut self, id: u64, data: Vec<u8>) {
        let inner = self.inner.lock().unwrap();

        if !inner.rdp_started || !matches!(inner.state, State::Ready(iid, _) if iid == id) {
            return;
        }

        if let Some(sender) = inner.ev_sender.as_ref() {
            let ts = inner.start_time.elapsed().as_millis() as _;
            let _ = sender.send(ServerEvent::Rdpsnd(RdpsndServerMessage::Wave(data, ts)));
        }
    }
}

#[derive(Clone, Debug)]
pub struct SoundHandler {
    inner: Arc<Mutex<Inner>>,
}

impl ServerEventSender for SoundHandler {
    fn set_sender(&mut self, sender: mpsc::UnboundedSender<ServerEvent>) {
        let mut inner = self.inner.lock().unwrap();

        inner.ev_sender = Some(sender);
    }
}

#[derive(Debug)]
pub(crate) struct RDPSndHandler {
    inner: Arc<Mutex<Inner>>,
}

impl RdpsndServerHandler for RDPSndHandler {
    fn get_formats(&self) -> &[pdu::AudioFormat] {
        &[pdu::AudioFormat {
            format: pdu::WaveFormat::PCM,
            n_channels: 2,
            n_samples_per_sec: 48000,
            n_avg_bytes_per_sec: 192000,
            n_block_align: 4,
            bits_per_sample: 16,
            data: None,
        }]
    }

    fn start(&mut self, _client_format: &pdu::ClientAudioFormatPdu) -> Option<u16> {
        let mut inner = self.inner.lock().unwrap();
        inner.rdp_started = true;
        Some(0)
    }

    fn stop(&mut self) {
        let mut inner = self.inner.lock().unwrap();
        inner.rdp_started = false;
    }
}

impl SoundServerFactory for SoundHandler {
    fn build_backend(&self) -> Box<dyn RdpsndServerHandler> {
        Box::new(RDPSndHandler {
            inner: self.inner.clone(),
        })
    }
}

impl SoundHandler {
    pub async fn connect<D>(dbus: zbus::Connection, dest: Option<D>) -> Result<Self>
    where
        D: TryInto<zbus::names::BusName<'static>>,
        D::Error: Into<qemu_display::Error>,
    {
        let audio = Audio::new(&dbus, dest).await?;
        let inner = Arc::new(Mutex::new(Inner {
            start_time: Instant::now(),
            state: State::Init,
            ev_sender: None,
            audio,
            rdp_started: false,
        }));

        let handler = DBusHandler {
            inner: inner.clone(),
        };
        inner
            .lock()
            .unwrap()
            .audio
            .register_out_listener(handler)
            .await?;

        Ok(Self { inner })
    }
}

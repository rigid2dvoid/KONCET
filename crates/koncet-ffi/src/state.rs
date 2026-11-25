//! Process-wide bridge state, ticker, and stream fan-out.
//!
//! The database and the single active session live behind one mutex. The
//! ticker advances the session, persists the batch, then publishes the same
//! committed data to subscribers. Streams never receive unpersisted output.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use koncet_app::{App, AppResult, RunUpdate};
use koncet_domain::RunId;
use tracing_subscriber::EnvFilter;

use crate::dto::{LogBatchDto, MetricBatchDto, RunUpdateDto, SessionFrameDto};
use crate::error::{guard, BridgeError, BridgeResult};
use crate::frb_generated::StreamSink;

const TICK_MS: u64 = 100;

pub(crate) struct Bridge {
    pub(crate) app: Option<App>,
    pub(crate) hub: Arc<Hub>,
    initialised: bool,
}

struct Subscription<T> {
    run_id: String,
    sender: Sender<T>,
}

pub(crate) struct Hub {
    session: Mutex<Vec<Subscription<SessionFrameDto>>>,
    logs: Mutex<Vec<Subscription<LogBatchDto>>>,
    metrics: Mutex<Vec<Subscription<MetricBatchDto>>>,
}

impl Hub {
    fn new() -> Self {
        Self {
            session: Mutex::new(Vec::new()),
            logs: Mutex::new(Vec::new()),
            metrics: Mutex::new(Vec::new()),
        }
    }

    fn clear(&self) {
        lock(&self.session).clear();
        lock(&self.logs).clear();
        lock(&self.metrics).clear();
    }
}

static BRIDGE: LazyLock<Mutex<Bridge>> = LazyLock::new(|| {
    Mutex::new(Bridge {
        app: None,
        hub: Arc::new(Hub::new()),
        initialised: false,
    })
});

static TICKER_STARTED: AtomicBool = AtomicBool::new(false);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(crate) fn lock_bridge() -> MutexGuard<'static, Bridge> {
    lock(&BRIDGE)
}

pub(crate) fn hub() -> Arc<Hub> {
    lock_bridge().hub.clone()
}

/// Installs an opened app and starts the wall-clock ticker once per process.
pub(crate) fn install(app: App) -> BridgeResult<()> {
    init_tracing();
    {
        let mut bridge = lock_bridge();
        bridge.app = Some(app);
        bridge.initialised = true;
    }
    ensure_ticker();
    Ok(())
}

fn init_tracing() {
    static TRACING: std::sync::Once = std::sync::Once::new();
    TRACING.call_once(|| {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("warn,koncet=info"));
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .try_init();
    });
}

/// Runs a use case against the installed app, mapping failures and panics.
pub(crate) fn with_app<T>(f: impl FnOnce(&mut App) -> AppResult<T>) -> BridgeResult<T> {
    guard(|| {
        let mut bridge = lock_bridge();
        let app = bridge
            .app
            .as_mut()
            .ok_or_else(BridgeError::not_initialised)?;
        f(app).map_err(BridgeError::from)
    })
}

/// Removes the app for shutdown. Returns `None` when already shut down.
pub(crate) fn take_app() -> BridgeResult<Option<App>> {
    let mut bridge = lock_bridge();
    if !bridge.initialised && bridge.app.is_none() {
        return Ok(None);
    }
    bridge.initialised = false;
    let app = bridge.app.take();
    bridge.hub.clear();
    Ok(app)
}

pub(crate) fn active_run_id() -> Option<RunId> {
    let bridge = lock_bridge();
    bridge.app.as_ref().and_then(App::active_run_id)
}

pub(crate) fn active_frame() -> Option<koncet_app::RunFrame> {
    let bridge = lock_bridge();
    bridge.app.as_ref().and_then(App::active_frame)
}

fn ensure_ticker() {
    if TICKER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }
    let _ = std::thread::Builder::new()
        .name("koncet-engine-ticker".to_string())
        .spawn(ticker_loop);
}

fn ticker_loop() {
    let mut last = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(TICK_MS));
        let elapsed = last.elapsed().as_millis().min(2_000) as u64;
        last = Instant::now();
        if elapsed == 0 {
            continue;
        }

        let published = {
            let mut bridge = lock_bridge();
            let Some(app) = bridge.app.as_mut() else {
                continue;
            };
            if app.active_run_id().is_none() {
                continue;
            }
            match app.advance_active(elapsed) {
                Ok(Some(update)) => Some((update, bridge.hub.clone())),
                Ok(None) => None,
                Err(error) => {
                    tracing::error!(
                        code = error.code(),
                        message = %error.message(),
                        "simulation tick failed; will retry"
                    );
                    None
                }
            }
        };

        if let Some((update, hub)) = published {
            publish_update(&hub, &update.into());
        }
    }
}

/// Publishes a committed update to all matching subscribers.
pub(crate) fn publish_update(hub: &Hub, update: &RunUpdateDto) {
    let run_id = update.frame.run.id.clone();
    let finalised = update.finalised;

    let frame = SessionFrameDto {
        run_id: run_id.clone(),
        frame: update.frame.clone(),
        events: update.events.clone(),
        watermark_seq: update.frame.run.last_event_seq,
        finalised,
    };
    send(&hub.session, &run_id, frame);

    if !update.logs.is_empty() || finalised {
        let watermark_seq = update.logs.last().map(|log| log.seq).unwrap_or(0);
        let batch = LogBatchDto {
            run_id: run_id.clone(),
            logs: update.logs.clone(),
            watermark_seq,
            finalised,
        };
        send(&hub.logs, &run_id, batch);
    }

    if !update.metrics.is_empty() || finalised {
        let watermark_ms = update
            .metrics
            .last()
            .map(|metric| metric.sim_time_ms)
            .unwrap_or(update.frame.clock_ms);
        let batch = MetricBatchDto {
            run_id: run_id.clone(),
            metrics: update.metrics.clone(),
            watermark_ms,
            finalised,
        };
        send(&hub.metrics, &run_id, batch);
    }
}

fn send<T: Clone>(subscriptions: &Mutex<Vec<Subscription<T>>>, run_id: &str, item: T) {
    let mut subscriptions = lock(subscriptions);
    subscriptions.retain_mut(|subscription| {
        if subscription.run_id != run_id {
            return true;
        }
        subscription.sender.send(item.clone()).is_ok()
    });
}

/// Registers a session subscriber and queues the current snapshot first.
pub(crate) fn subscribe_session(
    run_id: RunId,
    sink: StreamSink<SessionFrameDto>,
) -> BridgeResult<()> {
    let run_id_text = run_id.to_string();
    let (sender, receiver) = mpsc::channel();
    let snapshot = {
        let bridge = lock_bridge();
        let app = bridge
            .app
            .as_ref()
            .ok_or_else(BridgeError::not_initialised)?;
        match app.active_frame() {
            Some(frame) if frame.run.id == run_id => {
                let item = SessionFrameDto {
                    run_id: run_id_text.clone(),
                    watermark_seq: frame.run.last_event_seq,
                    finalised: frame.run.status.is_terminal(),
                    frame: (&frame).into(),
                    events: Vec::new(),
                };
                let _ = sender.send(item);
                Some(bridge.hub.clone())
            }
            _ => {
                return Err(BridgeError::new(
                    "run_not_active",
                    "the run is not active in this process",
                    "run",
                    false,
                ))
            }
        }
    };
    if let Some(hub) = snapshot {
        lock(&hub.session).push(Subscription {
            run_id: run_id_text,
            sender,
        });
    }
    spawn_session_forwarder(sink, receiver);
    Ok(())
}

pub(crate) fn subscribe_logs(run_id: RunId, sink: StreamSink<LogBatchDto>) -> BridgeResult<()> {
    subscribe_plain(run_id, &hub().logs, sink);
    Ok(())
}

pub(crate) fn subscribe_metrics(
    run_id: RunId,
    sink: StreamSink<MetricBatchDto>,
) -> BridgeResult<()> {
    subscribe_plain(run_id, &hub().metrics, sink);
    Ok(())
}

fn subscribe_plain<T: Clone + Send + 'static + crate::frb_generated::SseEncode>(
    run_id: RunId,
    subscriptions: &Mutex<Vec<Subscription<T>>>,
    sink: StreamSink<T>,
) {
    let (sender, receiver) = mpsc::channel();
    subscriptions
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .push(Subscription {
            run_id: run_id.to_string(),
            sender,
        });
    spawn_forwarder(sink, receiver);
}

fn spawn_forwarder<T: Send + 'static + crate::frb_generated::SseEncode>(
    sink: StreamSink<T>,
    receiver: Receiver<T>,
) {
    let _ = std::thread::Builder::new()
        .name("koncet-stream".to_string())
        .spawn(move || {
            while let Ok(item) = receiver.recv() {
                if sink.add(item).is_err() {
                    break;
                }
            }
        });
}

fn spawn_session_forwarder(sink: StreamSink<SessionFrameDto>, receiver: Receiver<SessionFrameDto>) {
    let _ = std::thread::Builder::new()
        .name("koncet-session-stream".to_string())
        .spawn(move || {
            let mut last_key = (0u64, i64::MIN);
            while let Ok(item) = receiver.recv() {
                let key = (item.watermark_seq, item.frame.clock_ms);
                if key <= last_key {
                    continue;
                }
                last_key = key;
                let finalised = item.finalised;
                if sink.add(item).is_err() {
                    break;
                }
                if finalised {
                    break;
                }
            }
        });
}

/// Convenience used by command handlers after a committed update.
pub(crate) fn publish(update: RunUpdate) -> RunUpdateDto {
    let dto: RunUpdateDto = update.into();
    publish_update(&hub(), &dto);
    dto
}

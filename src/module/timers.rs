//! Module-side timer registry.
//!
//! The node delivers each fire as `InvocationType::Timer`. This registry runs
//! the handler and returns `InvocationResultPayload::Timer`. `Serial` stays on
//! the module loop. `Task`, `Blocking`, and `Dedicated` do not.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use blvm_node::module::ipc::protocol::{InvocationResultMessage, InvocationResultPayload};
use blvm_node::module::timers::types::{
    AckMode, Clock, CreateMode, First, JitterShape, Missed, Overlap, Pacing, Persist, Randomness,
    Repeat, TimerAck, TimerFire, TimerId, TimerSpec, TimerTarget, monotonic_interval,
};
use blvm_node::module::traits::{ModuleError, NodeAPI};
use tokio::sync::{Semaphore, mpsc, oneshot};

pub use blvm_node::module::timers::types::TimerInfo;

type FireFut = Pin<Box<dyn Future<Output = Result<TimerAck, String>> + Send>>;
type FireFn = Arc<dyn Fn(TimerFire) -> FireFut + Send + Sync>;

/// Where a timer handler runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Exec {
    /// The module loop awaits the handler before the next event.
    Serial,
    /// `tokio::spawn`. Concurrent with events and other timers.
    Task,
    /// `spawn_blocking`, at most four per module.
    Blocking,
    /// One OS thread for this name. At most four names.
    Dedicated(String),
}

struct Slot {
    exec: Exec,
    ack: AckMode,
    run: FireFn,
}

/// Handlers keyed by the id the node returned from `timer_create`.
pub struct TimerRegistry {
    slots: Mutex<HashMap<TimerId, Slot>>,
    blocking: Arc<Semaphore>,
    dedicated: Mutex<HashMap<String, mpsc::UnboundedSender<DedicatedJob>>>,
}

struct DedicatedJob {
    fire: TimerFire,
    run: FireFn,
    done: oneshot::Sender<Result<TimerAck, String>>,
}

impl TimerRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            slots: Mutex::new(HashMap::new()),
            blocking: Arc::new(Semaphore::new(4)),
            dedicated: Mutex::new(HashMap::new()),
        })
    }

    pub fn register(&self, id: TimerId, exec: Exec, ack: AckMode, run: FireFn) {
        self.slots
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .insert(id, Slot { exec, ack, run });
    }

    pub fn is_detached(&self, id: &TimerId) -> bool {
        self.slots
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .get(id)
            .is_some_and(|slot| !matches!(slot.exec, Exec::Serial))
    }

    /// Start a Task, Blocking, or Dedicated handler. The result is sent later.
    pub fn start_detached(
        self: &Arc<Self>,
        fire: TimerFire,
        correlation_id: u64,
        result_tx: oneshot::Sender<InvocationResultMessage>,
    ) {
        let Some(slot) = self.slot(&fire.timer_id) else {
            let _ = result_tx.send(timer_failure(correlation_id, "unknown timer"));
            return;
        };
        if slot.ack == AckMode::Receipt {
            let _ = result_tx.send(timer_ok(correlation_id, TimerAck::Continue));
            self.launch(slot, fire, None);
            return;
        }
        self.launch(slot, fire, Some((correlation_id, result_tx)));
    }

    /// Await a `Serial` handler. `Receipt` is sent before the handler runs. The caller
    /// still waits, so a serial handler keeps the module loop off events until it returns.
    pub async fn run_serial(
        &self,
        correlation_id: u64,
        fire: &TimerFire,
        result_tx: oneshot::Sender<InvocationResultMessage>,
    ) {
        let Some(slot) = self.slot(&fire.timer_id) else {
            let _ = result_tx.send(timer_failure(correlation_id, "unknown timer"));
            return;
        };
        if slot.ack == AckMode::Receipt {
            let _ = result_tx.send(timer_ok(correlation_id, TimerAck::Continue));
            let _ = (slot.run)(fire.clone()).await;
            return;
        }
        let message = match (slot.run)(fire.clone()).await {
            Ok(ack) => timer_ok(correlation_id, ack),
            Err(err) => timer_failure(correlation_id, &err),
        };
        let _ = result_tx.send(message);
    }

    fn slot(&self, id: &TimerId) -> Option<Slot> {
        let slots = self.slots.lock().unwrap_or_else(|err| err.into_inner());
        let slot = slots.get(id)?;
        Some(Slot {
            exec: slot.exec.clone(),
            ack: slot.ack,
            run: Arc::clone(&slot.run),
        })
    }

    fn launch(
        self: &Arc<Self>,
        slot: Slot,
        fire: TimerFire,
        reply: Option<(u64, oneshot::Sender<InvocationResultMessage>)>,
    ) {
        let send = |result: Result<TimerAck, String>| {
            if let Some((correlation_id, tx)) = reply {
                let message = match result {
                    Ok(ack) => timer_ok(correlation_id, ack),
                    Err(err) => timer_failure(correlation_id, &err),
                };
                let _ = tx.send(message);
            }
        };
        match slot.exec {
            Exec::Serial => {
                let run = slot.run;
                tokio::spawn(async move {
                    send(run(fire).await);
                });
            }
            Exec::Task => {
                let run = slot.run;
                tokio::spawn(async move {
                    let joined = tokio::spawn(run(fire)).await;
                    send(flatten_join(joined));
                });
            }
            Exec::Blocking => {
                let run = slot.run;
                let blocking = Arc::clone(&self.blocking);
                tokio::spawn(async move {
                    let Ok(_permit) = blocking.acquire().await else {
                        send(Err("blocking pool closed".into()));
                        return;
                    };
                    let joined = tokio::task::spawn_blocking(move || {
                        let runtime = match tokio::runtime::Builder::new_current_thread()
                            .enable_all()
                            .build()
                        {
                            Ok(runtime) => runtime,
                            Err(err) => return Err(err.to_string()),
                        };
                        runtime.block_on(run(fire))
                    })
                    .await;
                    send(flatten_join(joined));
                });
            }
            Exec::Dedicated(name) => {
                let run = slot.run;
                let (done_tx, done_rx) = oneshot::channel();
                if let Err(err) = self.enqueue_dedicated(name, fire, run, done_tx) {
                    send(Err(err));
                    return;
                }
                tokio::spawn(async move {
                    match done_rx.await {
                        Ok(result) => send(result),
                        Err(_) => send(Err("dedicated thread dropped the fire".into())),
                    }
                });
            }
        }
    }

    fn enqueue_dedicated(
        &self,
        name: String,
        fire: TimerFire,
        run: FireFn,
        done: oneshot::Sender<Result<TimerAck, String>>,
    ) -> Result<(), String> {
        let mut threads = self.dedicated.lock().unwrap_or_else(|err| err.into_inner());
        if let Some(tx) = threads.get(&name) {
            return tx
                .send(DedicatedJob { fire, run, done })
                .map_err(|_| "dedicated thread stopped".to_string());
        }
        if threads.len() >= 4 {
            return Err("dedicated thread limit is 4".into());
        }
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send(DedicatedJob { fire, run, done })
            .map_err(|_| "dedicated thread stopped".to_string())?;
        threads.insert(name.clone(), tx);
        std::thread::Builder::new()
            .name(format!("blvm-timer-{name}"))
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(_) => return,
                };
                let local = tokio::task::LocalSet::new();
                local.block_on(&runtime, async move {
                    while let Some(job) = rx.recv().await {
                        let DedicatedJob { fire, run, done } = job;
                        let outcome = match tokio::task::spawn_local(run(fire)).await {
                            Ok(result) => result,
                            Err(err) => Err(err.to_string()),
                        };
                        let _ = done.send(outcome);
                    }
                });
            })
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

fn flatten_join(
    joined: Result<Result<TimerAck, String>, tokio::task::JoinError>,
) -> Result<TimerAck, String> {
    match joined {
        Ok(result) => result,
        Err(err) => Err(err.to_string()),
    }
}

pub fn timer_ok(correlation_id: u64, ack: TimerAck) -> InvocationResultMessage {
    InvocationResultMessage {
        correlation_id,
        success: true,
        payload: Some(InvocationResultPayload::Timer(ack)),
        error: None,
    }
}

pub fn timer_failure(correlation_id: u64, err: &str) -> InvocationResultMessage {
    InvocationResultMessage {
        correlation_id,
        success: false,
        payload: None,
        error: Some(err.to_string()),
    }
}

/// The 30 second tick that replaces `sleep` inside `select!`.
///
/// `FixedRate` plus `Overlap::Skip`. An event does not reset the interval.
pub fn sdk_tick_spec() -> TimerSpec {
    let mut spec = monotonic_interval(Duration::from_secs(30), Pacing::FixedRate);
    spec.first = First::Delay(Duration::from_secs(30));
    spec.overlap = Overlap::Skip;
    spec.key = Some("sdk.tick".into());
    spec.clock = Clock::Monotonic;
    spec
}

fn next_daily_utc_ms(hour: u32, minute: u32) -> u64 {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let day = 86_400_000u64;
    let today = now_ms - (now_ms % day);
    let at = (hour as u64) * 3_600_000 + (minute as u64) * 60_000;
    let target = today + at;
    if target > now_ms {
        target
    } else {
        target + day
    }
}

/// Builds a `TimerSpec` and, on `spawn`, registers the handler and calls `timer_create`.
pub struct TimerBuilder {
    spec: TimerSpec,
    exec: Exec,
    api: Option<Arc<dyn NodeAPI>>,
    registry: Option<Arc<TimerRegistry>>,
    chain_depth: u32,
    exp_floor: Option<Duration>,
    exp_cap: Option<Duration>,
}

impl TimerBuilder {
    pub fn new() -> Self {
        Self {
            spec: monotonic_interval(Duration::from_secs(10), Pacing::FixedRate),
            exec: Exec::Serial,
            api: None,
            registry: None,
            chain_depth: 0,
            exp_floor: None,
            exp_cap: None,
        }
    }

    pub fn with_runtime(api: Arc<dyn NodeAPI>, registry: Arc<TimerRegistry>) -> Self {
        Self {
            api: Some(api),
            registry: Some(registry),
            ..Self::new()
        }
    }

    pub fn every(mut self, period: Duration) -> Self {
        if let blvm_node::module::timers::types::Repeat::Interval {
            period: ref mut slot,
            ..
        } = self.spec.repeat
        {
            *slot = period;
        }
        self
    }

    pub fn once(mut self, delay: Duration) -> Self {
        self.spec.first = First::Delay(delay);
        self.spec.repeat = blvm_node::module::timers::types::Repeat::Once;
        self
    }

    pub fn fixed_rate(mut self) -> Self {
        if let blvm_node::module::timers::types::Repeat::Interval { ref mut pacing, .. } =
            self.spec.repeat
        {
            *pacing = Pacing::FixedRate;
        }
        self
    }

    pub fn on_overlap(mut self, overlap: Overlap) -> Self {
        self.spec.overlap = overlap;
        self
    }

    pub fn key(mut self, key: impl Into<String>) -> Self {
        self.spec.key = Some(key.into());
        self
    }

    pub fn exec(mut self, exec: Exec) -> Self {
        self.exec = exec;
        self
    }

    pub fn jitter(mut self, max: Duration, shape: JitterShape) -> Self {
        self.spec.randomness = match self.spec.randomness {
            Randomness::Spread { max: spread_max }
            | Randomness::JitterAndSpread { spread_max, .. } => Randomness::JitterAndSpread {
                jitter_max: max,
                shape,
                spread_max,
            },
            _ => Randomness::Jitter { max, shape },
        };
        self
    }

    pub fn spread(mut self, max: Duration) -> Self {
        self.spec.randomness = match self.spec.randomness {
            Randomness::Jitter {
                max: jitter_max,
                shape,
            }
            | Randomness::JitterAndSpread {
                jitter_max, shape, ..
            } => Randomness::JitterAndSpread {
                jitter_max,
                shape,
                spread_max: max,
            },
            _ => Randomness::Spread { max },
        };
        self
    }

    pub fn max_fires(mut self, n: u32) -> Self {
        match &mut self.spec.repeat {
            Repeat::Interval { max_fires, .. }
            | Repeat::Blocks { max_fires, .. }
            | Repeat::Cron { max_fires, .. } => *max_fires = Some(n),
            Repeat::Once => {}
        }
        self
    }

    pub fn missed(mut self, missed: Missed) -> Self {
        self.spec.missed = missed;
        self
    }

    pub fn persist_durable(mut self, key: impl Into<String>) -> Self {
        self.spec.key = Some(key.into());
        self.spec.persist = Persist::Durable { audit: false };
        self
    }

    /// Wall clock. Next `hour:minute` UTC, then every 24 hours.
    pub fn daily_at_utc(mut self, hour: u32, minute: u32) -> Self {
        self.spec.clock = Clock::Wall;
        self.spec.first = First::AtUnixMs(next_daily_utc_ms(hour, minute));
        self.spec.repeat = Repeat::Interval {
            period: Duration::from_secs(24 * 60 * 60),
            pacing: Pacing::FixedRate,
            max_fires: None,
            until: None,
        };
        self
    }

    pub fn after_blocks(mut self, blocks: u32) -> Self {
        self.spec.clock = Clock::Chain;
        self.spec.first = First::AfterBlocks {
            blocks,
            depth: self.chain_depth,
        };
        self.spec.repeat = Repeat::Once;
        self
    }

    pub fn depth(mut self, depth: u32) -> Self {
        self.chain_depth = depth;
        match &mut self.spec.first {
            First::AfterBlocks { depth: slot, .. } | First::AtHeight { depth: slot, .. } => {
                *slot = depth;
            }
            _ => {}
        }
        self
    }

    pub fn every_blocks(mut self, every: u32) -> Self {
        self.spec.clock = Clock::Chain;
        self.spec.first = First::AfterBlocks {
            blocks: 0,
            depth: self.chain_depth,
        };
        self.spec.repeat = Repeat::Blocks {
            every,
            max_fires: None,
            until: None,
        };
        self
    }

    pub fn poisson(mut self, mean: Duration) -> Self {
        let floor = self.exp_floor.unwrap_or(Duration::from_millis(1));
        let cap = self.exp_cap.unwrap_or(mean);
        if let Repeat::Interval {
            ref mut period,
            ref mut pacing,
            ..
        } = self.spec.repeat
        {
            *period = mean;
            *pacing = Pacing::FixedRate;
        }
        self.spec.randomness = Randomness::Exponential { mean, floor, cap };
        self
    }

    pub fn bounds(mut self, floor: Duration, cap: Duration) -> Self {
        self.exp_floor = Some(floor);
        self.exp_cap = Some(cap);
        if let Randomness::Exponential {
            floor: ref mut slot_floor,
            cap: ref mut slot_cap,
            ..
        } = self.spec.randomness
        {
            *slot_floor = floor;
            *slot_cap = cap;
        }
        self
    }

    pub fn spec(&self) -> TimerSpec {
        self.spec.clone()
    }

    pub async fn spawn<F, Fut>(self, handler: F) -> Result<TimerHandle, ModuleError>
    where
        F: Fn(TimerFire) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<TimerAck, String>> + Send + 'static,
    {
        let api = self
            .api
            .clone()
            .ok_or_else(|| ModuleError::OperationError("timer builder has no node api".into()))?;
        let registry = self
            .registry
            .clone()
            .ok_or_else(|| ModuleError::OperationError("timer builder has no registry".into()))?;
        let mode = if self.spec.key.is_some() {
            CreateMode::Ensure
        } else {
            CreateMode::CreateOnly
        };
        let info = api.timer_create(self.spec.clone(), mode).await?;
        let run: FireFn = Arc::new(move |fire| Box::pin(handler(fire)));
        registry.register(info.timer_id, self.exec, self.spec.delivery.ack, run);
        Ok(TimerHandle {
            id: info.timer_id,
            api,
        })
    }
}

impl Default for TimerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Dropping the handle does not cancel the timer.
pub struct TimerHandle {
    id: TimerId,
    api: Arc<dyn NodeAPI>,
}

impl TimerHandle {
    pub async fn cancel(&self) -> Result<(), ModuleError> {
        self.api.timer_cancel(TimerTarget::Id(self.id)).await
    }

    pub async fn pause(&self) -> Result<(), ModuleError> {
        self.api.timer_pause(TimerTarget::Id(self.id)).await
    }

    pub async fn resume(&self) -> Result<(), ModuleError> {
        self.api.timer_resume(TimerTarget::Id(self.id)).await
    }

    pub async fn trigger(&self) -> Result<(), ModuleError> {
        self.api.timer_trigger(TimerTarget::Id(self.id)).await
    }

    pub async fn info(&self) -> Result<TimerInfo, ModuleError> {
        self.api.timer_get(TimerTarget::Id(self.id)).await
    }

    pub fn id(&self) -> TimerId {
        self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn fire(id: TimerId) -> TimerFire {
        TimerFire {
            timer_id: id,
            key: None,
            payload: Vec::new(),
            seq: 1,
            attempt: 1,
            clock: Clock::Monotonic,
            scheduled_unix_ms: 0,
            planned_unix_ms: 0,
            fired_unix_ms: 0,
            lateness_ms: 0,
            missed: 0,
            coalesced: 0,
            skipped_overlap: 0,
            clock_step_ms: 0,
            tip_height: None,
            tip_hash: None,
            tip_mtp: None,
            is_last: false,
        }
    }

    #[test]
    fn the_tick_spec_is_a_fixed_rate_skip_timer() {
        let spec = sdk_tick_spec();
        assert_eq!(spec.clock, Clock::Monotonic);
        assert_eq!(spec.overlap, Overlap::Skip);
        assert_eq!(spec.key.as_deref(), Some("sdk.tick"));
        assert_eq!(spec.first, First::Delay(Duration::from_secs(30)));
    }

    #[test]
    fn the_documented_builders_match_timer_validation() {
        use blvm_node::module::timers::types::{TimerConfig, ValidateCtx, validate_spec};

        let ctx = ValidateCtx {
            now_unix_ms: 0,
            tip_height: Some(100),
        };
        let limits = TimerConfig::default();
        let poll = TimerBuilder::new()
            .every(Duration::from_secs(10))
            .fixed_rate()
            .jitter(Duration::from_secs(2), JitterShape::Symmetric)
            .spread(Duration::from_secs(10))
            .max_fires(8640)
            .on_overlap(Overlap::Skip)
            .key("upstream.poll")
            .exec(Exec::Task)
            .spec();
        validate_spec(&poll, &limits, &ctx).unwrap();
        assert!(matches!(
            poll.randomness,
            Randomness::JitterAndSpread { .. }
        ));

        let daily = TimerBuilder::new()
            .daily_at_utc(3, 0)
            .persist_durable("housekeeping.compact")
            .missed(Missed::FireOnce)
            .exec(Exec::Blocking)
            .spec();
        validate_spec(&daily, &limits, &ctx).unwrap();
        assert_eq!(daily.clock, Clock::Wall);
        assert_eq!(daily.missed, Missed::FireOnce);
        assert!(matches!(daily.persist, Persist::Durable { audit: false }));

        let after = TimerBuilder::new().after_blocks(6).depth(0).spec();
        validate_spec(&after, &limits, &ctx).unwrap();
        assert_eq!(
            after.first,
            First::AfterBlocks {
                blocks: 6,
                depth: 0
            }
        );

        let every = TimerBuilder::new()
            .every_blocks(1)
            .on_overlap(Overlap::Skip)
            .spec();
        validate_spec(&every, &limits, &ctx).unwrap();

        let poisson = TimerBuilder::new()
            .poisson(Duration::from_secs(5))
            .bounds(Duration::from_millis(500), Duration::from_secs(60))
            .spec();
        validate_spec(&poisson, &limits, &ctx).unwrap();
    }

    #[tokio::test]
    async fn blocking_handler_sleeps_off_the_caller() {
        let registry = TimerRegistry::new();
        let id = TimerId([8; 16]);
        registry.register(
            id,
            Exec::Blocking,
            AckMode::Completion,
            Arc::new(|_fire| {
                Box::pin(async move {
                    tokio::time::sleep(Duration::from_millis(30)).await;
                    Ok(TimerAck::Continue)
                })
            }),
        );
        let (tx, rx) = oneshot::channel();
        let started = std::time::Instant::now();
        registry.start_detached(fire(id), 3, tx);
        let message = rx.await.unwrap();
        assert!(message.success);
        assert!(started.elapsed() >= Duration::from_millis(30));
    }

    #[tokio::test]
    async fn a_dedicated_thread_runs_during_a_serial_handler() {
        let registry = TimerRegistry::new();
        let serial_id = TimerId([3; 16]);
        let dedicated_id = TimerId([4; 16]);
        let release = Arc::new(tokio::sync::Notify::new());
        let saw = Arc::new(AtomicUsize::new(0));
        let release_h = Arc::clone(&release);
        registry.register(
            serial_id,
            Exec::Serial,
            AckMode::Completion,
            Arc::new(move |_fire| {
                let release_h = Arc::clone(&release_h);
                Box::pin(async move {
                    release_h.notified().await;
                    Ok(TimerAck::Continue)
                })
            }),
        );
        let saw_d = Arc::clone(&saw);
        registry.register(
            dedicated_id,
            Exec::Dedicated("poll".into()),
            AckMode::Completion,
            Arc::new(move |_fire| {
                let saw_d = Arc::clone(&saw_d);
                Box::pin(async move {
                    saw_d.fetch_add(1, Ordering::SeqCst);
                    Ok(TimerAck::Continue)
                })
            }),
        );
        let run = registry.slot(&serial_id).unwrap();
        let serial = tokio::spawn(async move {
            let _ = (run.run)(fire(serial_id)).await;
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        let (tx, rx) = oneshot::channel();
        registry.start_detached(fire(dedicated_id), 1, tx);
        tokio::time::timeout(Duration::from_secs(2), async {
            while saw.load(Ordering::SeqCst) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("dedicated handler waited for the serial handler");
        release.notify_one();
        serial.await.unwrap();
        let message = rx.await.unwrap();
        assert!(message.success);
    }
}

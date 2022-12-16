use crate::clocks::WasiMonotonicClock;
use crate::stream::WasiStream;
use crate::Error;
use bitflags::bitflags;
use cap_std::time::{Duration, Instant};

bitflags! {
    pub struct RwEventFlags: u32 {
        const HANGUP = 0b1;
    }
}

pub struct RwSubscription<'a> {
    pub stream: &'a dyn WasiStream,
    status: Option<Result<(u64, RwEventFlags), Error>>,
}

impl<'a> RwSubscription<'a> {
    pub fn new(stream: &'a dyn WasiStream) -> Self {
        Self {
            stream,
            status: None,
        }
    }
    pub fn complete(&mut self, size: u64, flags: RwEventFlags) {
        self.status = Some(Ok((size, flags)))
    }
    pub fn error(&mut self, error: Error) {
        self.status = Some(Err(error))
    }
    pub fn result(&mut self) -> Option<Result<(u64, RwEventFlags), Error>> {
        self.status.take()
    }
}

pub struct MonotonicClockSubscription<'a> {
    pub clock: &'a dyn WasiMonotonicClock,
    pub deadline: Instant,
    pub precision: Duration,
}

impl<'a> MonotonicClockSubscription<'a> {
    pub fn now(&self) -> Instant {
        self.clock.now(self.precision)
    }
    pub fn duration_until(&self) -> Option<Duration> {
        self.deadline.checked_duration_since(self.now())
    }
    pub fn result(&self) -> Option<Result<(), Error>> {
        if self.now().checked_duration_since(self.deadline).is_some() {
            Some(Ok(()))
        } else {
            None
        }
    }
}

pub enum Subscription<'a> {
    Read(RwSubscription<'a>),
    Write(RwSubscription<'a>),
    MonotonicClock(MonotonicClockSubscription<'a>),
}

#[derive(Debug)]
pub enum SubscriptionResult {
    Read(Result<(u64, RwEventFlags), Error>),
    Write(Result<(u64, RwEventFlags), Error>),
    MonotonicClock(Result<(), Error>),
}

impl SubscriptionResult {
    pub fn from_subscription(s: Subscription) -> Option<SubscriptionResult> {
        match s {
            Subscription::Read(mut s) => s.result().map(SubscriptionResult::Read),
            Subscription::Write(mut s) => s.result().map(SubscriptionResult::Write),
            Subscription::MonotonicClock(s) => s.result().map(SubscriptionResult::MonotonicClock),
        }
    }
}

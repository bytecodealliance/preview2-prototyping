#![allow(unused_variables)]

use crate::preview2::poll::PollableEntry;
use crate::wasi::{
    monotonic_clock::{self, Instant},
    poll::Pollable,
    timezone::{self, Timezone, TimezoneDisplay},
    wall_clock::{self, Datetime},
};
use crate::WasiView;
use cap_std::time::SystemTime;

impl TryFrom<SystemTime> for Datetime {
    type Error = anyhow::Error;

    fn try_from(time: SystemTime) -> Result<Self, Self::Error> {
        let duration =
            time.duration_since(SystemTime::from_std(std::time::SystemTime::UNIX_EPOCH))?;

        Ok(Datetime {
            seconds: duration.as_secs(),
            nanoseconds: duration.subsec_nanos(),
        })
    }
}

#[async_trait::async_trait]
impl<T: WasiView> wall_clock::Host for T {
    async fn now(&mut self) -> anyhow::Result<Datetime> {
        let now = self.ctx().clocks.wall.now();
        Ok(Datetime {
            seconds: now.as_secs(),
            nanoseconds: now.subsec_nanos(),
        })
    }

    async fn resolution(&mut self) -> anyhow::Result<Datetime> {
        let res = self.ctx().clocks.wall.resolution();
        Ok(Datetime {
            seconds: res.as_secs(),
            nanoseconds: res.subsec_nanos(),
        })
    }
}

#[async_trait::async_trait]
impl<T: WasiView> monotonic_clock::Host for T {
    async fn now(&mut self) -> anyhow::Result<Instant> {
        Ok(self.ctx().clocks.monotonic.now())
    }

    async fn resolution(&mut self) -> anyhow::Result<Instant> {
        Ok(self.ctx().clocks.monotonic.resolution())
    }

    async fn subscribe(&mut self, when: Instant, absolute: bool) -> anyhow::Result<Pollable> {
        Ok(self
            .table_mut()
            .push(Box::new(PollableEntry::MonotonicClock(when, absolute)))?)
    }
}

#[async_trait::async_trait]
impl<T: WasiView> timezone::Host for T {
    async fn display(
        &mut self,
        timezone: Timezone,
        when: Datetime,
    ) -> anyhow::Result<TimezoneDisplay> {
        todo!()
    }

    async fn utc_offset(&mut self, timezone: Timezone, when: Datetime) -> anyhow::Result<i32> {
        todo!()
    }

    async fn drop_timezone(&mut self, timezone: Timezone) -> anyhow::Result<()> {
        todo!()
    }
}

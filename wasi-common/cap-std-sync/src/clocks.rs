use cap_std::time::{Duration, Instant, SystemClock};
use cap_std::{ambient_authority, AmbientAuthority};
use cap_time_ext::{MonotonicClockExt, SystemClockExt};
use wasi_common::clocks::{WasiClocks, WasiMonotonicClock, WasiWallClock};

pub struct WallClock {
    clock: cap_std::time::SystemClock,
    ambient_authority: AmbientAuthority,
}

impl WallClock {
    pub fn new(ambient_authority: AmbientAuthority) -> Self {
        Self {
            clock: cap_std::time::SystemClock::new(ambient_authority),
            ambient_authority,
        }
    }
}
impl WasiWallClock for WallClock {
    fn resolution(&self) -> Duration {
        self.clock.resolution()
    }
    fn now(&self) -> Duration {
        self.clock
            .now()
            .duration_since(SystemClock::UNIX_EPOCH)
            .unwrap()
    }
    fn dup(&self) -> Box<dyn WasiWallClock + Send + Sync> {
        let clock = cap_std::time::SystemClock::new(self.ambient_authority);
        Box::new(WallClock {
            clock,
            ambient_authority: self.ambient_authority,
        })
    }
}

pub struct MonotonicClock {
    clock: cap_std::time::MonotonicClock,
    initial: Instant,
    ambient_authority: AmbientAuthority,
}

impl MonotonicClock {
    pub fn new(ambient_authority: AmbientAuthority) -> Self {
        let clock = cap_std::time::MonotonicClock::new(ambient_authority);
        let initial = clock.now();
        MonotonicClock {
            clock,
            initial,
            ambient_authority,
        }
    }
}
impl WasiMonotonicClock for MonotonicClock {
    fn resolution(&self) -> u64 {
        self.clock.resolution().as_nanos().try_into().unwrap()
    }
    fn now(&self) -> u64 {
        self.clock
            .now()
            .duration_since(self.initial)
            .as_nanos()
            .try_into()
            .unwrap()
    }
    fn dup(&self) -> Box<dyn WasiMonotonicClock + Send + Sync> {
        let clock = cap_std::time::MonotonicClock::new(self.ambient_authority);
        Box::new(MonotonicClock {
            clock,
            initial: self.initial,
            ambient_authority: self.ambient_authority,
        })
    }
}

pub fn clocks_ctx() -> WasiClocks {
    let default_monotonic_clock = Box::new(MonotonicClock::new(ambient_authority()));
    let default_wall_clock = Box::new(WallClock::new(ambient_authority()));

    WasiClocks {
        default_monotonic_clock,
        default_wall_clock,
    }
}

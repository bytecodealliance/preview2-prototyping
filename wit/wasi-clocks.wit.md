# WASI Clocks API

WASI Clocks is a clock API intended to let users query the current time and
to measure elapsed time.

It is intended to be portable at least between Unix-family platforms and
Windows.

## `instant`
```wit
/// A timestamp in nanoseconds.
type instant = u64
```

## `datetime`
```wit
/// A time and date in seconds plus nanoseconds.
record datetime {
    seconds: u64,
    nanoseconds: u32,
}
```

## `monotonic-clock`
```wit
/// A monotonic clock is a clock which has an unspecified initial value, and
/// successive reads of the clock will produce non-decreasing values.
///
/// It is intended for measuring elapsed time.
resource monotonic-clock {
```

## `now`
```wit
/// Read the current value of the clock.
///
/// As this the clock is monotonic, calling this function repeatedly will produce
/// a sequence of non-decreasing values.
now: func() -> instant
```

## `resolution`
```wit
/// Query the resolution of the clock.
resolution: func() -> instant
```

## `new-timer`
```wit
/// This creates a new `monotonic-timer` with the given starting time. It will
/// count down from this time until it reaches zero.
new-timer: func(initial: instant) -> monotonic-timer
```

```wit
}
```

## `wall-clock`
```wit
/// A wall clock is a clock which measures the date and time according to some
/// external reference.
///
/// External references may be reset, so this clock is not necessarily
/// monotonic, making it unsuitable for measuring elapsed time.
///
/// It is intended for reporting the current date and time for humans.
resource wall-clock {
```

## `now`
```wit
/// Read the current value of the clock.
///
/// As this the clock is not monotonic, calling this function repeatedly will
/// not necessarily produce a sequence of non-decreasing values.
///
/// The returned timestamps represent the number of seconds since
/// 1970-01-01T00:00:00Z, also known as [POSIX's Seconds Since the Epoch], also
/// known as [Unix Time].
///
/// The nanoseconds field of the output is always less than 1000000000.
///
/// [POSIX's Seconds Since the Epoch]: https://pubs.opengroup.org/onlinepubs/9699919799/xrat/V4_xbd_chap04.html#tag_21_04_16
/// [Unix Time]: https://en.wikipedia.org/wiki/Unix_time
now: func() -> datetime
```

## `resolution`
```wit
/// Query the resolution of the clock.
///
/// The nanoseconds field of the output is always less than 1000000000.
resolution: func() -> datetime
```

```wit
}
```

## `monotonic-timer`
```wit
/// This is a timer that counts down from a given starting time down to zero
/// on a monotonic clock.
resource monotonic-timer {
```

## `current`
```wit
/// Returns the amount of time left before this timer reaches zero.
current: func() -> instant
```

## `expiration`
```wit
// /// Returns a future that completes when the timer reaches zero.
// expiration: func() -> future
```

```wit
}
```

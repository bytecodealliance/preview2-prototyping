# WASI Clocks API

WASI Clocks is a clock API intended to let users query the current time and
to measure elapsed time.

It is intended to be portable at least between Unix-family platforms and
Windows.

## `monotonic-clock`
```wit
/// A monotonic clock is a clock which has an unspecified initial value, and
/// successive reads of the clock will produce non-decreasing values.
///
/// It is intended for measuring elapsed time.
type monotonic-clock = u32
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
type wall-clock = u32
```

## `monotonic-timer`
```wit
/// This is a timer that counts down from a given starting time down to zero
/// on a monotonic clock.
type monotonic-timer = u32
```

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

## `now`
```wit
/// Read the current value of the clock.
///
/// As this the clock is monotonic, calling this function repeatedly will produce
/// a sequence of non-decreasing values.
monotonic-clock-now: func(fd: monotonic-clock) -> instant
```

## `resolution`
```wit
/// Query the resolution of the clock.
monotonic-clock-resolution: func(fd: monotonic-clock) -> instant
```

## `new-timer`
```wit
/// This creates a new `monotonic-timer` with the given starting time. It will
/// count down from this time until it reaches zero.
monotonic-clock-new-timer: func(fd: monotonic-clock, initial: instant) -> monotonic-timer
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
wall-clock-now: func(fd: wall-clock) -> datetime
```

## `resolution`
```wit
/// Query the resolution of the clock.
///
/// The nanoseconds field of the output is always less than 1000000000.
wall-clock-resolution: func(fd: wall-clock) -> datetime
```

## `current`
```wit
/// Returns the amount of time left before this timer reaches zero.
monotonic-timer-current: func(fd: monotonic-timer) -> instant
```

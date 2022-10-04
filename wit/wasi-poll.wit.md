# WASI Poll API

WASI Poll is a poll API intended to let users wait for I/O events on
multiple handles at once.

## `subscription`
```wit
/// The type of event to subscribe to.
record subscription {
    /// Information about the subscription.
    info: subscription-info,
    /// The value of the `userdata` to include in associated events.
    userdata: userdata,
}
```

## `subscription-info`
```wit
/// Information about events to subscribe to.
variant subscription-info {
    /// Set a monotonic clock timer.
    monotonic-clock-timeout(monotonic-clock-timeout),
    /// Set a wall clock timer.
    wall-clock-timeout(wall-clock-timeout),
    /// Wait for a readable stream to have data ready.
    read(descriptor),
    /// Wait for a writeable stream to be ready to accept data.
    write(descriptor),
}
```

## `monotonic-clock-timeout`
```wit
/// Information about a monotonic clock timeout.
record monotonic-clock-timeout {
    /// An absolute or relative timestamp.
    timeout: instant,
    /// Specifies an absolute, rather than relative, timeout.
    is-absolute: bool,
}
```

## `wall-clock-timeout`
```wit
/// Information about a wall clock timeout.
record wall-clock-timeout {
    /// An absolute or relative timestamp.
    timeout: datetime,
    /// Specifies an absolute, rather than relative, timeout.
    is-absolute: bool,
}
```

## `event`
```wit
/// An event which has occurred.
record event {
    /// The value of the `userdata` from the associated subscription.
    userdata: userdata,
    /// Information about the event.
    info: event-info,
}
```

## `event-info`
```wit
/// Information about an event which has occurred.
variant event-info {
    /// A monotonic clock timer expired.
    monotonic-clock-timeout,
    /// A wall clock timer expired.
    wall-clock-timeout,
    /// A readable stream has data ready.
    read(read-event),
    /// A writable stream is ready to accept data.
    write(write-event),
}
```

## `read-event`
```wit
/// An event indicating that a readable stream has data ready.
record read-event {
    /// The number of bytes ready to be read.
    nbytes: u64,
    /// Indicates the other end of the stream has disconnected and no further
    /// data will be available on this stream.
    is-closed: bool,
}
```

## `write-event`
```wit
/// An event indicating that a writeable stream is ready to accept data.
record write-event {
    /// The number of bytes ready to be accepted
    nbytes: u64,
    /// Indicates the other end of the stream has disconnected and no further
    /// data will be accepted on this stream.
    is-closed: bool,
}
```

## `userdata`
```wit
/// User-provided data provided with subscriptions that is copied back
/// into emitted events.
type userdata = u64
```

## `instant`
```wit
/// A timestamp in nanoseconds.
///
/// TODO: When wit-bindgen supports importing types from other wit files, use
/// the type from wasi-clocks.
type instant = u64
```

## `datetime`
```wit
/// A time and date in seconds plus nanoseconds.
///
/// TODO: When wit-bindgen supports importing types from other wit files, use
/// the type from wasi-clocks.
record datetime {
    seconds: u64,
    nanoseconds: u32,
}
```

## `descriptor`
```wit
/// A descriptor referring to a readable and/or writeable stream.
///
/// TODO: When wit-bindgen supports importing types from other wit files, use
/// the type from wasi-filesystem.
resource descriptor {
```

```wit
}
```

## `poll-oneoff`
/// Poll for events on a set of descriptors.
///
/// The "oneoff" in the name refers to the fact that this function must do a
/// linear scan through the entire list of subscriptions, which may be
/// inefficient if the number is large and the same subscriptions are used
/// many times. In the future, it may be accompanied by an API similar to
/// Linux's `epoll` which allows sets of subscriptions to be registered and
/// made efficiently reusable.
```wit
poll-oneoff: func(in: list<subscription>) -> list<event>
```

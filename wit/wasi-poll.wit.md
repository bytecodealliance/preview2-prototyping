# WASI Poll API

WASI Poll is a poll API intended to let users wait for I/O events on
multiple handles at once.

## `descriptor`
```wit
/// A "file" descriptor. In the future, this will be replaced by a handle type.
type descriptor = u32
```

## `future`
```wit
/// An asynchronous operation. In the future, this will be replaced by a handle type.
type wasi-future = u32
```

## `drop-future`
```wit
/// Dispose of the specified future, after which it may no longer be used.
drop-future: func(f: wasi-future)
```

## `bytes-result`
```wit
/// Result of querying bytes readable or writable for a `descriptor`
record bytes-result {
    /// Indicates the number of bytes readable or writable for a still-open descriptor
    nbytes: u64,
    /// Indicates whether the other end of the stream has disconnected, in which case
    /// no further data will be received (when reading) or accepted (when writing) on
    /// this stream.
    is-closed: bool
}
```

## `bytes-readable`
```wit
/// Query the specified `descriptor` for how many bytes are available to read.
bytes-readable: func(fd: descriptor) -> bytes-result
```

## `bytes-writable`
```wit
/// Query the specified `descriptor` for the number of bytes ready to be accepted.
bytes-writable: func(fd: descriptor) -> bytes-result
```

## `subscribe-read`
```wit
/// Create a future which will resolve once either the specified descriptor has bytes
/// available to read or the other end of the stream has been closed.
subscribe-read: func(fd: descriptor) -> wasi-future
```

## `subscribe-write`
```wit
/// Create a future which will resolve once either the specified descriptor is ready
/// to accept bytes or the other end of the stream has been closed.
subscribe-write: func(fd: descriptor) -> wasi-future
```

## `subscribe-wall-clock`
```wit
/// Create a future which will resolve once the specified time has been reached.
subscribe-wall-clock: func(when: datetime, absolute: bool) -> wasi-future
```

## `subscribe-monotonic-clock`
```wit
/// Create a future which will resolve once the specified time has been reached.
subscribe-monotonic-clock: func(when: instant, absolute: bool) -> wasi-future
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

## `poll-oneoff`
```wit
/// Poll for completion on a set of futures.
///
/// The "oneoff" in the name refers to the fact that this function must do a
/// linear scan through the entire list of subscriptions, which may be
/// inefficient if the number is large and the same subscriptions are used
/// many times. In the future, it may be accompanied by an API similar to
/// Linux's `epoll` which allows sets of subscriptions to be registered and
/// made efficiently reusable.
poll-oneoff: func(in: list<wasi-future>) -> list<bool>
```

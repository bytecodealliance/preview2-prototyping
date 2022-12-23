use rustix::io::{PollFd, PollFlags};
use std::convert::TryInto;
use wasi_common::sched::subscription::{RwEventFlags, Subscription};
use wasi_common::{sched::Poll, Error, ErrorExt};

pub async fn poll_oneoff<'a>(poll: &mut Poll<'a>) -> Result<(), Error> {
    if poll.is_empty() {
        return Ok(());
    }
    let mut pollfds = Vec::new();
    for s in poll.rw_subscriptions() {
        match s {
            Subscription::Read(f) => {
                let fd = f
                    .stream
                    .pollable_read()
                    .ok_or(Error::invalid_argument().context("file is not pollable"))?;
                pollfds.push(PollFd::from_borrowed_fd(fd, PollFlags::IN));
            }

            Subscription::Write(f) => {
                let fd = f
                    .stream
                    .pollable_write()
                    .ok_or(Error::invalid_argument().context("file is not pollable"))?;
                pollfds.push(PollFd::from_borrowed_fd(fd, PollFlags::OUT));
            }
            Subscription::MonotonicClock { .. } => unreachable!(),
        }
    }

    let ready = loop {
        let poll_timeout = if let Some(t) = poll.earliest_clock_deadline() {
            let duration = t.duration_until().unwrap_or(0);

            // Convert the timeout to milliseconds for `poll`, rounding up.
            //
            // TODO: On Linux and FreeBSD, we could use `ppoll` instead
            // which takes a `timespec.`
            ((duration + 999) / 1000)
                .try_into()
                .map_err(|_| Error::overflow().context("poll timeout"))?
        } else {
            std::os::raw::c_int::max_value()
        };
        tracing::debug!(
            poll_timeout = tracing::field::debug(poll_timeout),
            poll_fds = tracing::field::debug(&pollfds),
            "poll"
        );
        match rustix::io::poll(&mut pollfds, poll_timeout) {
            Ok(ready) => break ready,
            Err(rustix::io::Errno::INTR) => continue,
            Err(err) => return Err(std::io::Error::from(err).into()),
        }
    };
    if ready > 0 {
        for (rwsub, pollfd) in poll.rw_subscriptions().zip(pollfds.into_iter()) {
            let revents = pollfd.revents();
            let (nbytes, rwsub) = match rwsub {
                Subscription::Read(sub) => {
                    let ready = sub.stream.num_ready_bytes().await?;
                    (std::cmp::max(ready, 1), sub)
                }
                Subscription::Write(sub) => (0, sub),
                _ => unreachable!(),
            };
            if revents.contains(PollFlags::NVAL) {
                rwsub.error(Error::badf());
            } else if revents.contains(PollFlags::ERR) {
                rwsub.error(Error::io());
            } else if revents.contains(PollFlags::HUP) {
                rwsub.complete(nbytes, RwEventFlags::HANGUP);
            } else {
                rwsub.complete(nbytes, RwEventFlags::empty());
            };
        }
    } else {
        poll.earliest_clock_deadline()
            .expect("timed out")
            .result()
            .expect("timer deadline is past")
            .unwrap()
    }
    Ok(())
}

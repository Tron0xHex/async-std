use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use futures_timer::Delay;
use pin_project_lite::pin_project;

use crate::stream::Stream;
use crate::task::{Context, Poll};

pin_project! {
    /// A stream that only yields one element once every `duration`, and applies backpressure. Does not drop any elements.
    #[doc(hidden)]
    #[allow(missing_debug_implementations)]
    pub struct Throttle<S> {
        #[pin]
        stream: S,
        duration: Duration,
        #[pin]
        delay: Option<Delay>,
    }
}

impl<S: Stream> Throttle<S> {
    pub(super) fn new(stream: S, duration: Duration) -> Self {
        Throttle {
            stream,
            duration,
            delay: None,
        }
    }
}

impl<S: Stream> Stream for Throttle<S> {
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        let mut this = self.project();
        if let Some(d) = this.delay.as_mut().as_pin_mut() {
            if d.poll(cx).is_ready() {
                this.delay.set(None);
            } else {
                return Poll::Pending;
            }
        }

        match this.stream.poll_next(cx) {
            Poll::Pending => {
                cx.waker().wake_by_ref(); // Continue driving even though emitting Pending
                Poll::Pending
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Ready(Some(v)) => {
                this.delay.set(Some(Delay::new(*this.duration)));
                Poll::Ready(Some(v))
            }
        }
    }
}

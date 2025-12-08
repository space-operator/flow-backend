use std::time::Instant;

use metrics::Histogram;
use pin_project_lite::pin_project;

pin_project! {
    #[derive(Debug)]
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct HistogramFuture<F> {
        #[pin]
        future: F,
        histogram: Histogram,
        start_time: Option<Instant>,
    }
}

impl<F> Future for HistogramFuture<F>
where
    F: Future,
{
    type Output = F::Output;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let start_time = match self.start_time {
            Some(time) => time,
            None => {
                let now = Instant::now();
                *self.as_mut().project().start_time = Some(now);
                now
            }
        };
        let output = self.as_mut().project().future.poll(cx);
        if matches!(output, std::task::Poll::Ready(_)) {
            self.histogram.record(start_time.elapsed().as_secs_f64());
        }
        output
    }
}

pub trait FutureExt: Sized {
    fn histogram(self, histogram: Histogram) -> HistogramFuture<Self>;
}

impl<F> FutureExt for F
where
    F: Future,
{
    fn histogram(self, histogram: Histogram) -> HistogramFuture<Self> {
        HistogramFuture {
            future: self,
            histogram,
            start_time: None,
        }
    }
}

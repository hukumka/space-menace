use amethyst::ecs::{ReadExpect, System, SystemData};
use crossbeam::queue::ArrayQueue;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::time::{Duration, Instant};

pub struct TimeMetricsWrapperSystem<T> {
    inner: T,
    name: &'static str,
}

impl<T> TimeMetricsWrapperSystem<T> {
    pub fn new(inner: T, name: &'static str) -> Self {
        Self { inner, name }
    }
}

impl<'s, T> System<'s> for TimeMetricsWrapperSystem<T>
where
    T: System<'s>,
    <T as System<'s>>::SystemData: SystemData<'s>,
{
    type SystemData = (T::SystemData, ReadExpect<'s, TimeMessageChannel>);

    fn run(&mut self, data: Self::SystemData) {
        let (inner_res, channel) = data;
        let start = Instant::now();
        self.inner.run(inner_res);
        let end = Instant::now();
        channel.push_message(self.name, start, end);
    }
}

pub struct TimeMetricsWriterSystem<T> {
    writer: T,
}

impl<T> TimeMetricsWriterSystem<T> {
    pub fn new(writer: T) -> Self {
        Self { writer }
    }
}

impl TimeMetricsWriterSystem<BufWriter<File>> {
    /// Create new file with `path` and use it as buffer for TimeMetricWriterSystem
    /// If file already exist then truncates it.
    pub fn with_file<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        Ok(Self::new(writer))
    }
}

impl<'s, T: Write> System<'s> for TimeMetricsWriterSystem<T> {
    type SystemData = ReadExpect<'s, TimeMessageChannel>;

    fn run(&mut self, channel: Self::SystemData) {
        while let Some(msg) = channel.pop() {
            writeln!(
                self.writer,
                "{} started={} ended={}",
                msg.name,
                msg.start.as_micros(),
                msg.end.as_micros()
            )
            .expect("Failed to write message");
        }
    }
}

pub struct TimeMessageChannel {
    queue: ArrayQueue<TimeMessage>,
    start: Instant,
}

impl TimeMessageChannel {
    /// Create new instance of `TimeMessageChannel`
    ///
    /// `capacity` - amount of messages then can be pushed into channel
    /// before consuming them. Since every `TimeMetricsWrapper` will
    /// produce exactly 1 message per run it's enough to have capacity
    /// equal to number of systems.
    pub fn new(capacity: usize) -> Self {
        Self {
            queue: ArrayQueue::new(capacity),
            start: Instant::now(),
        }
    }

    fn push_message(&self, name: &'static str, start: Instant, end: Instant) {
        self.queue
            .push(TimeMessage {
                name,
                start: start.duration_since(self.start),
                end: end.duration_since(self.start),
            })
            .expect("Failed to create TimeMessage. Queue is full");
    }

    fn pop(&self) -> Option<TimeMessage> {
        self.queue.pop().ok()
    }
}

struct TimeMessage {
    name: &'static str,
    start: Duration,
    end: Duration,
}

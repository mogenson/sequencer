use std::rc::Rc;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering::SeqCst},
    Arc,
};
use std::thread::{spawn, JoinHandle};
use std::time::{Duration, Instant};

pub trait Clock {
    // start clock
    fn start(&mut self);

    // stop clock
    fn stop(&mut self);

    // reset clock state
    fn reset(&mut self);

    // returns true if clock is running
    fn is_running(&self) -> bool;

    // set the interval between ticks
    fn with_period(&mut self, period: Duration) -> &mut Self;

    // register a callback to be called on each clock tick
    fn on_tick<F>(&mut self, callback: F) -> &mut Self
    where
        F: Fn(usize) + Send + Sync + 'static;

    // get number of ticks since clock start
    fn get_ticks(&self) -> usize;

    // get current period
    fn get_period(&self) -> Duration;
}

#[derive(Clone)]
// a clock source based on polling OS system time
pub struct SystemClock {
    callback: Arc<dyn Fn(usize) + Send + Sync>, // on tick callback
    handle: Rc<Option<JoinHandle<()>>>,         // worker thread wrapped in Rc for Clone
    period: Duration,                           // duration between clock ticks
    running: Arc<AtomicBool>,                   // clock state
    ticks: Arc<AtomicUsize>,                    // number of ticks since clock start
}

impl SystemClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn build(&self) -> Self {
        let mut clock = Self::default();
        clock.callback = self.callback.clone();
        clock.period = self.period;
        clock
    }
}

impl Default for SystemClock {
    fn default() -> Self {
        Self {
            callback: Arc::new(|_| {}),
            handle: Rc::new(Option::None),
            period: Duration::from_secs(1),
            running: Arc::new(AtomicBool::new(false)),
            ticks: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl Clock for SystemClock {
    fn start(&mut self) {
        if self.running.load(SeqCst) {
            return; // already running
        }

        // start running
        self.running.store(true, SeqCst);

        // variables to move into closure
        let running = self.running.clone();
        let ticks = self.ticks.clone();
        let period = self.period;
        let callback = self.callback.clone();

        self.handle = Rc::new(Some(spawn(move || {
            let mut now = Instant::now();
            // The first tick occurs immediately so the clock pattern looks like
            // |-|-|-| where | are ticks and - are periods.
            // Ticks should start counting at 1 so it is equal to the
            // total number of ticks that have occurred and the value is
            // accurate for the duration of the current period.
            callback(ticks.fetch_add(1, SeqCst) + 1);
            while running.load(SeqCst) {
                // This is an inefficient way of keeping time. Polling the
                // system time without yielding will saturate a CPU core. It
                // would be better to have a Clock implementation backed by a
                // hardware timer. However, I think this is the best cross-
                // platform way to generate tick events at regular intervals
                // using only the standard library.
                if now.elapsed() >= period {
                    now = now.checked_add(period).unwrap_or_else(Instant::now);
                    // This callback must return before the next tick, otherwise
                    // future ticks will be consistently behind for the rest of
                    // execution.
                    callback(ticks.fetch_add(1, SeqCst) + 1);
                }
            }
        })));
    }

    fn stop(&mut self) {
        self.running.store(false, SeqCst);
        if let Some(reference) = Rc::get_mut(&mut self.handle) {
            if let Some(handle) = reference.take() {
                // consume handle and replace with None.
                // only panics if worker thread panics.
                // blocks here until clock is stopped.
                handle.join().unwrap();
            }
        }
    }

    fn reset(&mut self) {
        self.ticks.store(0, SeqCst);
    }

    fn is_running(&self) -> bool {
        self.running.load(SeqCst)
    }

    fn with_period(&mut self, period: Duration) -> &mut Self {
        self.period = period;
        self
    }

    fn on_tick<F>(&mut self, callback: F) -> &mut Self
    where
        F: Fn(usize) + Send + Sync + 'static,
    {
        self.callback = Arc::new(callback);
        self
    }

    fn get_ticks(&self) -> usize {
        self.ticks.load(SeqCst)
    }

    fn get_period(&self) -> Duration {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn double_calls() {
        let mut clock = SystemClock::new();
        clock.start();
        clock.start();
        assert_eq!(clock.is_running(), true);
        clock.stop();
        clock.stop();
        assert_eq!(clock.is_running(), false);
        clock.reset();
        clock.reset();
        assert_eq!(clock.get_ticks(), 0);
    }

    #[test]
    fn count_ticks() {
        let count = 10;
        let period = Duration::from_millis(10);
        let x = Arc::new(AtomicUsize::new(0));
        let y = x.clone();
        let mut clock = SystemClock::new()
            .with_period(period)
            .on_tick(move |_| {
                let _ = y.fetch_add(1, SeqCst);
            })
            .build();
        clock.start();
        sleep(count * period - period / 2);
        clock.stop();
        assert_eq!(count as usize, clock.get_ticks());
        assert_eq!(count as usize, x.load(SeqCst));
    }

    #[test]
    fn stop_start() {
        let period = Duration::from_millis(10);
        let mut clock = SystemClock::new().with_period(period).build();
        clock.start();
        sleep(3 * period / 2);
        clock.stop();
        let last_tick = clock.get_ticks();
        clock.on_tick(move |tick| assert_eq!(last_tick + 1, tick));
        clock.start();
        sleep(period / 2);
        clock.stop();
    }
}

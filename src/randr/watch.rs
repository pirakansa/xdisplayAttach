use super::X11Randr;
use crate::{
    signal, AttachError, CommandResult, DisplayConfig, ErrorKind, ExitStatus, Result, WatchOptions,
};
use std::thread;
use std::time::{Duration, Instant};
use x11rb::connection::Connection;
use x11rb::protocol::randr::{ConnectionExt as RandrConnectionExt, Notify, NotifyMask};
use x11rb::protocol::Event;

const ACTIVE_POLL_INTERVAL: Duration = Duration::from_millis(50);
const IDLE_POLL_INTERVAL: Duration = Duration::from_millis(250);

trait WatchBackend {
    fn poll_event(&self) -> Result<Option<Event>>;
    fn enforce(&self, config: &DisplayConfig) -> Result<CommandResult>;
    fn now(&self) -> Instant;
    fn sleep(&self, duration: Duration);
    fn shutdown_requested(&self) -> bool;
}

impl WatchBackend for X11Randr {
    fn poll_event(&self) -> Result<Option<Event>> {
        self.conn
            .poll_for_event()
            .map_err(|error| AttachError::xorg(format!("X event stream failed: {error}")))
    }

    fn enforce(&self, config: &DisplayConfig) -> Result<CommandResult> {
        X11Randr::enforce(self, config, false)
    }

    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep(&self, duration: Duration) {
        thread::sleep(duration);
    }

    fn shutdown_requested(&self) -> bool {
        signal::shutdown_requested()
    }
}

#[derive(Default)]
struct Debounce {
    deadline: Option<Instant>,
}

impl Debounce {
    fn observe(&mut self, now: Instant, delay: Duration) {
        self.deadline = Some(now + delay);
    }

    fn take_due(&mut self, now: Instant) -> bool {
        if self.deadline.is_some_and(|deadline| now >= deadline) {
            self.deadline = None;
            true
        } else {
            false
        }
    }

    fn wait_duration(&self, now: Instant) -> Duration {
        self.deadline.map_or(IDLE_POLL_INTERVAL, |deadline| {
            deadline
                .saturating_duration_since(now)
                .min(ACTIVE_POLL_INTERVAL)
        })
    }
}

fn run_watch(
    backend: &impl WatchBackend,
    config: &DisplayConfig,
    options: WatchOptions,
) -> Result<CommandResult> {
    eprintln!(
        "watch started: debounce={}ms retry={} retry-delay={}ms",
        options.debounce.as_millis(),
        options.retry_count,
        options.retry_delay.as_millis()
    );
    enforce_with_retry(backend, config, options, "initial")?;
    let mut debounce = Debounce::default();
    while !backend.shutdown_requested() {
        while let Some(event) = backend.poll_event()? {
            if let Some(name) = randr_event_name(&event) {
                eprintln!("watch event: {name}");
                debounce.observe(backend.now(), options.debounce);
            }
        }
        let now = backend.now();
        if debounce.take_due(now) {
            enforce_with_retry(backend, config, options, "event")?;
            continue;
        }
        backend.sleep(debounce.wait_duration(now));
    }
    eprintln!("watch stopped");
    Ok(CommandResult::without_status_line(ExitStatus::Changed))
}

fn enforce_with_retry(
    backend: &impl WatchBackend,
    config: &DisplayConfig,
    options: WatchOptions,
    phase: &str,
) -> Result<()> {
    let mut attempt = 0;
    loop {
        match backend.enforce(config) {
            Ok(result) => {
                log_watch_result(phase, &result);
                return Ok(());
            }
            Err(error)
                if should_retry_watch_error(error.kind()) && attempt < options.retry_count =>
            {
                attempt += 1;
                eprintln!(
                    "watch {phase}: {error}; retrying ({attempt}/{})",
                    options.retry_count
                );
                sleep_until_retry_or_shutdown(backend, options.retry_delay);
                if backend.shutdown_requested() {
                    return Ok(());
                }
            }
            Err(error) if should_retry_watch_error(error.kind()) => {
                eprintln!("watch {phase}: {error}; retry exhausted");
                return Ok(());
            }
            Err(error) => return Err(error),
        }
    }
}

impl X11Randr {
    pub(crate) fn watch_enforce(
        &self,
        config: &DisplayConfig,
        options: WatchOptions,
    ) -> Result<CommandResult> {
        signal::install_shutdown_handlers();
        self.subscribe_randr_events()?;
        run_watch(self, config, options)
    }

    fn subscribe_randr_events(&self) -> Result<()> {
        let setup = self.conn.setup();
        let screen = setup
            .roots
            .get(self.screen_num)
            .ok_or_else(|| AttachError::xorg("default X screen is unavailable"))?;
        let mask = NotifyMask::SCREEN_CHANGE
            | NotifyMask::CRTC_CHANGE
            | NotifyMask::OUTPUT_CHANGE
            | NotifyMask::OUTPUT_PROPERTY
            | NotifyMask::RESOURCE_CHANGE;
        self.conn
            .randr_select_input(screen.root, mask)
            .map_err(|error| AttachError::xorg(format!("RandR event selection failed: {error}")))?;
        self.conn
            .flush()
            .map_err(|error| AttachError::xorg(format!("RandR event selection failed: {error}")))?;
        Ok(())
    }
}
fn randr_event_name(event: &Event) -> Option<&'static str> {
    match event {
        Event::RandrScreenChangeNotify(_) => Some("screen-change"),
        Event::RandrNotify(event) if event.sub_code == Notify::CRTC_CHANGE => Some("crtc-change"),
        Event::RandrNotify(event) if event.sub_code == Notify::OUTPUT_CHANGE => {
            Some("output-change")
        }
        Event::RandrNotify(event) if event.sub_code == Notify::OUTPUT_PROPERTY => {
            Some("output-property")
        }
        Event::RandrNotify(event) if event.sub_code == Notify::RESOURCE_CHANGE => {
            Some("resource-change")
        }
        _ => None,
    }
}

fn should_retry_watch_error(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::Unavailable | ErrorKind::RandrFailed)
}

fn log_watch_result(phase: &str, result: &CommandResult) {
    for message in result.messages() {
        eprintln!("watch {phase}: {message}");
    }
    for warning in result.warnings() {
        eprintln!("watch {phase} warning: {warning}");
    }
    eprintln!("watch {phase}: {}", result.status());
}

fn sleep_until_retry_or_shutdown(backend: &impl WatchBackend, duration: Duration) {
    let deadline = backend.now() + duration;
    while !backend.shutdown_requested() {
        let now = backend.now();
        if now >= deadline {
            return;
        }
        backend.sleep((deadline - now).min(ACTIVE_POLL_INTERVAL));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    struct MockWatch {
        start: Instant,
        elapsed: Cell<Duration>,
        shutdown_at: Duration,
        events: RefCell<VecDeque<(Duration, Event)>>,
        results: RefCell<VecDeque<Result<CommandResult>>>,
        applications: RefCell<Vec<Duration>>,
        poll_fails: bool,
    }

    impl MockWatch {
        fn new(shutdown_at: Duration) -> Self {
            Self {
                start: Instant::now(),
                elapsed: Cell::new(Duration::ZERO),
                shutdown_at,
                events: RefCell::new(VecDeque::new()),
                results: RefCell::new(VecDeque::new()),
                applications: RefCell::new(Vec::new()),
                poll_fails: false,
            }
        }
    }

    impl WatchBackend for MockWatch {
        fn poll_event(&self) -> Result<Option<Event>> {
            if self.poll_fails {
                return Err(AttachError::xorg("X event stream failed"));
            }
            let mut events = self.events.borrow_mut();
            if events
                .front()
                .is_some_and(|(at, _)| *at <= self.elapsed.get())
            {
                return Ok(events.pop_front().map(|(_, event)| event));
            }
            Ok(None)
        }

        fn enforce(&self, _: &DisplayConfig) -> Result<CommandResult> {
            self.applications.borrow_mut().push(self.elapsed.get());
            self.results
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| Ok(CommandResult::new(ExitStatus::AlreadySatisfied)))
        }

        fn now(&self) -> Instant {
            self.start + self.elapsed.get()
        }
        fn sleep(&self, duration: Duration) {
            self.elapsed.set(self.elapsed.get() + duration);
        }
        fn shutdown_requested(&self) -> bool {
            self.elapsed.get() >= self.shutdown_at
        }
    }

    fn config() -> DisplayConfig {
        DisplayConfig {
            schema_version: None,
            outputs: Vec::new(),
        }
    }

    fn options() -> WatchOptions {
        WatchOptions {
            debounce: Duration::from_millis(100),
            retry_count: 2,
            retry_delay: Duration::from_millis(100),
        }
    }

    fn screen_change() -> Event {
        Event::RandrScreenChangeNotify(Default::default())
    }

    #[test]
    fn consecutive_events_extend_debounce_before_applying_once() {
        let backend = MockWatch::new(Duration::from_millis(300));
        backend.events.borrow_mut().extend([
            (Duration::ZERO, screen_change()),
            (Duration::from_millis(20), screen_change()),
        ]);
        run_watch(&backend, &config(), options()).unwrap();
        assert_eq!(
            *backend.applications.borrow(),
            vec![Duration::ZERO, Duration::from_millis(150)]
        );
    }

    #[test]
    fn unrelated_events_do_not_schedule_an_application() {
        let backend = MockWatch::new(Duration::from_millis(500));
        backend
            .events
            .borrow_mut()
            .push_back((Duration::ZERO, Event::Unknown(vec![])));
        run_watch(&backend, &config(), options()).unwrap();
        assert_eq!(*backend.applications.borrow(), vec![Duration::ZERO]);
    }

    #[test]
    fn retry_limit_counts_retries_after_initial_attempt() {
        let backend = MockWatch::new(Duration::from_secs(10));
        backend
            .results
            .borrow_mut()
            .extend((0..3).map(|_| Err(AttachError::randr("temporary failure"))));
        enforce_with_retry(&backend, &config(), options(), "event").unwrap();
        assert_eq!(
            *backend.applications.borrow(),
            vec![
                Duration::ZERO,
                Duration::from_millis(100),
                Duration::from_millis(200)
            ]
        );
    }

    #[test]
    fn retry_stops_after_success() {
        let backend = MockWatch::new(Duration::from_secs(10));
        backend
            .results
            .borrow_mut()
            .push_back(Err(AttachError::unavailable("temporary failure")));
        enforce_with_retry(&backend, &config(), options(), "event").unwrap();
        assert_eq!(
            *backend.applications.borrow(),
            vec![Duration::ZERO, Duration::from_millis(100)]
        );
    }

    #[test]
    fn zero_retries_does_not_wait() {
        let backend = MockWatch::new(Duration::from_secs(10));
        backend
            .results
            .borrow_mut()
            .push_back(Err(AttachError::randr("temporary failure")));
        enforce_with_retry(
            &backend,
            &config(),
            WatchOptions {
                retry_count: 0,
                ..options()
            },
            "event",
        )
        .unwrap();
        assert_eq!(*backend.applications.borrow(), vec![Duration::ZERO]);
        assert_eq!(backend.elapsed.get(), Duration::ZERO);
    }

    #[test]
    fn shutdown_during_retry_prevents_another_application() {
        let backend = MockWatch::new(Duration::from_millis(50));
        backend
            .results
            .borrow_mut()
            .push_back(Err(AttachError::randr("temporary failure")));
        enforce_with_retry(&backend, &config(), options(), "event").unwrap();
        assert_eq!(*backend.applications.borrow(), vec![Duration::ZERO]);
        assert_eq!(backend.elapsed.get(), Duration::from_millis(50));
    }

    #[test]
    fn fatal_error_is_propagated_without_retry() {
        let backend = MockWatch::new(Duration::from_secs(10));
        backend
            .results
            .borrow_mut()
            .push_back(Err(AttachError::xorg("connection lost")));
        assert_eq!(
            run_watch(&backend, &config(), options())
                .unwrap_err()
                .kind(),
            ErrorKind::XorgUnavailable
        );
        assert_eq!(*backend.applications.borrow(), vec![Duration::ZERO]);
    }

    #[test]
    fn event_stream_error_is_propagated() {
        let mut backend = MockWatch::new(Duration::from_secs(10));
        backend.poll_fails = true;
        assert_eq!(
            run_watch(&backend, &config(), options())
                .unwrap_err()
                .kind(),
            ErrorKind::XorgUnavailable
        );
    }

    #[test]
    fn watch_retries_only_event_time_operational_errors() {
        assert!(should_retry_watch_error(crate::ErrorKind::Unavailable));
        assert!(should_retry_watch_error(crate::ErrorKind::RandrFailed));
        assert!(!should_retry_watch_error(crate::ErrorKind::Usage));
        assert!(!should_retry_watch_error(crate::ErrorKind::XorgUnavailable));
    }
}

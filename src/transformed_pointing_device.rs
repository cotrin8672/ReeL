use core::future::pending;

use embassy_time::{Duration, Instant, Timer};
use embedded_hal_async::digital::Wait;
use rmk::event::{Axis, AxisEvent, AxisValType, PointingEvent};
use rmk::input_device::pointing::{InitState, PointingDriver};
use rmk::macros::input_device;

use crate::trackball_transform::TrackballTransform;

#[input_device(publish = PointingEvent)]
pub struct TransformingPointingDevice<S: PointingDriver> {
    sensor: S,
    init_state: InitState,
    poll_interval: Duration,
    id: u8,
    report_interval: Duration,
    last_poll: Instant,
    last_report: Instant,
    accumulated_x: i32,
    accumulated_y: i32,
    transform: TrackballTransform,
}

impl<S: PointingDriver> TransformingPointingDevice<S> {
    const MAX_INIT_RETRIES: u8 = 3;
    const DEFAULT_POLL_INTERVAL_US: u64 = 500;

    pub fn with_report_hz(id: u8, sensor: S, report_hz: u16) -> Self {
        let report_interval = Duration::from_hz(report_hz as u64);

        Self {
            sensor,
            init_state: InitState::Pending,
            poll_interval: Duration::from_micros(Self::DEFAULT_POLL_INTERVAL_US)
                .min(report_interval),
            id,
            report_interval,
            last_poll: Instant::MIN,
            last_report: Instant::MIN,
            accumulated_x: 0,
            accumulated_y: 0,
            transform: TrackballTransform::new(),
        }
    }

    async fn try_init(&mut self) -> bool {
        match self.init_state {
            InitState::Ready => return true,
            InitState::Failed => return false,
            InitState::Pending => {
                self.init_state = InitState::Initializing(0);
            }
            InitState::Initializing(_) => {}
        }

        if let InitState::Initializing(retry_count) = self.init_state {
            match self.sensor.init().await {
                Ok(()) => {
                    self.init_state = InitState::Ready;
                    return true;
                }
                Err(_) => {
                    if retry_count + 1 >= Self::MAX_INIT_RETRIES {
                        self.init_state = InitState::Failed;
                        return false;
                    }
                    self.init_state = InitState::Initializing(retry_count + 1);
                    Timer::after(Duration::from_millis(100)).await;
                }
            }
        }

        false
    }

    async fn poll_once(&mut self) {
        if self.init_state != InitState::Ready && !self.try_init().await {
            return;
        }

        if !self.sensor.motion_pending() {
            return;
        }

        if let Ok(motion) = self.sensor.read_motion().await {
            self.accumulated_x = self.accumulated_x.saturating_add(i32::from(motion.dx));
            self.accumulated_y = self.accumulated_y.saturating_add(i32::from(motion.dy));
        }
    }

    fn take_report_event(&mut self) -> Option<PointingEvent> {
        if self.accumulated_x == 0 && self.accumulated_y == 0 {
            return None;
        }

        let raw_x = self.accumulated_x.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        let raw_y = self.accumulated_y.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
        self.accumulated_x = 0;
        self.accumulated_y = 0;

        let (x, y) = self.transform.apply(raw_x, raw_y);

        Some(PointingEvent {
            device_id: self.id,
            axes: [
                AxisEvent {
                    typ: AxisValType::Rel,
                    axis: Axis::X,
                    value: x,
                },
                AxisEvent {
                    typ: AxisValType::Rel,
                    axis: Axis::Y,
                    value: y,
                },
                AxisEvent {
                    typ: AxisValType::Rel,
                    axis: Axis::Z,
                    value: 0,
                },
            ],
        })
    }

    async fn read_pointing_event(&mut self) -> PointingEvent {
        use rmk::embassy_futures::select::{Either, select};

        if self.last_poll == Instant::MIN {
            self.last_poll = Instant::now();
        }
        if self.last_report == Instant::MIN {
            self.last_report = Instant::now();
        }

        loop {
            let poll_wait = async {
                if let Some(gpio) = self.sensor.motion_gpio() {
                    let _ = gpio.wait_for_low().await;
                } else {
                    Timer::after(
                        self.poll_interval
                            .checked_sub(self.last_poll.elapsed())
                            .unwrap_or(Duration::MIN),
                    )
                    .await;
                }
            };

            let report_wait = async {
                if self.accumulated_x != 0 || self.accumulated_y != 0 {
                    Timer::after(
                        self.report_interval
                            .checked_sub(self.last_report.elapsed())
                            .unwrap_or(Duration::MIN),
                    )
                    .await;
                } else {
                    pending::<()>().await;
                }
            };

            match select(poll_wait, report_wait).await {
                Either::First(_) => {
                    self.poll_once().await;
                    self.last_poll = Instant::now();
                }
                Either::Second(_) => {
                    if let Some(event) = self.take_report_event() {
                        self.last_report = Instant::now();
                        return event;
                    }
                }
            }
        }
    }
}

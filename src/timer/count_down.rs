use super::*;

use cast::u16;
use fugit::{MicrosDurationU32, TimerDurationU32};

/// Marker trait that indicates that a timer is periodic
pub trait Periodic {}

/// A count down timer
pub trait CountDown {
    /// An enumeration of `CountDown` errors.
    ///
    /// For infallible implementations, will be `Infallible`
    type Error: core::fmt::Debug;

    /// Starts a new count down
    fn start<const F: u32>(&mut self, count: TimerDurationU32<F>) -> Result<(), Self::Error>;

    /// Non-blockingly "waits" until the count down finishes
    ///
    /// # Contract
    ///
    /// - If `Self: Periodic`, the timer will start a new count down right after the last one
    /// finishes.
    /// - Otherwise the behavior of calling `wait` after the last call returned `Ok` is UNSPECIFIED.
    /// Implementers are suggested to panic on this scenario to signal a programmer error.
    fn wait(&mut self) -> nb::Result<(), Self::Error>;
}

impl<T> CountDown for &mut T
where
    T: CountDown,
{
    type Error = T::Error;

    fn start<const F: u32>(&mut self, count: TimerDurationU32<F>) -> Result<(), Self::Error> {
        T::start::<F>(self, count)
    }

    fn wait(&mut self) -> nb::Result<(), Self::Error> {
        T::wait(self)
    }
}

/// Trait for cancelable countdowns.
pub trait Cancel: CountDown {
    /// Tries to cancel this countdown.
    ///
    /// # Errors
    ///
    /// An error will be returned if the countdown has already been canceled or was never started.
    /// An error is also returned if the countdown is not `Periodic` and has already expired.
    fn cancel(&mut self) -> Result<(), Self::Error>;
}

impl<T> Cancel for &mut T
where
    T: Cancel,
{
    fn cancel(&mut self) -> Result<(), Self::Error> {
        T::cancel(self)
    }
}

/// Timer that waits given time
pub struct CountDownTimer<TIM, const FREQ: u32> {
    tim: TIM,
}

/// `CountDownTimer` with sampling of 1 MHz
pub type CountDownTimerUs<TIM> = CountDownTimer<TIM, 1_000_000>;

/// `CountDownTimer` with sampling of 1 kHz
///
/// NOTE: don't use this if your system frequency more than 65 MHz
pub type CountDownTimerMs<TIM> = CountDownTimer<TIM, 1_000>;

impl<TIM> Timer<TIM>
where
    TIM: Instance,
{
    /// Creates CountDownTimer with custom sampling
    pub fn count_down<const FREQ: u32>(self) -> CountDownTimer<TIM, FREQ> {
        let Self { tim, clk } = self;
        CountDownTimer::<TIM, FREQ>::new(tim, clk)
    }
    /// Creates CountDownTimer with sampling of 1 MHz
    pub fn count_down_us(self) -> CountDownTimerUs<TIM> {
        self.count_down::<1_000_000>()
    }

    /// Creates CountDownTimer with sampling of 1 kHz
    ///
    /// NOTE: don't use this if your system frequency more than 65 MHz
    pub fn count_down_ms(self) -> CountDownTimerMs<TIM> {
        self.count_down::<1_000>()
    }
}

impl<TIM, const FREQ: u32> Periodic for CountDownTimer<TIM, FREQ> {}

impl Timer<SYST> {
    /// Creates SysCountDownTimer
    pub fn count_down(self) -> SysCountDownTimer {
        let Self { tim, clk } = self;
        SysCountDownTimer { tim, clk }
    }
}

pub struct SysCountDownTimer {
    tim: SYST,
    clk: Hertz,
}

impl SysCountDownTimer {
    /// Starts listening for an `event`
    pub fn listen(&mut self, event: Event) {
        match event {
            Event::TimeOut => self.tim.enable_interrupt(),
        }
    }

    /// Stops listening for an `event`
    pub fn unlisten(&mut self, event: Event) {
        match event {
            Event::TimeOut => self.tim.disable_interrupt(),
        }
    }
}

impl SysCountDownTimer {
    pub fn start(&mut self, timeout: MicrosDurationU32) -> Result<(), Error> {
        let mul = self.clk.0 / 1_000_000;
        let rvr = timeout.ticks() * mul - 1;

        assert!(rvr < (1 << 24));

        self.tim.set_reload(rvr);
        self.tim.clear_current();
        self.tim.enable_counter();
        Ok(())
    }

    pub fn wait(&mut self) -> nb::Result<(), Error> {
        if self.tim.has_wrapped() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    pub fn cancel(&mut self) -> Result<(), Error> {
        if !self.tim.is_counter_enabled() {
            return Err(Error::Disabled);
        }

        self.tim.disable_counter();
        Ok(())
    }
}

impl CountDown for SysCountDownTimer {
    type Error = Error;

    fn start<const F: u32>(&mut self, timeout: TimerDurationU32<F>) -> Result<(), Self::Error> {
        self.start(timeout.convert())
    }

    fn wait(&mut self) -> nb::Result<(), Self::Error> {
        self.wait()
    }
}

impl Cancel for SysCountDownTimer {
    fn cancel(&mut self) -> Result<(), Self::Error> {
        self.cancel()
    }
}

impl<TIM, const FREQ: u32> CountDownTimer<TIM, FREQ>
where
    TIM: General,
{
    fn new(mut tim: TIM, clk: Hertz) -> Self {
        let psc = clk.0 / FREQ - 1;
        tim.set_prescaler(u16(psc).unwrap());
        Self { tim }
    }

    /// Starts listening for an `event`
    ///
    /// Note, you will also have to enable the TIM2 interrupt in the NVIC to start
    /// receiving events.
    pub fn listen(&mut self, event: Event) {
        match event {
            Event::TimeOut => {
                // Enable update event interrupt
                self.tim.listen_update_interrupt(true);
            }
        }
    }

    /// Clears interrupt associated with `event`.
    ///
    /// If the interrupt is not cleared, it will immediately retrigger after
    /// the ISR has finished.
    pub fn clear_interrupt(&mut self, event: Event) {
        match event {
            Event::TimeOut => {
                // Clear interrupt flag
                self.tim.clear_update_interrupt_flag();
            }
        }
    }

    /// Stops listening for an `event`
    pub fn unlisten(&mut self, event: Event) {
        match event {
            Event::TimeOut => {
                // Disable update event interrupt
                self.tim.listen_update_interrupt(false);
            }
        }
    }

    /// Releases the TIM peripheral
    pub fn release(mut self) -> TIM {
        // pause counter
        self.tim.disable_counter();
        self.tim
    }
}

impl<TIM, const FREQ: u32> CountDownTimer<TIM, FREQ>
where
    TIM: General,
{
    pub fn start(&mut self, timeout: TimerDurationU32<FREQ>) -> Result<(), Error> {
        // pause
        self.tim.disable_counter();
        // reset counter
        self.tim.reset_counter();

        let arr = timeout.ticks() - 1;
        self.tim.set_auto_reload(arr)?;

        // Trigger update event to load the registers
        self.tim.trigger_update();

        // start counter
        self.tim.enable_counter();

        Ok(())
    }

    pub fn wait(&mut self) -> nb::Result<(), Error> {
        if self.tim.get_update_interrupt_flag() {
            Err(nb::Error::WouldBlock)
        } else {
            self.tim.clear_update_interrupt_flag();
            Ok(())
        }
    }

    pub fn cancel(&mut self) -> Result<(), Error> {
        if !self.tim.is_counter_enabled() {
            return Err(Error::Disabled);
        }

        // disable counter
        self.tim.disable_counter();
        Ok(())
    }
}

impl<TIM, const FREQ: u32> CountDown for CountDownTimer<TIM, FREQ>
where
    TIM: General,
{
    type Error = Error;

    fn start<const F: u32>(&mut self, timeout: TimerDurationU32<F>) -> Result<(), Self::Error> {
        self.start(timeout.convert())
    }

    fn wait(&mut self) -> nb::Result<(), Self::Error> {
        self.wait()
    }
}

impl<TIM, const FREQ: u32> Cancel for CountDownTimer<TIM, FREQ>
where
    TIM: General,
{
    fn cancel(&mut self) -> Result<(), Self::Error> {
        self.cancel()
    }
}

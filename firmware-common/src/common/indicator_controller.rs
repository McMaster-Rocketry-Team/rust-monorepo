use crate::{Clock, Indicator};
use embedded_hal_async::delay::DelayNs;
use futures::join;

pub struct IndicatorController<
    R: Indicator,
    G: Indicator,
    B: Indicator,
    T: Clock,
    DL: DelayNs + Copy,
> {
    red: R,
    green: G,
    blue: B,
    clock: T,
    delay: DL,
}

impl<R: Indicator, G: Indicator, B: Indicator, T: Clock, DL: DelayNs + Copy>
    IndicatorController<R, G, B, T, DL>
{
    pub fn new(red: R, green: G, blue: B, clock: T, delay: DL) -> Self {
        Self {
            red,
            green,
            blue,
            clock,
            delay,
        }
    }

    async fn run_single<const N: usize>(
        indicator: &mut impl Indicator,
        pattern: [u16; N],
        clock: T,
        mut delay: DL,
    ) {
        if pattern.len() == 0 {
            return;
        }
        let mut start_time = clock.now_ms();
        loop {
            let mut is_enabled = true;
            for duration in pattern.iter() {
                if *duration == 0 {
                    is_enabled = !is_enabled;
                    continue;
                }
                indicator.set_enable(is_enabled).await;
                is_enabled = !is_enabled;
                let end_time = start_time + *duration as f64;
                delay.delay_ms((end_time - clock.now_ms()) as u32).await;
                start_time = end_time;
            }
        }
    }

    pub async fn reset(&mut self) {
        self.red.set_enable(false).await;
        self.green.set_enable(false).await;
        self.blue.set_enable(false).await;
    }

    pub async fn run<const RN: usize, const GN: usize, const BN: usize>(
        &mut self,
        red_pattern: [u16; RN],
        green_pattern: [u16; GN],
        blue_pattern: [u16; BN],
    ) -> ! {
        let red_fut = Self::run_single(&mut self.red, red_pattern, self.clock, self.delay);
        let green_fut = Self::run_single(&mut self.green, green_pattern, self.clock, self.delay);
        let blue_fut = Self::run_single(&mut self.blue, blue_pattern, self.clock, self.delay);
        join!(red_fut, green_fut, blue_fut);
        log_unreachable!()
    }
}

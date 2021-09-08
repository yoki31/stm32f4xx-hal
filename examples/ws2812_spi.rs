#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_halt as _;
use stm32f4xx_hal as hal;

use cortex_m_rt::entry;
use hal::{gpio::NoPin, pac, prelude::*, spi::Spi};
use smart_leds::{SmartLedsWrite, RGB};
use ws2812_spi as ws2812;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().expect("cannot take peripherals");
    let cp = cortex_m::Peripherals::take().expect("cannot take core peripherals");

    // Configure APB bus clock to 48MHz, cause ws2812 requires 3Mbps SPI
    let rcc = dp.RCC.constrain();
    let clocks = rcc.cfgr.sysclk(48.mhz()).freeze();

    let mut delay = hal::delay::Delay::new(cp.SYST, &clocks);
    let gpioa = dp.GPIOA.split();

    let spi = Spi::new(
        dp.SPI1,
        (NoPin, NoPin, gpioa.pa7),
        ws2812::MODE,
        3.mhz(),
        clocks,
    );
    let mut ws = ws2812::Ws2812::new(spi);

    let mut cnt: usize = 0;
    let mut data: [RGB<u8>; 64] = [RGB::default(); 64];
    loop {
        for (idx, color) in data.iter_mut().enumerate() {
            *color = match (cnt + idx) % 8 {
                0 => RGB { r: 8, g: 0, b: 0 },
                1 => RGB { r: 0, g: 4, b: 0 },
                2 => RGB { r: 0, g: 0, b: 2 },
                _ => RGB { r: 0, g: 0, b: 0 },
            };
        }
        ws.write(data.iter().cloned()).unwrap();
        cnt += 1;
        delay.delay_ms(50_u16);
    }
}

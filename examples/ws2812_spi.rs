#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_halt as _;
use stm32f4xx_hal as hal;

use cortex_m_rt::entry;
use hal::{gpio::NoPin, pac, prelude::*, spi::Spi};
use smart_leds::{SmartLedsWrite,
    hsv::{hsv2rgb, Hsv},
};
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
    let gpioc = dp.GPIOC.split();
    let mut pc13 = gpioc.pc13.into_push_pull_output();

    let spi = Spi::new(
        dp.SPI1,
        (NoPin, NoPin, gpioa.pa7),
        ws2812::MODE,
        3.mhz(),
        clocks,
    );
    // Holds the colour values

    let mut ws = ws2812::Ws2812::new(spi);

    const NUM_LEDS: usize = 8;
    const BRIGHTNESS: u8 = 50;
    const SATURATION: u8 = 255;
    let mut leds: [Hsv; NUM_LEDS] = [ Hsv{hue: 0, sat: 0, val: 0}; NUM_LEDS ];
    loop {
        
      for _j in 0..8 {
        for i in 0..NUM_LEDS {
          leds[i] = Hsv{hue: ((i * 32) as u8) + 32, sat: SATURATION, val: BRIGHTNESS};
          /* The higher the value 4 the less fade there is and vice versa */ 
        }
        let rgb_iterator = leds.iter().cloned().map(hsv2rgb);
        ws.write(rgb_iterator).unwrap();
        delay.delay_ms(200_u16); /* Change this to your hearts desire, the lower the value the faster your colors move (and vice versa) */
        pc13.toggle();
      }
        
    }
}

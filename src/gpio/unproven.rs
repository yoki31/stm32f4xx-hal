use super::*;

pub struct FourBitOutputPort<const P: char, const N0: u8, const N1: u8, const N2: u8, const N3: u8>
{
    pub d0: PX<Output<PushPull>, P, N0>,
    pub d1: PX<Output<PushPull>, P, N1>,
    pub d2: PX<Output<PushPull>, P, N2>,
    pub d3: PX<Output<PushPull>, P, N3>,
}

impl<const P: char, const N0: u8, const N1: u8, const N2: u8, const N3: u8>
    FourBitOutputPort<P, N0, N1, N2, N3>
{
    const fn new(
        d0: PX<Output<PushPull>, P, N0>,
        d1: PX<Output<PushPull>, P, N1>,
        d2: PX<Output<PushPull>, P, N2>,
        d3: PX<Output<PushPull>, P, N3>,
    ) {
        Self { d0, d1, d2, d3 }
    }

    const fn value_for_write_bsrr(val: u32) -> u32 {
        let b0 = (val & 0b1) != 0;
        let b1 = ((val >> 1) & 0b1) != 0;
        let b2 = ((val >> 2) & 0b1) != 0;
        let b3 = ((val >> 3) & 0b1) != 0;
        1 << (if b0 { N0 } else { N0 + 16 })
            | 1 << (if b1 { N1 } else { N1 + 16 })
            | 1 << (if b2 { N2 } else { N2 + 16 })
            | 1 << (if b3 { N3 } else { N3 + 16 })
    }
    pub fn write_u8(&mut self, word: u8) {
        unsafe {
            (*Gpio::<P>::ptr())
                .bsrr
                .write(|w| w.bits(Self::value_for_write_bsrr(word as u32)))
        }
    }
}

#[repr(C)] // NOTE: Without repr(C) alignment causes corruption
#[derive(Debug, Clone)]
// NOTE: The vergilius generated struct is incorrectly sized
pub struct IdtEntry64 {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_middle: u16,
    offset_high: u32,
    _reserved: u32,
}

impl IdtEntry64 {
    pub fn offset(&self) -> u64 {
        (self.offset_high as u64) << 32 | (self.offset_middle as u64) << 16 | self.offset_low as u64
    }

    pub fn selector(&self) -> u16 {
        self.selector
    }

    pub fn ist(&self) -> u8 {
        self.ist & 0b111
    }

    pub fn gate_type(&self) -> u8 {
        self.type_attr & 0b1111
    }

    pub fn dpl(&self) -> u8 {
        (self.type_attr >> 5) & 0b11
    }

    pub fn present(&self) -> bool {
        (self.type_attr >> 7) & 1 == 1
    }
}

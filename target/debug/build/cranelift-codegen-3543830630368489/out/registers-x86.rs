pub static INFO: RegInfo = RegInfo {
    banks: &[
        RegBank {
            name: "FloatRegs",
            first_unit: 0,
            units: 16,
            names: &[],
            prefix: "xmm",
            first_toprc: 1,
            num_toprcs: 1,
            pressure_tracking: true,
        },
        RegBank {
            name: "IntRegs",
            first_unit: 16,
            units: 16,
            names: &["rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi"],
            prefix: "r",
            first_toprc: 0,
            num_toprcs: 1,
            pressure_tracking: true,
        },
        RegBank {
            name: "FlagRegs",
            first_unit: 32,
            units: 1,
            names: &["rflags"],
            prefix: "",
            first_toprc: 2,
            num_toprcs: 1,
            pressure_tracking: false,
        },
    ],
    classes: &[
        &GPR_DATA,
        &FPR_DATA,
        &FLAG_DATA,
        &GPR8_DATA,
        &ABCD_DATA,
        &FPR8_DATA,
    ],
};
pub static GPR_DATA: RegClassData = RegClassData {
    name: "GPR",
    index: 0,
    width: 1,
    bank: 1,
    toprc: 0,
    first: 16,
    subclasses: 0x19,
    mask: [0xffff0000, 0x00000000, 0x00000000],
    pinned_reg: Some(31),
    info: &INFO,
};
#[allow(dead_code)]
pub static GPR: RegClass = &GPR_DATA;
pub static FPR_DATA: RegClassData = RegClassData {
    name: "FPR",
    index: 1,
    width: 1,
    bank: 0,
    toprc: 1,
    first: 0,
    subclasses: 0x22,
    mask: [0x0000ffff, 0x00000000, 0x00000000],
    pinned_reg: None,
    info: &INFO,
};
#[allow(dead_code)]
pub static FPR: RegClass = &FPR_DATA;
pub static FLAG_DATA: RegClassData = RegClassData {
    name: "FLAG",
    index: 2,
    width: 1,
    bank: 2,
    toprc: 2,
    first: 32,
    subclasses: 0x4,
    mask: [0x00000000, 0x00000001, 0x00000000],
    pinned_reg: None,
    info: &INFO,
};
#[allow(dead_code)]
pub static FLAG: RegClass = &FLAG_DATA;
pub static GPR8_DATA: RegClassData = RegClassData {
    name: "GPR8",
    index: 3,
    width: 1,
    bank: 1,
    toprc: 0,
    first: 16,
    subclasses: 0x18,
    mask: [0x00ff0000, 0x00000000, 0x00000000],
    pinned_reg: Some(31),
    info: &INFO,
};
#[allow(dead_code)]
pub static GPR8: RegClass = &GPR8_DATA;
pub static ABCD_DATA: RegClassData = RegClassData {
    name: "ABCD",
    index: 4,
    width: 1,
    bank: 1,
    toprc: 0,
    first: 16,
    subclasses: 0x10,
    mask: [0x000f0000, 0x00000000, 0x00000000],
    pinned_reg: Some(31),
    info: &INFO,
};
#[allow(dead_code)]
pub static ABCD: RegClass = &ABCD_DATA;
pub static FPR8_DATA: RegClassData = RegClassData {
    name: "FPR8",
    index: 5,
    width: 1,
    bank: 0,
    toprc: 1,
    first: 0,
    subclasses: 0x20,
    mask: [0x000000ff, 0x00000000, 0x00000000],
    pinned_reg: None,
    info: &INFO,
};
#[allow(dead_code)]
pub static FPR8: RegClass = &FPR8_DATA;
#[allow(dead_code, non_camel_case_types)]
#[derive(Clone, Copy)]
pub enum RU {
    xmm0 = 0,
    xmm1 = 1,
    xmm2 = 2,
    xmm3 = 3,
    xmm4 = 4,
    xmm5 = 5,
    xmm6 = 6,
    xmm7 = 7,
    xmm8 = 8,
    xmm9 = 9,
    xmm10 = 10,
    xmm11 = 11,
    xmm12 = 12,
    xmm13 = 13,
    xmm14 = 14,
    xmm15 = 15,
    rax = 16,
    rcx = 17,
    rdx = 18,
    rbx = 19,
    rsp = 20,
    rbp = 21,
    rsi = 22,
    rdi = 23,
    r8 = 24,
    r9 = 25,
    r10 = 26,
    r11 = 27,
    r12 = 28,
    r13 = 29,
    r14 = 30,
    r15 = 31,
    rflags = 32,
}
impl Into<RegUnit> for RU {
    fn into(self) -> RegUnit {
        self as RegUnit
    }
}

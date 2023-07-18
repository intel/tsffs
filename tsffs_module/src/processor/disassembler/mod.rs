pub mod x86_64;

#[derive(Debug)]
pub enum CmpExpr {
    Deref(Box<CmpExpr>),
    Reg(String),
    Mul(Box<CmpExpr>, Box<CmpExpr>),
    Add(Box<CmpExpr>, Box<CmpExpr>),
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    Addr(u64),
}

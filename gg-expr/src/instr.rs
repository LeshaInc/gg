#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Instr {
    Nop,
    Panic,
    Copy {
        src: RegId,
        dst: RegId,
    },
    LoadConst {
        id: ConstId,
        dst: RegId,
    },
    NewList {
        seq: RegSeq,
        dst: RegId,
    },
    NewMap {
        seq: RegSeq,
        dst: RegId,
    },
    Jump {
        offset: InstrOffset,
    },
    JumpIfTrue {
        cond: RegId,
        offset: InstrOffset,
    },
    JumpIfFalse {
        cond: RegId,
        offset: InstrOffset,
    },
    BinOp {
        op: BinOp,
        lhs: RegId,
        rhs: RegId,
        dst: RegId,
    },
    UnOp {
        op: UnOp,
        arg: RegId,
        dst: RegId,
    },
    Call {
        seq: RegSeq,
        dst: RegId,
    },
}

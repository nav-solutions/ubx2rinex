use gnss::prelude::SV;
use hifitime::Epoch;

pub struct Rawxm {
    pub t: Epoch,
    pub sv: SV,
    pub pr: f64,
    pub cp: f64,
    pub dop: f32,
    pub cno: u8,
}

impl std::fmt::Display for Rawxm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}({}) pr={:.7E} cp={:.7E} dop={:.7E} cno={}",
            self.t, self.sv, self.pr, self.cp, self.dop, self.cno,
        )
    }
}

impl Rawxm {
    pub fn new(t: Epoch, sv: SV, pr: f64, cp: f64, dop: f32, cno: u8) -> Self {
        Self {
            t,
            sv,
            pr,
            cp,
            dop,
            cno,
        }
    }
}

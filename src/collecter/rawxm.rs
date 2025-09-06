use rinex::prelude::{Epoch, SV};

#[derive(Debug, Clone, Copy)]
pub struct Rawxm {
    /// [Epoch] of observation
    pub epoch: Epoch,

    /// [SV]
    pub sv: SV,

    /// freq_id
    pub freq_id: u8,

    /// PR measurement
    pub pr: f64,

    /// CP measurement
    pub cp: f64,

    /// DOP measurement
    pub dop: f32,

    /// CNO
    pub cno: u8,
}

impl std::fmt::Display for Rawxm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}({}) freq_id={} pr={:.7E} cp={:.7E} dop={:.7E} cno={}",
            self.epoch, self.sv, self.freq_id, self.pr, self.cp, self.dop, self.cno,
        )
    }
}

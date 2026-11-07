mod placed;
mod protocol;
mod server;

pub use placed::{CollectorFolder, Placed, place};
pub use protocol::{PORT, PROTOCOL};
pub(crate) use server::SILENCE;
pub use server::serve;

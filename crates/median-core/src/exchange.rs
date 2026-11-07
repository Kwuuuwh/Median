/// Source that confirms a recorded trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// The user entered the trade by hand.
    Hand,
    /// The game log confirmed the trade.
    Stream,
}

impl Origin {
    /// Value stored for this origin in the account store, e.g. `"hand"`.
    pub fn word(self) -> &'static str {
        match self {
            Origin::Hand => "hand",
            Origin::Stream => "stream",
        }
    }
}

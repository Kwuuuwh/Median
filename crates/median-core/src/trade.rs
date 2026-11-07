use crate::worldstate::Vendor;

/// Currency a vendor price is paid in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Currency {
    Ducats,
    Credits,
    Aya,
    RegalAya,
}

/// Amount in one currency.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Price {
    pub currency: Currency,
    pub amount: u32,
}

/// One item a vendor offers, with its prices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    /// Item key in the catalog's spelling.
    pub item: String,
    pub prices: Vec<Price>,
}

/// Items a rotating vendor sells during its current visit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shelf {
    pub vendor: Vendor,
    pub offers: Vec<Offer>,
}

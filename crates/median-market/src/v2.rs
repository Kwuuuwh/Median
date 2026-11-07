use serde::Deserialize;

use crate::book::Book;
use crate::error::MarketError;
use crate::gate::Pass;
use crate::http::Client;
use crate::model::{ItemMeta, ItemRef, Order};

/// v2 wraps every response in `{ apiVersion, data, error }`.
#[derive(Debug, Deserialize)]
struct Envelope<T> {
    data: T,
}

impl Client {
    /// Metadata of one item, under a kilobyte. Only this endpoint states whether the item has
    /// ranks, subtypes or stars.
    pub async fn item(&self, pass: Pass, slug: &str) -> Result<ItemMeta, MarketError> {
        let url = self.v2(&format!("/item/{slug}"));
        let answer: Envelope<ItemMeta> = self.get(pass, url, slug).await?;
        Ok(answer.data)
    }

    /// Every item warframe.market lists, 1.6 MB in one response. Only this endpoint maps item ids
    /// to slugs without one request per item.
    pub async fn items(&self, pass: Pass) -> Result<Vec<ItemRef>, MarketError> {
        let url = self.v2("/items");
        let answer: Envelope<Vec<ItemRef>> = self.get(pass, url, "items").await?;
        Ok(answer.data)
    }

    /// Orders recently placed across the whole market, newest first. Needs no token.
    pub async fn recent(&self, pass: Pass) -> Result<Vec<Order>, MarketError> {
        let url = self.v2("/orders/recent");
        let answer: Envelope<Vec<Order>> = self.get(pass, url, "recent orders").await?;
        Ok(answer.data)
    }

    /// Every open order on one item, all variants at once, 100–200 KB per response.
    pub async fn book(&self, pass: Pass, slug: &str) -> Result<Book, MarketError> {
        let url = self.v2(&format!("/orders/item/{slug}"));
        let answer: Envelope<Vec<Order>> = self.get(pass, url, slug).await?;
        Ok(Book::from(answer.data))
    }
}

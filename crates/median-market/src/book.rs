use median_core::{Seller, held_ask};

use crate::model::{Order, Selector, Side};

/// Every open order on an item, all variants together.
#[derive(Debug, Clone, PartialEq)]
pub struct Book {
    orders: Vec<Order>,
}

impl From<Vec<Order>> for Book {
    fn from(orders: Vec<Order>) -> Self {
        Self { orders }
    }
}

/// Price summary of one variant's open orders at one moment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Snapshot {
    /// Lowest sell price from any seller.
    pub lowest: Option<u32>,
    /// Lowest sell price among sellers in game.
    pub lowest_ingame: Option<u32>,
    /// Lowest in-game sell price the market holds at, ignoring a lone give-away price below it
    /// (`median_core::held_ask`).
    pub held_ingame: Option<u32>,
    /// Highest buy price among buyers in game.
    pub highest_bid_ingame: Option<u32>,
    /// Number of sell orders from sellers in game.
    pub sellers_ingame: usize,
}

impl Book {
    pub fn rows(&self) -> &[Order] {
        &self.orders
    }

    /// Rows of one variant, in the order the market listed them.
    pub fn of(&self, selector: &Selector) -> impl Iterator<Item = &Order> {
        self.orders
            .iter()
            .filter(move |order| matches(order, selector))
    }

    /// Summarises the rows `keep` accepts into one snapshot.
    pub fn summarise(&self, keep: impl Fn(&Order) -> bool) -> Snapshot {
        let mut snapshot = Snapshot::default();
        let mut sellers = 0;
        let mut reachable_sellers = Vec::new();
        for order in self.orders.iter().filter(|order| keep(order)) {
            let reachable = order.user.status.reachable();
            match order.side {
                Side::Sell => {
                    snapshot.lowest = least(snapshot.lowest, order.platinum);
                    if reachable {
                        snapshot.lowest_ingame = least(snapshot.lowest_ingame, order.platinum);
                        reachable_sellers.push(Seller {
                            platinum: order.platinum,
                            quantity: order.quantity,
                            name: &order.user.name,
                        });
                        sellers += 1;
                    }
                }
                Side::Buy if reachable => {
                    snapshot.highest_bid_ingame =
                        Some(snapshot.highest_bid_ingame.unwrap_or(0).max(order.platinum));
                }
                Side::Buy => {}
            }
        }
        snapshot.held_ingame = held_ask(&reachable_sellers);
        snapshot.sellers_ingame = sellers;
        snapshot
    }

    /// The snapshot of one variant.
    pub fn snapshot(&self, selector: &Selector) -> Snapshot {
        self.summarise(|order| matches(order, selector))
    }
}

fn least(current: Option<u32>, platinum: u32) -> Option<u32> {
    Some(current.map_or(platinum, |seen| seen.min(platinum)))
}

fn matches(order: &Order, selector: &Selector) -> bool {
    match selector {
        Selector::Whole => true,
        Selector::Rank(rank) => order.rank == Some(*rank),
        Selector::Subtype(subtype) => order.subtype.as_deref() == Some(subtype.as_str()),
        Selector::Both { rank, subtype } => {
            order.rank == Some(*rank) && order.subtype.as_deref() == Some(subtype.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(side: &str, platinum: u32, status: &str, variant: &str) -> String {
        format!(
            r#"{{"id":"{side}{platinum}{status}","type":"{side}","platinum":{platinum},
                "quantity":1,{variant}"createdAt":"2017-12-12T19:58:08Z",
                "updatedAt":"2026-08-09T18:17:46Z",
                "user":{{"ingameName":"p{platinum}","status":"{status}"}}}}"#
        )
    }

    fn book(rows: &[String]) -> Book {
        let orders: Vec<Order> =
            serde_json::from_str(&format!("[{}]", rows.join(","))).expect("orders parse");
        Book::from(orders)
    }

    /// A cheap offline seller below an online one and an in-game one, plus two bids.
    fn mixed_statuses() -> Book {
        book(&[
            row("sell", 13, "offline", ""),
            row("sell", 18, "online", ""),
            row("sell", 20, "ingame", ""),
            row("buy", 15, "ingame", ""),
            row("buy", 17, "offline", ""),
        ])
    }

    #[test]
    fn the_cheapest_reachable_seller_is_not_the_cheapest_seller() {
        let book = mixed_statuses();
        assert_eq!(book.snapshot(&Selector::Whole).lowest, Some(13));
        assert_eq!(book.snapshot(&Selector::Whole).lowest_ingame, Some(20));
    }

    #[test]
    fn the_market_stands_over_a_lone_give_away() {
        let book = book(&[
            row("sell", 110, "ingame", ""),
            row("sell", 20, "ingame", ""),
            row("sell", 15, "offline", ""),
            row("sell", 98, "ingame", ""),
        ]);
        let snapshot = book.snapshot(&Selector::Whole);
        assert_eq!(snapshot.lowest_ingame, Some(20));
        assert_eq!(snapshot.held_ingame, Some(98));
        assert_eq!(
            mixed_statuses().snapshot(&Selector::Whole).held_ingame,
            Some(20)
        );
    }

    #[test]
    fn bids_count_only_from_players_who_can_be_reached() {
        let book = mixed_statuses();
        assert_eq!(book.snapshot(&Selector::Whole).highest_bid_ingame, Some(15));
    }

    #[test]
    fn reachable_sellers_are_counted_not_all_of_them() {
        let book = mixed_statuses();
        assert_eq!(book.rows().len(), 5);
        assert_eq!(book.snapshot(&Selector::Whole).sellers_ingame, 1);
    }

    #[test]
    fn one_answer_carries_every_variant_and_each_is_read_apart() {
        let book = book(&[
            row("sell", 60, "ingame", r#""rank":0,"#),
            row("sell", 70, "online", r#""rank":0,"#),
            row("sell", 120, "ingame", r#""rank":10,"#),
            row("buy", 60, "ingame", r#""rank":10,"#),
        ]);
        assert_eq!(book.snapshot(&Selector::Rank(0)).lowest_ingame, Some(60));
        assert_eq!(book.snapshot(&Selector::Rank(10)).lowest_ingame, Some(120));
        assert_eq!(
            book.snapshot(&Selector::Rank(10)).highest_bid_ingame,
            Some(60)
        );
        assert_eq!(book.snapshot(&Selector::Rank(0)).highest_bid_ingame, None);
        // Without a variant filter, the same rows give a lowest sell of 60 and a top bid of 60.
        assert_eq!(book.snapshot(&Selector::Whole).lowest_ingame, Some(60));
    }

    #[test]
    fn subtypes_are_filtered_the_same_way() {
        let book = book(&[
            row("sell", 11, "ingame", r#""subtype":"intact","#),
            row("sell", 30, "ingame", r#""subtype":"radiant","#),
        ]);
        assert_eq!(
            book.snapshot(&Selector::Subtype("radiant".into()))
                .lowest_ingame,
            Some(30)
        );
        assert_eq!(
            book.snapshot(&Selector::Subtype("intact".into()))
                .lowest_ingame,
            Some(11)
        );
        assert_eq!(
            book.snapshot(&Selector::Subtype("flawless".into()))
                .lowest_ingame,
            None
        );
    }

    #[test]
    fn a_variant_nobody_sells_has_no_price_rather_than_zero() {
        let book = book(&[row("buy", 5, "ingame", r#""rank":0,"#)]);
        assert_eq!(book.snapshot(&Selector::Rank(0)).lowest_ingame, None);
        assert_eq!(book.snapshot(&Selector::Rank(0)).sellers_ingame, 0);
    }
}

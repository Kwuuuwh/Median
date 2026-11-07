use std::collections::HashMap;

use crate::time::Instant;

/// One recorded trade, as input to `margins`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deal {
    pub at: Instant,
    /// Item and variant key, e.g. `primed_heated_charge:mod_rank=10`.
    pub pair: String,
    pub selling: bool,
    /// Price per copy, in platinum.
    pub platinum: u32,
    pub quantity: u32,
}

/// Copies of one item-variant pair still held, and their total cost in platinum.
#[derive(Default)]
struct Held {
    copies: u64,
    cost: u64,
}

/// Margin in platinum of each sale over the average cost of the copies bought before it, indexed
/// like `deals`; `None` for a purchase and for a sale with no copies bought before it.
/// Deals are settled in time order, whatever their order in the slice. A sale of more copies than
/// are held is costed on the held copies only.
pub fn margins(deals: &[Deal]) -> Vec<Option<i64>> {
    let mut order: Vec<usize> = (0..deals.len()).collect();
    order.sort_by_key(|&at| deals[at].at);

    let mut held: HashMap<&str, Held> = HashMap::new();
    let mut margins = vec![None; deals.len()];
    for at in order {
        let deal = &deals[at];
        let pair = held.entry(deal.pair.as_str()).or_default();
        let copies = u64::from(deal.quantity);
        let price = u64::from(deal.platinum);
        if !deal.selling {
            pair.copies += copies;
            pair.cost += price * copies;
            continue;
        }
        let costed = copies.min(pair.copies);
        if costed == 0 {
            continue;
        }
        let cost = pair.cost * costed / pair.copies;
        pair.copies -= costed;
        pair.cost -= cost;
        margins[at] = Some((price * costed) as i64 - cost as i64);
    }
    margins
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deal(at: Instant, selling: bool, platinum: u32, quantity: u32) -> Deal {
        Deal {
            at,
            pair: "primed_heated_charge:mod_rank=10".into(),
            selling,
            platinum,
            quantity,
        }
    }

    #[test]
    fn a_sale_makes_what_it_went_for_over_what_the_copy_cost() {
        let margins = margins(&[deal(1, false, 74, 1), deal(2, true, 110, 1)]);

        assert_eq!(margins, vec![None, Some(36)]);
    }

    #[test]
    fn copies_bought_at_different_prices_cost_their_average() {
        let margins = margins(&[
            deal(1, false, 40, 1),
            deal(2, false, 60, 1),
            deal(3, true, 70, 1),
            deal(4, true, 50, 1),
        ]);

        assert_eq!(margins[2..], [Some(20), Some(0)]);
    }

    #[test]
    fn a_sale_of_copies_never_bought_makes_nothing_to_count() {
        assert_eq!(margins(&[deal(1, true, 30, 1)]), vec![None]);
    }

    #[test]
    fn only_the_copies_bought_count_toward_what_a_sale_made() {
        let margins = margins(&[deal(1, false, 50, 1), deal(2, true, 80, 3)]);

        assert_eq!(margins[1], Some(80 - 50));
    }

    #[test]
    fn deals_are_settled_in_the_order_they_happened_whatever_order_they_come_in() {
        let margins = margins(&[deal(2, true, 90, 1), deal(1, false, 60, 1)]);

        assert_eq!(margins, vec![Some(30), None]);
    }
}

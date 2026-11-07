use std::collections::BTreeMap;

/// One input of a recipe, with its own recipe when it is built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assembly {
    pub item: String,
    /// Copies the parent recipe needs; 1 at the top of the tree.
    pub count: u32,
    /// `None` for an input that is found or bought rather than built.
    pub craft: Option<Craft>,
}

/// Foundry recipe of an assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Craft {
    pub blueprint: String,
    /// Whether building uses the blueprint up.
    pub consumed: bool,
    pub credits: u64,
    /// Build time in the foundry, in seconds.
    pub time: u64,
    /// Copies one build produces.
    pub output: u32,
    pub parts: Vec<Assembly>,
    /// How to buy the blueprint when it is not held; `None` when no vendor sells it.
    pub bought: Option<Purchase>,
}

/// Vendor price of one purchase of a blueprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Purchase {
    /// Item the price is paid in, and its amount; `None` when the price is credits only.
    pub pays: Option<(String, u32)>,
    pub credits: u64,
    /// Copies one purchase gives.
    pub count: u32,
}

/// Copies builds can draw on, in this order: held copies, copies the foundry is building, then
/// surplus copies from earlier builds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stock {
    held: BTreeMap<String, u32>,
    forging: BTreeMap<String, u32>,
    surplus: BTreeMap<String, u32>,
    /// Surplus copies from builds the holdings cannot make yet.
    promised: BTreeMap<String, u32>,
}

impl Stock {
    pub fn new(held: BTreeMap<String, u32>, forging: BTreeMap<String, u32>) -> Self {
        Self {
            held,
            forging,
            surplus: BTreeMap::new(),
            promised: BTreeMap::new(),
        }
    }

    /// Copies held.
    pub fn held(&self, item: &str) -> u32 {
        self.held.get(item).copied().unwrap_or(0)
    }

    /// Copies the foundry is building.
    pub fn forging(&self, item: &str) -> u32 {
        self.forging.get(item).copied().unwrap_or(0)
    }

    /// Copies a build can still draw on, from all pools.
    pub fn available(&self, item: &str) -> u32 {
        self.held(item)
            + self.forging(item)
            + self.surplus.get(item).copied().unwrap_or(0)
            + self.promised.get(item).copied().unwrap_or(0)
    }

    /// Takes up to `wanted` copies, held ones first; returns the copies taken and how many of
    /// them come from builds that cannot be made yet.
    fn take(&mut self, item: &str, wanted: u32) -> (u32, u32) {
        let mut taken = 0;
        for pool in [&mut self.held, &mut self.forging, &mut self.surplus] {
            taken += spend(pool, item, wanted - taken);
        }
        let promised = spend(&mut self.promised, item, wanted - taken);
        (taken + promised, promised)
    }

    /// Sets the held and in-foundry copies of `item` to its state after its walk.
    fn give_back(&mut self, item: &str, held: u32, forging: u32) {
        self.held.insert(item.to_string(), held);
        self.forging.insert(item.to_string(), forging);
    }

    /// Stores the surplus copies of a build; when the build cannot be made yet (`ready` is
    /// false), they go to the `promised` pool.
    fn keep(&mut self, item: &str, count: u32, ready: bool) {
        let pool = match ready {
            true => &mut self.surplus,
            false => &mut self.promised,
        };
        if count > 0 {
            *pool.entry(item.to_string()).or_default() += count;
        }
    }
}

impl From<BTreeMap<String, u32>> for Stock {
    fn from(held: BTreeMap<String, u32>) -> Self {
        Self::new(held, BTreeMap::new())
    }
}

/// How well the holdings cover one step of a build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    /// The holdings cover every copy the step asks for.
    Held,
    /// Not held, but its blueprints are, and every part is held or can be built.
    Buildable,
    Short,
}

/// One step of a build, checked against the holdings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Judged {
    pub item: String,
    /// Copies the step asks for.
    pub count: u32,
    /// Copies still available in the holdings when the walk reached the step.
    pub have: u32,
    /// Copies the foundry was building when the walk reached the step.
    pub forging: u32,
    pub readiness: Readiness,
    /// How the missing copies are built; `None` when nothing is missing.
    pub craft: Option<Forged>,
}

/// How the missing copies of a step are built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Forged {
    pub blueprint: String,
    /// Whether building uses the blueprint up.
    pub consumed: bool,
    /// Blueprints the build asks for.
    pub count: u32,
    /// Blueprints available when the walk reached them.
    pub have: u32,
    /// Credits of the build, including buying its blueprint, excluding the builds of its parts.
    pub credits: u64,
    /// Build time in the foundry, in seconds.
    pub time: u64,
    pub parts: Vec<Judged>,
}

impl Judged {
    /// Credits of the step, including every build below it.
    pub fn credits(&self) -> u64 {
        self.craft.as_ref().map_or(0, |craft| {
            craft.credits + craft.parts.iter().map(Judged::credits).sum::<u64>()
        })
    }

    /// Foundry seconds along the step's longest chain of builds; parallel builds overlap.
    pub fn time(&self) -> u64 {
        self.craft.as_ref().map_or(0, |craft| {
            craft.time + craft.parts.iter().map(Judged::time).max().unwrap_or(0)
        })
    }
}

impl Forged {
    /// Whether the blueprints are held and every part is held or can be built.
    fn is_ready(&self) -> bool {
        self.have >= self.count
            && self
                .parts
                .iter()
                .all(|part| part.readiness != Readiness::Short)
    }
}

/// One line of a bill: copies of one item the build needs, and how many were covered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Line {
    pub count: u32,
    /// Copies held when the walk first reached it.
    pub have: u32,
    /// Copies the foundry was building when the walk first reached it.
    pub forging: u32,
    /// Whether it is a blueprint the build does not consume, so it is needed once however many
    /// builds run.
    pub kept: bool,
    /// Whether the missing copies are built rather than obtained directly.
    pub built: bool,
}

impl Line {
    /// Copies neither the holdings nor the foundry cover.
    pub fn short(&self) -> u32 {
        self.count.saturating_sub(self.have + self.forging)
    }

    fn join(&mut self, other: Line) {
        self.kept = self.kept || other.kept;
        self.built = self.built || other.built;
        self.count = match self.kept {
            true => self.count.max(other.count),
            false => self.count + other.count,
        };
        self.have = self.have.max(other.have);
        self.forging = self.forging.max(other.forging);
    }
}

/// Everything needed to obtain an item, line by line, with total credits and foundry time.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bill {
    /// Inputs that are found, farmed, bought or already held, by item.
    pub lines: BTreeMap<String, Line>,
    pub credits: u64,
    /// Foundry seconds along the longest chain of builds; parallel builds overlap.
    pub time: u64,
}

impl Bill {
    /// Adds another item's bill to this one; the time is the longer of the two.
    pub fn add(&mut self, other: &Bill) {
        for (item, line) in &other.lines {
            self.note(item, *line);
        }
        self.credits += other.credits;
        self.time = self.time.max(other.time);
    }

    fn note(&mut self, item: &str, line: Line) {
        self.lines.entry(item.to_string()).or_default().join(line);
    }
}

/// Builds the foundry can start now (every part held), by item, with the copies each is short of;
/// a step reached by several walks counts the largest shortfall.
pub fn startable(walks: &[Judged]) -> BTreeMap<String, u32> {
    gathered(walks, true)
}

/// Builds the holdings allow, building their parts first where needed, by item, with the copies
/// each is short of; a step reached by several walks counts the largest shortfall.
pub fn buildable(walks: &[Judged]) -> BTreeMap<String, u32> {
    gathered(walks, false)
}

fn gathered(walks: &[Judged], now: bool) -> BTreeMap<String, u32> {
    let mut found: BTreeMap<String, u32> = BTreeMap::new();
    for walked in walks {
        let mut own = BTreeMap::new();
        gather(walked, now, &mut own);
        for (item, count) in own {
            let most = found.entry(item).or_default();
            *most = (*most).max(count);
        }
    }
    found
}

fn gather(judged: &Judged, now: bool, found: &mut BTreeMap<String, u32>) {
    let Some(craft) = &judged.craft else {
        return;
    };
    let parts_held = craft
        .parts
        .iter()
        .all(|part| part.readiness == Readiness::Held);
    if judged.readiness == Readiness::Buildable && (parts_held || !now) {
        *found.entry(judged.item.clone()).or_default() += judged.count.saturating_sub(judged.have);
    }
    for part in &craft.parts {
        gather(part, now, found);
    }
}

/// Checks one new build of the assembly against `stock`; copies of the assembly itself in stock
/// are not used, but its parts are taken from stock.
pub fn judge(assembly: &Assembly, stock: &mut Stock) -> Judged {
    step(assembly, assembly.count, stock, false)
}

/// Walks every planned item, items used inside another planned item first. The built copy then
/// goes into the build above it (its mastery is kept) instead of requiring a second copy.
pub fn walk_plan(tops: &[Assembly], stock: &mut Stock) -> Vec<Judged> {
    ordered(tops)
        .into_iter()
        .map(|top| walk_top(top, stock))
        .collect()
}

/// Walks every planned item separately against a copy of the whole stock, to see what the
/// foundry can build; items that share an input each count it as their own.
pub fn walk_forge(tops: &[Assembly], stock: &Stock) -> Vec<Judged> {
    tops.iter()
        .map(|top| walk_top(top, &mut stock.clone()))
        .collect()
}

/// Planned items ordered so that an item used inside another comes first.
fn ordered(tops: &[Assembly]) -> Vec<&Assembly> {
    let mut order: Vec<&Assembly> = tops.iter().collect();
    order.sort_by_key(|top| {
        tops.iter()
            .filter(|other| other.item != top.item && holds(top, &other.item))
            .count()
    });
    order
}

/// Walks one planned item, then returns its copy to the stock for the builds above it.
fn walk_top(top: &Assembly, stock: &mut Stock) -> Judged {
    let held = stock.held(&top.item);
    let forging = stock.forging(&top.item);
    let got = top.count.saturating_sub(stock.available(&top.item));
    let judged = walk(top, stock);
    stock.give_back(&top.item, held, forging + got);
    judged
}

/// Whether `item` is used anywhere below the top of `assembly`.
fn holds(assembly: &Assembly, item: &str) -> bool {
    assembly.craft.as_ref().is_some_and(|craft| {
        craft
            .parts
            .iter()
            .any(|part| part.item == item || holds(part, item))
    })
}

/// Walks one build of the assembly down the recipe tree, using up `stock`.
pub fn walk(assembly: &Assembly, stock: &mut Stock) -> Judged {
    step(assembly, assembly.count, stock, true)
}

/// Bill for obtaining the assembly after `stock` is used up on it.
pub fn bill(assembly: &Assembly, stock: &mut Stock) -> Bill {
    billed(&walk(assembly, stock))
}

/// Bill of a walked build, line by line.
pub fn billed(top: &Judged) -> Bill {
    let mut bill = Bill {
        time: top.time(),
        ..Bill::default()
    };
    if top.craft.is_none() && top.readiness != Readiness::Held {
        bill.note(&top.item, line(top));
    }
    if let Some(craft) = &top.craft {
        itemise(craft, &mut bill);
    }
    bill
}

/// Bill for copies of an input walked on its own; the input itself gets a line, as a part would.
pub fn supplied(top: &Judged) -> Bill {
    let mut bill = Bill {
        time: top.time(),
        ..Bill::default()
    };
    if is_noted(top) {
        bill.note(&top.item, line(top));
    }
    if let Some(craft) = &top.craft {
        itemise(craft, &mut bill);
    }
    bill
}

/// Walks `copies` of the assembly down the recipe tree, using up the stock as it goes.
fn step(assembly: &Assembly, copies: u32, stock: &mut Stock, spends_itself: bool) -> Judged {
    let have = stock.held(&assembly.item);
    let forging = stock.forging(&assembly.item);
    let (short, promised) = match spends_itself {
        true => {
            let (taken, promised) = stock.take(&assembly.item, copies);
            (copies - taken, promised)
        }
        false => (copies, 0),
    };
    let craft = match (&assembly.craft, short) {
        (Some(craft), 1..) => {
            let output = craft.output.max(1);
            let builds = short.div_ceil(output);
            let forged = forge(craft, builds, stock);
            stock.keep(&assembly.item, builds * output - short, forged.is_ready());
            Some(forged)
        }
        _ => None,
    };
    let readiness = match (&craft, short) {
        _ if promised > 0 => Readiness::Short,
        (_, 0) => Readiness::Held,
        (Some(forged), _) if forged.is_ready() => Readiness::Buildable,
        _ => Readiness::Short,
    };
    Judged {
        item: assembly.item.clone(),
        count: copies,
        have,
        forging,
        readiness,
        craft,
    }
}

/// Walks `builds` runs of the recipe: its blueprints, then its parts, then the cost of buying the
/// missing blueprints.
fn forge(craft: &Craft, builds: u32, stock: &mut Stock) -> Forged {
    let have = stock.available(&craft.blueprint);
    let count = match craft.consumed {
        true => {
            stock.take(&craft.blueprint, builds);
            builds
        }
        false => 1,
    };
    let mut credits = craft.credits * u64::from(builds);
    let mut parts: Vec<Judged> = craft
        .parts
        .iter()
        .map(|part| step(part, part.count * builds, stock, true))
        .collect();
    if let Some(bought) = &craft.bought {
        let lots = count.saturating_sub(have).div_ceil(bought.count.max(1));
        if lots > 0 {
            credits += bought.credits * u64::from(lots);
            if let Some((currency, cost)) = &bought.pays {
                let paid = Assembly {
                    item: currency.clone(),
                    count: cost * lots,
                    craft: None,
                };
                parts.push(step(&paid, paid.count, stock, true));
            }
        }
    }
    Forged {
        blueprint: craft.blueprint.clone(),
        consumed: craft.consumed,
        count,
        have,
        credits,
        time: craft.time,
        parts,
    }
}

/// Adds the blueprints and parts of a build to `bill` as lines.
fn itemise(craft: &Forged, bill: &mut Bill) {
    bill.note(
        &craft.blueprint,
        Line {
            count: craft.count,
            have: craft.have,
            forging: 0,
            kept: !craft.consumed,
            built: false,
        },
    );
    bill.credits += craft.credits;
    for part in &craft.parts {
        if is_noted(part) {
            bill.note(&part.item, line(part));
        }
        if let Some(built) = &part.craft {
            itemise(built, bill);
        }
    }
}

/// Whether a step gets its own line: found or bought, or partly covered. A step built entirely
/// from scratch is shown through its blueprint and parts.
fn is_noted(judged: &Judged) -> bool {
    judged.craft.is_none() || judged.have > 0 || judged.forging > 0
}

fn line(judged: &Judged) -> Line {
    Line {
        count: judged.count,
        have: judged.have,
        forging: judged.forging,
        kept: false,
        built: judged.craft.is_some(),
    }
}

/// Takes up to `wanted` copies out of one pool; returns how many it took.
fn spend(pool: &mut BTreeMap<String, u32>, item: &str, wanted: u32) -> u32 {
    let Some(have) = pool.get_mut(item) else {
        return 0;
    };
    let taken = (*have).min(wanted);
    *have -= taken;
    taken
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(item: &str, count: u32) -> Assembly {
        Assembly {
            item: item.to_string(),
            count,
            craft: None,
        }
    }

    fn built(item: &str, count: u32, blueprint: &str, time: u64, parts: Vec<Assembly>) -> Assembly {
        Assembly {
            item: item.to_string(),
            count,
            craft: Some(Craft {
                blueprint: blueprint.to_string(),
                consumed: true,
                credits: 15_000,
                time,
                output: 1,
                parts,
                bought: None,
            }),
        }
    }

    /// Volt Prime with its catalog recipe.
    fn volt_prime() -> Assembly {
        let part = |name: &str, time: u64| {
            built(
                &format!("{name}Component"),
                1,
                &format!("{name}Blueprint"),
                time,
                vec![found("OrokinCell", 2), found("Rubedo", 500)],
            )
        };
        built(
            "VoltPrime",
            1,
            "VoltPrimeBlueprint",
            259_200,
            vec![
                part("VoltPrimeChassis", 43_200),
                part("VoltPrimeHelmet", 43_200),
                part("VoltPrimeSystems", 86_400),
                found("OrokinCell", 1),
            ],
        )
    }

    fn holding(items: &[(&str, u32)]) -> BTreeMap<String, u32> {
        items
            .iter()
            .map(|(item, count)| (item.to_string(), *count))
            .collect()
    }

    fn stock(items: &[(&str, u32)]) -> Stock {
        Stock::from(holding(items))
    }

    /// Bill lines with copies still missing, and how many.
    fn short(bill: &Bill) -> BTreeMap<String, u32> {
        bill.lines
            .iter()
            .filter(|(_, line)| line.short() > 0)
            .map(|(item, line)| (item.clone(), line.short()))
            .collect()
    }

    #[test]
    fn a_part_already_built_stands_as_a_covered_line_and_takes_its_branch_off() {
        let mut held = stock(&[
            ("VoltPrimeBlueprint", 1),
            ("VoltPrimeChassisComponent", 1),
            ("VoltPrimeHelmetBlueprint", 1),
            ("OrokinCell", 3),
            ("Rubedo", 2_000),
        ]);

        let bill = bill(&volt_prime(), &mut held);

        assert_eq!(
            short(&bill),
            holding(&[("VoltPrimeSystemsBlueprint", 1), ("OrokinCell", 2)])
        );
        assert_eq!(
            bill.lines.get("VoltPrimeChassisComponent"),
            Some(&Line {
                count: 1,
                have: 1,
                forging: 0,
                kept: false,
                built: false
            })
        );
        assert!(!bill.lines.contains_key("VoltPrimeChassisBlueprint"));
        assert_eq!(bill.credits, 45_000);
        assert_eq!(bill.time, 259_200 + 86_400);
    }

    #[test]
    fn a_part_held_short_and_built_for_the_rest_is_marked_built() {
        let alloy = built("Alloy", 3, "AlloyBlueprint", 60, vec![found("Ore", 10)]);
        let mut held = stock(&[("Alloy", 1), ("AlloyBlueprint", 1)]);

        let bill = bill(
            &built("KDrive", 1, "KDriveBlueprint", 60, vec![alloy]),
            &mut held,
        );

        let line = bill
            .lines
            .get("Alloy")
            .expect("the alloy stays on the bill");
        assert!(line.built);
        assert!(!bill.lines["Ore"].built);
    }

    #[test]
    fn a_line_the_holdings_cover_stays_on_the_bill() {
        let mut held = stock(&[("OrokinCell", 50), ("Rubedo", 9_000)]);

        let bill = bill(&volt_prime(), &mut held);

        assert_eq!(
            bill.lines.get("OrokinCell"),
            Some(&Line {
                count: 7,
                have: 50,
                forging: 0,
                kept: false,
                built: false
            })
        );
        assert_eq!(bill.lines.get("Rubedo").map(Line::short), Some(0));
    }

    #[test]
    fn nothing_held_asks_for_every_blueprint_and_every_resource() {
        let bill = bill(&volt_prime(), &mut Stock::default());

        assert_eq!(
            short(&bill),
            holding(&[
                ("VoltPrimeBlueprint", 1),
                ("VoltPrimeChassisBlueprint", 1),
                ("VoltPrimeHelmetBlueprint", 1),
                ("VoltPrimeSystemsBlueprint", 1),
                ("OrokinCell", 7),
                ("Rubedo", 1_500),
            ])
        );
        assert_eq!(bill.credits, 60_000);
    }

    #[test]
    fn an_item_already_held_asks_for_nothing() {
        let mut held = stock(&[("VoltPrime", 1)]);

        assert_eq!(bill(&volt_prime(), &mut held), Bill::default());
    }

    #[test]
    fn an_item_nothing_builds_is_a_line_of_its_own() {
        let bill = bill(&found("Serration", 1), &mut Stock::default());

        assert_eq!(short(&bill), holding(&[("Serration", 1)]));
    }

    #[test]
    fn two_copies_of_one_part_need_two_of_everything_under_it() {
        let broken_war = built(
            "BrokenWar",
            1,
            "BrokenWarBlueprint",
            86_400,
            vec![built(
                "WarBlade",
                2,
                "WarBladeBlueprint",
                43_200,
                vec![found("Ferrite", 100)],
            )],
        );

        let bill = bill(&broken_war, &mut stock(&[("WarBladeBlueprint", 1)]));

        assert_eq!(short(&bill).get("WarBladeBlueprint"), Some(&1));
        assert_eq!(short(&bill).get("Ferrite"), Some(&200));
        assert_eq!(bill.credits, 45_000);
    }

    #[test]
    fn a_blueprint_the_foundry_keeps_is_wanted_once_whatever_the_count() {
        let mut kept = built(
            "Forma",
            3,
            "FormaBlueprint",
            86_400,
            vec![found("Ferrite", 10)],
        );
        if let Some(craft) = kept.craft.as_mut() {
            craft.consumed = false;
        }
        let mut held = Stock::default();

        let mut total = bill(&kept, &mut held);
        total.add(&bill(&kept, &mut held));

        assert_eq!(short(&total).get("FormaBlueprint"), Some(&1));
        assert_eq!(short(&total).get("Ferrite"), Some(&60));
    }

    #[test]
    fn items_billed_through_one_holding_share_it_out() {
        let mut held = stock(&[("OrokinCell", 7), ("Rubedo", 1_500)]);

        let first = bill(&volt_prime(), &mut held);
        let second = bill(&volt_prime(), &mut held);

        assert_eq!(first.lines.get("OrokinCell").map(Line::short), Some(0));
        assert_eq!(second.lines.get("OrokinCell").map(Line::short), Some(7));

        let mut total = first.clone();
        total.add(&second);
        assert_eq!(total.lines.get("OrokinCell").map(Line::short), Some(7));
        assert_eq!(short(&total).get("VoltPrimeBlueprint"), Some(&2));
        assert_eq!(total.credits, first.credits + second.credits);
    }

    #[test]
    fn a_recipe_handing_over_several_copies_runs_as_few_times_as_covers_the_need() {
        let alloy = |count: u32| {
            let mut alloy = built(
                "AdramalAlloy",
                count,
                "AdramalAlloyBlueprint",
                60,
                vec![found("Adramalium", 20)],
            );
            if let Some(craft) = alloy.craft.as_mut() {
                craft.output = 20;
            }
            alloy
        };
        let engine = |name: &str| built(name, 1, &format!("{name}Blueprint"), 60, vec![alloy(15)]);
        let mut held = Stock::default();

        let first = bill(&engine("BonewidowEngine"), &mut held);
        let second = bill(&engine("VoidrigEngine"), &mut held);

        assert_eq!(short(&first).get("AdramalAlloyBlueprint"), Some(&1));
        assert_eq!(short(&first).get("Adramalium"), Some(&20));
        assert_eq!(short(&second).get("AdramalAlloyBlueprint"), Some(&1));
        assert_eq!(short(&second).get("Adramalium"), Some(&20));

        let third = bill(&engine("CortegeEngine"), &mut held);
        assert_eq!(short(&third).get("Adramalium"), Some(&20));
        assert_eq!(held.available("AdramalAlloy"), 15);
    }

    #[test]
    fn what_the_foundry_builds_covers_a_line_and_is_told_apart_from_what_is_held() {
        let baza = built("Baza", 1, "BazaBlueprint", 86_400, vec![found("Forma", 1)]);
        let mut held = Stock::new(holding(&[("BazaBlueprint", 1)]), holding(&[("Forma", 1)]));

        let bill = bill(&baza, &mut held);

        assert_eq!(
            bill.lines.get("Forma"),
            Some(&Line {
                count: 1,
                have: 0,
                forging: 1,
                kept: false,
                built: false
            })
        );
        assert!(short(&bill).is_empty());
        assert_eq!(held.available("Forma"), 0);
    }

    #[test]
    fn a_build_whose_every_input_is_held_can_be_built_down_to_its_parts() {
        let mut held = stock(&[
            ("VoltPrimeBlueprint", 1),
            ("VoltPrimeChassisBlueprint", 1),
            ("VoltPrimeHelmetBlueprint", 1),
            ("VoltPrimeSystemsBlueprint", 1),
            ("OrokinCell", 7),
            ("Rubedo", 1_500),
        ]);

        let judged = judge(&volt_prime(), &mut held);

        assert_eq!(judged.readiness, Readiness::Buildable);
        let parts = &judged.craft.as_ref().expect("a build").parts;
        assert!(parts.iter().all(|part| part.readiness != Readiness::Short));
        assert_eq!(judged.credits(), 60_000);
    }

    #[test]
    fn inputs_shared_by_parts_run_out_at_the_part_that_finds_them_spent() {
        let mut held = stock(&[
            ("VoltPrimeBlueprint", 1),
            ("VoltPrimeChassisBlueprint", 1),
            ("VoltPrimeHelmetBlueprint", 1),
            ("VoltPrimeSystemsBlueprint", 1),
            ("OrokinCell", 7),
            ("Rubedo", 1_499),
        ]);

        let judged = judge(&volt_prime(), &mut held);

        let readiness: Vec<Readiness> = judged
            .craft
            .as_ref()
            .expect("a build")
            .parts
            .iter()
            .map(|part| part.readiness)
            .collect();
        assert_eq!(
            readiness,
            [
                Readiness::Buildable,
                Readiness::Buildable,
                Readiness::Short,
                Readiness::Held
            ]
        );
        assert_eq!(judged.readiness, Readiness::Short);
    }

    #[test]
    fn a_held_part_is_not_walked_below() {
        let mut held = stock(&[("VoltPrimeChassisComponent", 1)]);

        let judged = judge(&volt_prime(), &mut held);

        let chassis = &judged.craft.as_ref().expect("a build").parts[0];
        assert_eq!(chassis.readiness, Readiness::Held);
        assert_eq!(chassis.craft, None);
    }

    #[test]
    fn the_top_is_judged_as_a_new_build_whatever_is_held_of_it() {
        let mut held = stock(&[("VoltPrime", 2)]);

        let judged = judge(&volt_prime(), &mut held);

        assert_eq!(judged.have, 2);
        assert_eq!(judged.readiness, Readiness::Short);
        assert!(judged.craft.is_some());
    }

    #[test]
    fn forma_walked_on_its_own_stands_as_a_line_with_what_the_foundry_holds() {
        let forma = built(
            "Forma",
            3,
            "FormaBlueprint",
            82_800,
            vec![found("Neurodes", 1)],
        );
        let mut held = Stock::new(
            holding(&[("FormaBlueprint", 5), ("Neurodes", 1)]),
            holding(&[("Forma", 1)]),
        );

        let bill = supplied(&walk(&forma, &mut held));

        assert_eq!(bill.lines["Forma"].forging, 1);
        assert_eq!(bill.lines["Forma"].count, 3);
        assert_eq!(bill.lines["FormaBlueprint"].count, 2);
        assert_eq!(short(&bill), holding(&[("Forma", 2), ("Neurodes", 1)]));
        assert_eq!(bill.credits, 30_000);
    }

    #[test]
    fn forma_held_in_full_stands_as_a_covered_line() {
        let forma = built(
            "Forma",
            2,
            "FormaBlueprint",
            82_800,
            vec![found("Neurodes", 1)],
        );
        let mut held = stock(&[("Forma", 4)]);

        let bill = supplied(&walk(&forma, &mut held));

        assert_eq!(bill.lines.len(), 1);
        assert_eq!(bill.lines["Forma"].short(), 0);
        assert_eq!(bill.time, 0);
    }

    /// Syam, whose blueprint Teshin's Cave sells for 50 Pathos Clamps.
    fn syam() -> Assembly {
        let mut syam = built(
            "Syam",
            1,
            "SyamBlueprint",
            86_400,
            vec![found("RunicMarrow", 30)],
        );
        if let Some(craft) = &mut syam.craft {
            craft.bought = Some(Purchase {
                pays: Some(("PathosClamp".to_string(), 50)),
                credits: 0,
                count: 1,
            });
        }
        syam
    }

    #[test]
    fn a_blueprint_on_sale_that_is_not_held_asks_for_its_price() {
        let mut held = stock(&[("PathosClamp", 14), ("RunicMarrow", 79)]);

        let bill = bill(&syam(), &mut held);

        assert_eq!(
            short(&bill),
            holding(&[("PathosClamp", 36), ("SyamBlueprint", 1)])
        );
    }

    #[test]
    fn a_blueprint_held_is_not_bought_again() {
        let mut held = stock(&[("SyamBlueprint", 1), ("RunicMarrow", 79)]);

        let bill = bill(&syam(), &mut held);

        assert!(short(&bill).is_empty());
        assert!(!bill.lines.contains_key("PathosClamp"));
    }

    #[test]
    fn a_blueprint_sold_for_credits_adds_them_to_the_build() {
        let mut braton = built(
            "Braton",
            1,
            "BratonBlueprint",
            60,
            vec![found("Ferrite", 50)],
        );
        if let Some(craft) = &mut braton.craft {
            craft.bought = Some(Purchase {
                pays: None,
                credits: 1_500,
                count: 1,
            });
        }
        let mut held = stock(&[("Ferrite", 50)]);

        let bill = bill(&braton, &mut held);

        assert_eq!(bill.credits, 16_500);
        assert_eq!(short(&bill), holding(&[("BratonBlueprint", 1)]));
    }

    #[test]
    fn the_foundry_can_start_a_step_whose_parts_are_all_held() {
        let mut held = stock(&[
            ("VoltPrimeBlueprint", 1),
            ("VoltPrimeChassisBlueprint", 1),
            ("OrokinCell", 3),
            ("Rubedo", 500),
            ("VoltPrimeHelmetComponent", 1),
            ("VoltPrimeSystemsComponent", 1),
        ]);
        let walked = walk(&volt_prime(), &mut held);

        let ready = startable(&[walked]);

        assert_eq!(ready, holding(&[("VoltPrimeChassisComponent", 1)]));
    }

    #[test]
    fn what_the_holdings_build_in_turn_counts_the_step_above_a_startable_one() {
        let mut held = stock(&[
            ("VoltPrimeBlueprint", 1),
            ("VoltPrimeChassisBlueprint", 1),
            ("VoltPrimeHelmetBlueprint", 1),
            ("VoltPrimeSystemsBlueprint", 1),
            ("OrokinCell", 7),
            ("Rubedo", 1_500),
        ]);
        let walked = walk(&volt_prime(), &mut held);

        let built = buildable(&[walked]);

        assert_eq!(
            built,
            holding(&[
                ("VoltPrime", 1),
                ("VoltPrimeChassisComponent", 1),
                ("VoltPrimeHelmetComponent", 1),
                ("VoltPrimeSystemsComponent", 1),
            ])
        );
    }

    #[test]
    fn a_step_whose_shared_input_runs_out_is_not_buildable() {
        let mut held = stock(&[
            ("VoltPrimeBlueprint", 1),
            ("VoltPrimeChassisBlueprint", 1),
            ("VoltPrimeHelmetBlueprint", 1),
            ("VoltPrimeSystemsBlueprint", 1),
            ("OrokinCell", 4),
            ("Rubedo", 1_500),
        ]);
        let walked = walk(&volt_prime(), &mut held);

        let built = buildable(&[walked]);

        assert_eq!(
            built,
            holding(&[
                ("VoltPrimeChassisComponent", 1),
                ("VoltPrimeHelmetComponent", 1),
            ])
        );
    }

    #[test]
    fn a_planned_item_that_goes_into_another_is_not_asked_for_twice() {
        let kama = built(
            "SingleKama",
            2,
            "SingleKamaBlueprint",
            43_200,
            vec![found("Ferrite", 1_200)],
        );
        let dual = |count| {
            built(
                "DualKamas",
                count,
                "DualKamasBlueprint",
                43_200,
                vec![kama.clone(), found("OrokinCell", 1)],
            )
        };
        let raza = built(
            "DualRaza",
            1,
            "DualRazaBlueprint",
            43_200,
            vec![dual(1), found("OrokinCell", 1)],
        );
        let mut held = Stock::new(
            holding(&[
                ("DualKamasBlueprint", 1),
                ("SingleKamaBlueprint", 2),
                ("DualRazaBlueprint", 1),
                ("OrokinCell", 2),
                ("Ferrite", 5_000),
            ]),
            holding(&[("DualKamas", 1)]),
        );

        let walks = walk_plan(&[raza, dual(1)], &mut held);

        assert_eq!(buildable(&walks), holding(&[("DualRaza", 1)]));
    }

    #[test]
    fn what_a_batch_the_holdings_cannot_make_leaves_over_builds_nothing() {
        let cut = |count| Assembly {
            item: "CutGem".to_string(),
            count,
            craft: Some(Craft {
                blueprint: "CutGemBlueprint".to_string(),
                consumed: false,
                credits: 1_000,
                time: 60,
                output: 10,
                parts: vec![found("Gem", 10)],
                bought: None,
            }),
        };
        let barrel = |name: &str| built(name, 1, &format!("{name}Blueprint"), 43_200, vec![cut(5)]);
        let mut held = stock(&[
            ("CutGemBlueprint", 1),
            ("FirstBarrelBlueprint", 1),
            ("SecondBarrelBlueprint", 1),
            ("Gem", 7),
        ]);

        let walks = walk_plan(&[barrel("FirstBarrel"), barrel("SecondBarrel")], &mut held);

        assert!(buildable(&walks).is_empty());
    }

    #[test]
    fn an_item_that_cannot_be_made_leaves_what_it_shares_to_one_that_can() {
        let held = stock(&[
            ("ShortBlueprint", 1),
            ("WholeBlueprint", 1),
            ("Rotoblade", 10),
        ]);
        let short = built(
            "Short",
            1,
            "ShortBlueprint",
            60,
            vec![found("Rotoblade", 10), found("Missing", 1)],
        );
        let whole = built(
            "Whole",
            1,
            "WholeBlueprint",
            60,
            vec![found("Rotoblade", 10)],
        );

        let walks = walk_forge(&[short, whole], &held);

        assert_eq!(buildable(&walks), holding(&[("Whole", 1)]));
    }

    #[test]
    fn items_the_holdings_make_one_or_the_other_of_are_both_buildable() {
        let held = stock(&[("FirstBlueprint", 1), ("SecondBlueprint", 1), ("Sagan", 9)]);
        let first = built("First", 1, "FirstBlueprint", 60, vec![found("Sagan", 6)]);
        let second = built("Second", 1, "SecondBlueprint", 60, vec![found("Sagan", 5)]);

        let walks = walk_forge(&[first, second], &held);

        assert_eq!(buildable(&walks), holding(&[("First", 1), ("Second", 1)]));
    }
}

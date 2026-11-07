use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::{Arc, PoisonError, RwLock};
use std::time::Duration;

use median_core::GoodRolls;
use tokio::time::sleep;
use tracing::{info, warn};

/// CSV export URL of the community sheet of in-demand riven stats, one tab per weapon class.
const SHEET: &str = "https://docs.google.com/spreadsheets/d/1zbaeJBuBn44cbVKzJins_E3hTDpnmvOk8heYN-G8yy8/gviz/tq?tqx=out:csv&sheet=";

/// Tabs of the sheet; kitguns are listed under secondary.
const TABS: [&str; 5] = ["primary", "secondary", "melee", "archgun", "robotic"];

/// Refresh interval of the sheet.
const EVERY: Duration = Duration::from_secs(24 * 60 * 60);

/// Delay before the next attempt after a failed download.
const RETRY: Duration = Duration::from_secs(60 * 60);

/// Good rolls of each weapon's rivens, keyed by riven family in lower case.
#[derive(Default)]
pub struct Sheet(RwLock<HashMap<String, GoodRolls>>);

impl Sheet {
    /// Good rolls for rivens of `family`, matched case-insensitively; `None` when the sheet does
    /// not list it.
    pub fn of(&self, family: &str) -> Option<GoodRolls> {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&family.to_lowercase())
            .cloned()
    }

    fn replace(&self, tabs: &[String]) -> Result<(), String> {
        let mut by_family = HashMap::new();
        let mut unknown = BTreeSet::new();
        for tab in tabs {
            for (weapon, positive, negative) in rows(tab)? {
                let (rolls, missed) = GoodRolls::parse(&positive, &negative);
                unknown.extend(missed);
                by_family.insert(weapon.to_lowercase(), rolls);
            }
        }
        info!(
            weapons = by_family.len(),
            ?unknown,
            "riven good rolls sheet read"
        );
        *self.0.write().unwrap_or_else(PoisonError::into_inner) = by_family;
        Ok(())
    }
}

/// Loads the sheet saved in `cache` at once, then downloads the live sheet now and every
/// `EVERY` (after `RETRY` on failure), saving each download to `cache`.
pub async fn tend(sheet: Arc<Sheet>, cache: PathBuf) -> Result<(), String> {
    if let Ok(kept) = read_cache(&cache)
        && let Err(why) = sheet.replace(&kept)
    {
        warn!(error = %why, "saved riven good rolls sheet unreadable");
    }
    let client = crate::http::http_client()?;
    loop {
        let fetched = fetch(&client).await;
        match fetched.and_then(|tabs| sheet.replace(&tabs).map(|()| tabs)) {
            Ok(tabs) => {
                if let Err(why) = write_cache(&cache, &tabs) {
                    warn!(error = %why, "riven good rolls sheet not saved");
                }
                sleep(EVERY).await;
            }
            Err(why) => {
                warn!(error = %why, "riven good rolls sheet download failed");
                sleep(RETRY).await;
            }
        }
    }
}

async fn fetch(client: &reqwest::Client) -> Result<Vec<String>, String> {
    let mut tabs = Vec::with_capacity(TABS.len());
    for tab in TABS {
        let response = client
            .get(format!("{SHEET}{tab}"))
            .send()
            .await
            .map_err(|e| e.to_string())?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("tab {tab} returned {status}"));
        }
        tabs.push(response.text().await.map_err(|e| e.to_string())?);
    }
    Ok(tabs)
}

fn read_cache(cache: &std::path::Path) -> Result<Vec<String>, String> {
    TABS.iter()
        .map(|tab| std::fs::read_to_string(cache.join(format!("{tab}.csv"))))
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())
}

fn write_cache(cache: &std::path::Path, tabs: &[String]) -> Result<(), String> {
    std::fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    for (tab, text) in TABS.iter().zip(tabs) {
        std::fs::write(cache.join(format!("{tab}.csv")), text).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Weapon, positive-stats and negative-stats cells of each row of one tab, with the columns
/// found by the header row; `Err` when the tab has no stat columns.
fn rows(tab: &str) -> Result<Vec<(String, String, String)>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(tab.as_bytes());
    let mut columns = None;
    let mut found = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| e.to_string())?;
        let cell = |index: usize| record.get(index).unwrap_or("").trim().to_owned();
        let Some((positive, negative)) = columns else {
            let position = |title: &str| record.iter().position(|each| each.trim() == title);
            if let (Some(positive), Some(negative)) =
                (position("POSITIVE STATS:"), position("NEGATIVE STATS:"))
            {
                columns = Some((positive, negative));
            }
            continue;
        };
        let weapon = cell(0);
        if !weapon.is_empty() {
            found.push((weapon, cell(positive), cell(negative)));
        }
    }
    match columns {
        Some(_) => Ok(found),
        None => Err("a tab carries no stat columns".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MELEE: &str = r#""WEAPON","POSITIVE STATS:","","","","","NEGATIVE STATS:","","","Notes:"
"ack & brunt","CD AS RANGE or CD/RANGE AS ELEC","","","","","EFF/SLIDE/FIN","","",""
"arca titron","CD AS RANGE","","","","","SLIDE/FIN/IMP","","","(NOTE: ELEC, is for ""influence"")"
"#;

    #[test]
    fn a_tab_reads_into_weapon_rows_by_its_headers() {
        let found = rows(MELEE).expect("rows");
        assert_eq!(found.len(), 2);
        assert_eq!(
            found[0],
            (
                "ack & brunt".to_owned(),
                "CD AS RANGE or CD/RANGE AS ELEC".to_owned(),
                "EFF/SLIDE/FIN".to_owned()
            )
        );
    }

    #[test]
    fn a_family_is_found_whatever_its_case() {
        let sheet = Sheet::default();
        sheet.replace(&[MELEE.to_owned()]).expect("read");
        assert!(sheet.of("Ack & Brunt").is_some());
        assert!(sheet.of("Soma").is_none());
    }

    #[test]
    fn a_tab_without_stat_columns_is_refused() {
        assert!(rows("\"a\",\"b\"\n").is_err());
    }
}

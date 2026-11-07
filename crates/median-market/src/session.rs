use std::sync::Arc;

use median_core::Permit;
use serde::Deserialize;

use crate::error::MarketError;
use crate::gate::Pass;
use crate::http::Client;
use crate::model::Side;

/// v1 sign-in endpoint; it takes an email and a password.
const SIGN_IN: &str = "/auth/signin";

/// Header that carries the token in the sign-in response and in later requests.
const AUTHORIZATION: &str = "authorization";

/// Value the sign-in request sends on that header. Without it the market's CSRF check rejects
/// the call with `CSRF: Token is not present in the request header`.
const SCHEME: &str = "JWT";

/// Scheme v2 requires for the same token. v1 issues it as `JWT <token>` and accepts it that
/// way; every v2 route returns 401 unless the scheme is `Bearer`.
const V2_SCHEME: &str = "Bearer";

/// The token without the scheme word in front of it.
pub(crate) fn bare(authorization: &str) -> &str {
    authorization
        .split_once(' ')
        .map_or(authorization, |(_, token)| token)
}

/// v2 wraps every response in `{ apiVersion, data, error }`.
#[derive(Debug, Deserialize)]
struct Envelope<T> {
    data: T,
}

/// The part of the sign-in response that is kept; when fields are absent the name stays
/// unknown.
#[derive(Debug, Deserialize)]
struct SignedIn {
    payload: Option<SignedInPayload>,
}

#[derive(Debug, Deserialize)]
struct SignedInPayload {
    user: Option<SignedInUser>,
}

#[derive(Debug, Deserialize)]
struct SignedInUser {
    ingame_name: Option<String>,
}

/// One of the account's own orders. The market omits the owner, since it is the account.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mine {
    pub id: String,
    #[serde(rename = "type")]
    pub side: Side,
    pub platinum: u32,
    pub quantity: u32,
    #[serde(default)]
    pub per_trade: Option<u32>,
    pub visible: bool,
    #[serde(default)]
    pub rank: Option<u8>,
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub amber_stars: Option<u8>,
    #[serde(default)]
    pub cyan_stars: Option<u8>,
    #[serde(default)]
    pub item_id: Option<String>,
    /// RFC 3339 timestamp, unparsed.
    pub created_at: String,
    pub updated_at: String,
}

/// One order to place. The permit passed with it must be for the same item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placing {
    /// Game key of the item.
    pub item: String,
    /// warframe.market id of the same item.
    pub item_id: String,
    pub side: Side,
    pub platinum: u32,
    pub quantity: u32,
    /// Copies per trade. `None` for items the market trades one at a time: the market rejects
    /// the field itself on those, not only a lot size above one.
    pub per_trade: Option<u32>,
    pub rank: Option<u8>,
    pub subtype: Option<String>,
    pub visible: bool,
}

impl Placing {
    /// The order as the v2 request body; a variant field the item lacks is omitted.
    fn body(&self) -> serde_json::Value {
        let mut body = serde_json::Map::new();
        body.insert("itemId".into(), self.item_id.clone().into());
        body.insert("type".into(), self.side.as_str().into());
        body.insert("platinum".into(), self.platinum.into());
        body.insert("quantity".into(), self.quantity.into());
        if let Some(per_trade) = self.per_trade {
            body.insert("perTrade".into(), per_trade.max(1).into());
        }
        body.insert("visible".into(), self.visible.into());
        if let Some(rank) = self.rank {
            body.insert("rank".into(), rank.into());
        }
        if let Some(subtype) = &self.subtype {
            body.insert("subtype".into(), subtype.clone().into());
        }
        serde_json::Value::Object(body)
    }
}

/// A signed-in account. Every call that needs the token is made through a session.
pub struct Session {
    client: Arc<Client>,
    /// The `Authorization` value exactly as the market issued it, scheme included.
    authorization: String,
    /// The account name from the sign-in response; `None` when the response had none.
    name: Option<String>,
}

impl Session {
    /// Signs in by email and password. The token arrives in a response header, not in the body.
    pub async fn open(
        client: Arc<Client>,
        pass: Pass,
        email: &str,
        password: &str,
    ) -> Result<Self, MarketError> {
        let body = serde_json::json!({
            "email": email,
            "password": password,
            "auth_type": "header",
        });
        let url = client.v1(SIGN_IN);
        let (authorization, bytes) = client
            .header(pass, url, Some(SCHEME), body, AUTHORIZATION, "sign in")
            .await?;

        let name = serde_json::from_slice::<SignedIn>(&bytes)
            .ok()
            .and_then(|answer| answer.payload)
            .and_then(|payload| payload.user)
            .and_then(|user| user.ingame_name);

        Ok(Self {
            client,
            authorization,
            name,
        })
    }

    /// Restores a session from a stored token; makes no request.
    pub fn resume(client: Arc<Client>, authorization: String, name: Option<String>) -> Self {
        Self {
            client,
            authorization,
            name,
        }
    }

    /// The `Authorization` header value, for storing the token.
    pub fn authorization(&self) -> &str {
        &self.authorization
    }

    /// The account's warframe.market username.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// The client every call of this session is sent through.
    pub(crate) fn client(&self) -> &Client {
        &self.client
    }

    /// The `Authorization` value for v2: the same token with the `Bearer` scheme.
    fn bearer(&self) -> String {
        format!("{V2_SCHEME} {}", bare(&self.authorization))
    }

    /// Places one order; `Err(NotPermitted)` when the permit is for another item. The permit
    /// allows writes for this item only.
    pub async fn place(
        &self,
        pass: Pass,
        permit: &Permit,
        order: &Placing,
    ) -> Result<Mine, MarketError> {
        if permit.item() != order.item {
            return Err(MarketError::NotPermitted(order.item.clone()));
        }
        self.post(pass, order).await
    }

    /// Places one order the user asked for, without a permit.
    pub async fn place_by_hand(&self, pass: Pass, order: &Placing) -> Result<Mine, MarketError> {
        self.post(pass, order).await
    }

    async fn post(&self, pass: Pass, order: &Placing) -> Result<Mine, MarketError> {
        let url = self.client.v2("/order");
        let answer: Envelope<Mine> = self
            .client
            .json(
                pass,
                reqwest::Method::POST,
                url,
                Some(&self.bearer()),
                Some(order.body()),
                "place order",
            )
            .await?;
        Ok(answer.data)
    }

    /// Closes one order by the number of copies traded.
    pub async fn close(&self, pass: Pass, id: &str, quantity: u32) -> Result<(), MarketError> {
        let url = self.client.v2(&format!("/order/{id}/close"));
        let _: Envelope<serde_json::Value> = self
            .client
            .json(
                pass,
                reqwest::Method::POST,
                url,
                Some(&self.bearer()),
                Some(serde_json::json!({ "quantity": quantity })),
                "close order",
            )
            .await?;
        Ok(())
    }

    /// Changes one order; an omitted field stays unchanged.
    pub async fn amend(
        &self,
        pass: Pass,
        id: &str,
        change: serde_json::Value,
    ) -> Result<Mine, MarketError> {
        let url = self.client.v2(&format!("/order/{id}"));
        let answer: Envelope<Mine> = self
            .client
            .json(
                pass,
                reqwest::Method::PATCH,
                url,
                Some(&self.bearer()),
                Some(change),
                "amend order",
            )
            .await?;
        Ok(answer.data)
    }

    /// Deletes one order.
    pub async fn withdraw(&self, pass: Pass, id: &str) -> Result<(), MarketError> {
        let url = self.client.v2(&format!("/order/{id}"));
        let _: Envelope<serde_json::Value> = self
            .client
            .json(
                pass,
                reqwest::Method::DELETE,
                url,
                Some(&self.bearer()),
                None,
                "withdraw order",
            )
            .await?;
        Ok(())
    }

    /// Every open order of this account.
    pub async fn my_orders(&self, pass: Pass) -> Result<Vec<Mine>, MarketError> {
        let url = self.client.v2("/orders/my");
        let answer: Envelope<Vec<Mine>> = self
            .client
            .json(
                pass,
                reqwest::Method::GET,
                url,
                Some(&self.bearer()),
                None,
                "my orders",
            )
            .await?;
        Ok(answer.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use median_core::{Chance, Means, Policy};

    fn placing(item: &str) -> Placing {
        Placing {
            item: item.into(),
            item_id: "54a73e65e779893a797fff9b".into(),
            side: Side::Buy,
            platinum: 20,
            quantity: 1,
            per_trade: Some(1),
            rank: Some(0),
            subtype: None,
            visible: true,
        }
    }

    fn permit(item: &str) -> Permit {
        let chance = Chance {
            item: item.into(),
            entry: 20,
            worth: Some(60),
            volume: Some(40),
            per_trade: 1,
            copies: 1,
            listed: false,
            falling: false,
        };
        let policy = Policy {
            most_tied: 1_000,
            ..Policy::default()
        };
        let means = Means {
            held: None,
            free: Some(1_000),
            tied: 0,
            exchanges: Some(10),
            orders: 0,
            hand: 0,
        };
        Permit::issue(&policy, &chance, &means).expect("the policy lets this one through")
    }

    #[test]
    fn v2_takes_the_token_under_another_scheme_than_v1_issued_it() {
        let (client, _doorman) = client();
        let session = Session::resume(client, "JWT eyJhbGciOi".into(), None);

        assert_eq!(session.bearer(), "Bearer eyJhbGciOi");
        // The value passed to the token store stays exactly as the market issued it.
        assert_eq!(session.authorization(), "JWT eyJhbGciOi");
    }

    #[test]
    fn a_token_that_carries_no_scheme_is_taken_whole() {
        assert_eq!(bare("eyJhbGciOi"), "eyJhbGciOi");
        assert_eq!(bare("JWT eyJhbGciOi"), "eyJhbGciOi");
    }

    #[test]
    fn the_body_carries_the_dimensions_the_variant_has_and_no_others() {
        let body = placing("/Lotus/Mods/Vitality").body();

        assert_eq!(body["itemId"], "54a73e65e779893a797fff9b");
        assert_eq!(body["type"], "buy");
        assert_eq!(body["rank"], 0);
        assert!(body.get("subtype").is_none());
    }

    #[test]
    fn one_copy_at_a_time_is_still_one_copy_an_exchange() {
        let order = Placing {
            per_trade: Some(0),
            ..placing("/Lotus/Mods/Vitality")
        };

        assert_eq!(order.body()["perTrade"], 1);
    }

    #[test]
    fn an_item_that_names_no_lot_sends_no_lot() {
        let order = Placing {
            per_trade: None,
            ..placing("/Lotus/Mods/Vitality")
        };

        assert!(order.body().get("perTrade").is_none());
    }

    fn client() -> (Arc<crate::http::Client>, crate::gate::Doorman) {
        let (gate, doorman) = crate::gate::Gate::open(
            crate::gate::Budget {
                per_second: 3.0,
                burst: 3.0,
                failures_before_open: 3,
                cooldown: std::time::Duration::from_secs(60),
            },
            std::time::Instant::now(),
        );
        let client =
            crate::http::Client::new(gate, crate::http::Host::default(), "median-test/0.1")
                .expect("client builds");
        (Arc::new(client), doorman)
    }

    #[tokio::test]
    async fn a_permit_for_another_item_does_not_write_this_one() {
        let (client, doorman) = client();
        let pass = doorman.admit().await.expect("a token");
        let session = Session::resume(client, "JWT none".into(), None);

        let refused = session
            .place(
                pass,
                &permit("/Lotus/Mods/Vitality"),
                &placing("/Lotus/Mods/Redirection"),
            )
            .await;

        assert_eq!(
            refused,
            Err(MarketError::NotPermitted("/Lotus/Mods/Redirection".into()))
        );
    }
}

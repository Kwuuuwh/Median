/// Whether the account is signed in to warframe.market.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Standing {
    /// No token is held.
    #[default]
    Out,
    /// A token is held.
    In { name: Option<String> },
    /// warframe.market rejected the token, and it was deleted from the token store.
    Expired,
}

/// Account status shown to other traders on warframe.market.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Status {
    Online,
    Ingame,
    #[default]
    Invisible,
}

impl Status {
    /// Every status, in the order the interface offers them.
    pub const ALL: [Status; 3] = [Status::Online, Status::Ingame, Status::Invisible];

    /// Code the status is sent to warframe.market and stored under, e.g. `"ingame"`.
    pub fn as_code(self) -> &'static str {
        match self {
            Status::Online => "online",
            Status::Ingame => "ingame",
            Status::Invisible => "invisible",
        }
    }

    /// Status with this code; `None` when no status has it.
    pub fn parse(code: &str) -> Option<Status> {
        Status::ALL
            .into_iter()
            .find(|status| status.as_code() == code)
    }
}

/// warframe.market session: sign-in state and the status shown to other traders.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Membership {
    pub standing: Standing,
    pub status: Status,
}

/// Change in the warframe.market session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MembershipEvent {
    SignedIn {
        name: Option<String>,
    },
    SignedOut,
    /// warframe.market rejected the token.
    Refused,
    StatusSet(Status),
}

/// Membership after `event`; signing out or a rejected token also sets the status to `Invisible`.
pub fn update(membership: &Membership, event: MembershipEvent) -> Membership {
    let mut next = membership.clone();
    match event {
        MembershipEvent::SignedIn { name } => next.standing = Standing::In { name },
        MembershipEvent::SignedOut => {
            next.standing = Standing::Out;
            next.status = Status::Invisible;
        }
        MembershipEvent::Refused => {
            next.standing = Standing::Expired;
            next.status = Status::Invisible;
        }
        MembershipEvent::StatusSet(status) => next.status = status,
    }
    next
}

/// Status to set for the game state: `Ingame` while `running`, `Invisible` otherwise. `None` when
/// `follow` is off, the account is not signed in, or the status is already set.
pub fn follows(membership: &Membership, follow: bool, running: bool) -> Option<Status> {
    if !follow || !matches!(membership.standing, Standing::In { .. }) {
        return None;
    }
    let wanted = match running {
        true => Status::Ingame,
        false => Status::Invisible,
    };
    (wanted != membership.status).then_some(wanted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_status_answers_to_its_own_code() {
        for status in Status::ALL {
            assert_eq!(Status::parse(status.as_code()), Some(status));
        }
    }

    #[test]
    fn signing_out_hides_the_account() {
        let signed = update(
            &Membership::default(),
            MembershipEvent::SignedIn { name: None },
        );
        let shown = update(&signed, MembershipEvent::StatusSet(Status::Ingame));

        let out = update(&shown, MembershipEvent::SignedOut);

        assert_eq!(out.standing, Standing::Out);
        assert_eq!(out.status, Status::Invisible);
    }

    #[test]
    fn a_refused_token_is_not_the_same_as_signing_out() {
        let signed = update(
            &Membership::default(),
            MembershipEvent::SignedIn { name: None },
        );

        let refused = update(&signed, MembershipEvent::Refused);

        assert_eq!(refused.standing, Standing::Expired);
    }

    #[test]
    fn the_status_follows_the_game_only_while_asked_to() {
        let signed = update(
            &Membership::default(),
            MembershipEvent::SignedIn { name: None },
        );
        assert_eq!(follows(&signed, false, true), None);

        assert_eq!(follows(&signed, true, true), Some(Status::Ingame));
        assert_eq!(follows(&signed, true, false), None);
    }

    #[test]
    fn the_status_does_not_follow_the_game_without_a_session() {
        assert_eq!(follows(&Membership::default(), true, true), None);
    }
}

use crate::{
    completion::Composite,
    history::History,
};

/// Contains all the state for a shell session.
pub struct Session {
    completion: Composite,
    history: History,
}

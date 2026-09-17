pub mod gcup;
pub mod icons;
pub mod infinite;
pub mod update;
pub mod validate;

use anyhow::Result;
use crate::args::Action;
use crate::i18n::Messages;
use std::path::Path;

pub fn dispatch(action: Action, game: &Path, msgs: &Messages, yes: bool) -> Result<()> {
    match action {
        Action::Update   => update::run(game, msgs, yes),
        Action::Icons    => icons::run(game, msgs),
        Action::Infinite => infinite::run(game, msgs),
        Action::Validate => validate::run(game, msgs),
    }
}
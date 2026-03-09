pub mod duel;
pub mod er;
pub mod leaderboard;
pub mod lobby;
pub mod player_state;
pub mod profile;
pub mod ranked;
pub mod reward;
pub mod room;
pub mod setup;

use anchor_lang::prelude::*;

use crate::error::GuessrError;
use crate::state::PlayerStatus;

pub fn ensure_wallet_matches_status(
    player_status: &PlayerStatus,
    wallet_address: Pubkey,
) -> Result<()> {
    require!(
        player_status.player == wallet_address,
        GuessrError::Unauthorized
    );
    Ok(())
}

pub fn ensure_player_authority(player_status: &PlayerStatus, authority: Pubkey) -> Result<()> {
    let wallet_authorized = player_status.player == authority;
    let session_authorized = player_status.session_address != Pubkey::default()
        && player_status.session_address == authority;

    require!(
        wallet_authorized || session_authorized,
        GuessrError::Unauthorized
    );
    Ok(())
}

pub use duel::*;
pub use er::*;
pub use leaderboard::*;
pub use lobby::*;
pub use player_state::*;
pub use profile::*;
pub use ranked::*;
pub use reward::*;
pub use room::*;
pub use setup::*;

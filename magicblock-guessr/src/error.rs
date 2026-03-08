use anchor_lang::prelude::*;

#[error_code]
pub enum GuessrError {
    #[msg("Only the player account owner can call this instruction")]
    Unauthorized,
    #[msg("Math overflow")]
    Overflow,
    #[msg("Math underflow")]
    Underflow,
    #[msg("Player is offline")]
    PlayerOffline,
    #[msg("Heartbeat TTL must be above zero")]
    InvalidHeartbeatTtl,
    #[msg("Heartbeat expired; player must rejoin")]
    HeartbeatExpired,
    #[msg("Player room does not match target room")]
    RoomMismatch,
    #[msg("Invalid match mode")]
    InvalidMatchMode,
    #[msg("Ranked room already settled")]
    RoomAlreadySettled,
    #[msg("Reward multiplier must be above zero")]
    InvalidMultiplier,
    #[msg("Penalty divisor must be above zero")]
    InvalidPenaltyDivisor,
    #[msg("Treasury token account mint mismatch")]
    InvalidTreasuryMint,
    #[msg("Player token account owner mismatch")]
    InvalidTokenOwner,
    #[msg("Player token account mint mismatch")]
    InvalidTokenMint,
    #[msg("Ranked room must be settled before close")]
    RoomNotSettled,
    #[msg("Invalid ranked action kind")]
    InvalidActionKind,
    #[msg("Accuracy basis points must be between 0 and 10000")]
    InvalidAccuracy,
    #[msg("Room already has two players")]
    RoomFull,
    #[msg("Player is not part of the target room")]
    PlayerNotInRoom,
    #[msg("Duel room is already settled")]
    DuelAlreadySettled,
    #[msg("Winner is not a participant in this room")]
    InvalidWinner,
    #[msg("Reward mint account does not match provided reward mint")]
    InvalidRewardMint,
    #[msg("Duel room requires both host and challenger before settlement")]
    DuelRoomIncomplete,
    #[msg("Invalid delegation target")]
    InvalidDelegationTarget,
    #[msg("Invalid reward amount")]
    InvalidAmount,
    #[msg("Reward has already been claimed")]
    AlreadyClaimed,
}

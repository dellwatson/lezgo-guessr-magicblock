use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, MintTo, Token, TokenAccount};
use ephemeral_rollups_sdk::anchor::action;

use crate::error::GuessrError;
use crate::state::{DuelRoom, RankedConfig, RankedRoom};

pub fn mint_ranked_reward_handler(ctx: Context<MintRankedReward>) -> Result<()> {
    let mut config_data: &[u8] = &ctx.accounts.ranked_config.try_borrow_data()?;
    let ranked_config = RankedConfig::try_deserialize(&mut config_data)?;

    let mut room_data: &[u8] = &ctx.accounts.ranked_room.try_borrow_data()?;
    let ranked_room = RankedRoom::try_deserialize(&mut room_data)?;

    require!(ranked_room.is_settled, GuessrError::RoomNotSettled);
    require!(
        ctx.accounts.reward_mint.key() == ranked_config.reward_mint,
        GuessrError::InvalidRewardMint
    );
    require!(
        ctx.accounts.player_token_account.owner == ranked_room.player,
        GuessrError::InvalidTokenOwner
    );
    require!(
        ctx.accounts.player_token_account.mint == ranked_config.reward_mint,
        GuessrError::InvalidTokenMint
    );

    let net_reward = ranked_room
        .total_earned
        .saturating_sub(ranked_room.total_lost);
    if net_reward == 0 {
        return Ok(());
    }

    let signer_seeds: &[&[u8]] = &[b"mint-authority", &[ranked_config.mint_authority_bump]];
    let cpi_accounts = MintTo {
        mint: ctx.accounts.reward_mint.to_account_info(),
        to: ctx.accounts.player_token_account.to_account_info(),
        authority: ctx.accounts.mint_authority.to_account_info(),
    };
    token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            cpi_accounts,
            &[signer_seeds],
        ),
        net_reward,
    )?;

    Ok(())
}

pub fn mint_duel_rewards_handler(ctx: Context<MintDuelRewards>) -> Result<()> {
    let mut config_data: &[u8] = &ctx.accounts.ranked_config.try_borrow_data()?;
    let ranked_config = RankedConfig::try_deserialize(&mut config_data)?;

    let mut room_data: &[u8] = &ctx.accounts.duel_room.try_borrow_data()?;
    let duel_room = DuelRoom::try_deserialize(&mut room_data)?;

    require!(duel_room.is_settled, GuessrError::DuelNotSettled);
    require!(
        ctx.accounts.reward_mint.key() == ranked_config.reward_mint,
        GuessrError::InvalidRewardMint
    );
    require!(
        ctx.accounts.host_token_account.owner == duel_room.host,
        GuessrError::InvalidTokenOwner
    );
    require!(
        ctx.accounts.challenger_token_account.owner == duel_room.challenger,
        GuessrError::InvalidTokenOwner
    );
    require!(
        ctx.accounts.host_token_account.mint == ranked_config.reward_mint,
        GuessrError::InvalidTokenMint
    );
    require!(
        ctx.accounts.challenger_token_account.mint == ranked_config.reward_mint,
        GuessrError::InvalidTokenMint
    );

    let host_reward = duel_room
        .host_earned
        .saturating_sub(duel_room.host_lost);
    let challenger_reward = duel_room
        .challenger_earned
        .saturating_sub(duel_room.challenger_lost);

    if host_reward == 0 && challenger_reward == 0 {
        return Ok(());
    }

    let signer_seeds: &[&[u8]] = &[b"mint-authority", &[ranked_config.mint_authority_bump]];

    if host_reward > 0 {
        let cpi_accounts = MintTo {
            mint: ctx.accounts.reward_mint.to_account_info(),
            to: ctx.accounts.host_token_account.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        };
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                &[signer_seeds],
            ),
            host_reward,
        )?;
    }

    if challenger_reward > 0 {
        let cpi_accounts = MintTo {
            mint: ctx.accounts.reward_mint.to_account_info(),
            to: ctx.accounts.challenger_token_account.to_account_info(),
            authority: ctx.accounts.mint_authority.to_account_info(),
        };
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                cpi_accounts,
                &[signer_seeds],
            ),
            challenger_reward,
        )?;
    }

    Ok(())
}

#[action]
#[derive(Accounts)]
pub struct MintRankedReward<'info> {
    /// CHECK: Delegated/base RankedConfig PDA.
    pub ranked_config: UncheckedAccount<'info>,
    /// CHECK: Delegated/base RankedRoom PDA.
    pub ranked_room: UncheckedAccount<'info>,
    #[account(mut)]
    pub reward_mint: Account<'info, Mint>,
    /// CHECK: Mint authority PDA.
    pub mint_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub player_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[action]
#[derive(Accounts)]
pub struct MintDuelRewards<'info> {
    /// CHECK: Delegated/base RankedConfig PDA.
    pub ranked_config: UncheckedAccount<'info>,
    /// CHECK: Delegated/base DuelRoom PDA.
    pub duel_room: UncheckedAccount<'info>,
    #[account(mut)]
    pub reward_mint: Account<'info, Mint>,
    /// CHECK: Mint authority PDA.
    pub mint_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub host_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub challenger_token_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

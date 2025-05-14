use solana_sdk::pubkey::Pubkey;

/// Direction of the swap: A to B or B to A.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeDirection {
    AtoB,
    BtoA,
}

/// All accounts required to build a Meteora swap instruction.
#[derive(Debug, Clone)]
pub struct MeteoraSwapParams {
    pub pool: Pubkey,
    pub direction: TradeDirection,

    // User token accounts
    pub user_source: Pubkey,      // WSOL ATA
    pub user_destination: Pubkey, // ATA for the target token (will be created if absent)

    // Pool vaults
    pub a_vault: Pubkey,
    pub b_vault: Pubkey,

    // Token vaults inside vault program
    pub a_token_vault: Pubkey,
    pub b_token_vault: Pubkey,

    // LP related accounts
    pub a_vault_lp_mint: Pubkey,
    pub b_vault_lp_mint: Pubkey,
    pub a_vault_lp: Pubkey,
    pub b_vault_lp: Pubkey,

    // Protocol fee token account (depends on side)
    pub protocol_token_fee: Pubkey,

    // Programs
    pub vault_program: Pubkey,
    pub token_program: Pubkey,

    pub mint_target_token: Pubkey,
}

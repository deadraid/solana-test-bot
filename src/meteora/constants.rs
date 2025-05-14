// Program ID for Meteora Dynamic AMM pools program.
pub const METEORA_PROGRAM_ID: &str = "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB";

// Wrapped SOL mint address on Solana mainnet.
pub const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

pub mod init_pool_indices {
    pub const POOL: usize = 0;
    #[allow(dead_code)]
    pub const CONFIG: usize = 1;
    #[allow(dead_code)]
    pub const LP_MINT: usize = 2;
    pub const TOKEN_A_MINT: usize = 3;
    pub const TOKEN_B_MINT: usize = 4;
    pub const A_VAULT: usize = 5;
    pub const B_VAULT: usize = 6;
    pub const A_TOKEN_VAULT: usize = 7;
    pub const B_TOKEN_VAULT: usize = 8;
    pub const A_VAULT_LP_MINT: usize = 9;
    pub const B_VAULT_LP_MINT: usize = 10;
    pub const A_VAULT_LP: usize = 11;
    pub const B_VAULT_LP: usize = 12;
    #[allow(dead_code)]
    pub const PAYER_TOKEN_A: usize = 13;
    #[allow(dead_code)]
    pub const PAYER_TOKEN_B: usize = 14;
    #[allow(dead_code)]
    pub const PAYER_POOL_LP: usize = 15;
    pub const PROTOCOL_TOKEN_A_FEE: usize = 16;
    pub const PROTOCOL_TOKEN_B_FEE: usize = 17;
    // indices 18-21 correspond to rent, metadata etc. and are not required for buying.
    pub const VAULT_PROGRAM: usize = 22;
    pub const TOKEN_PROGRAM: usize = 23;
}

/// Hard-coded account indices for the `swap` instruction.
pub mod swap_indices {
    #[allow(dead_code)]
    pub const POOL: usize = 0;
    #[allow(dead_code)]
    pub const USER_SOURCE_TOKEN: usize = 1;
    #[allow(dead_code)]
    pub const USER_DEST_TOKEN: usize = 2;
    #[allow(dead_code)]
    pub const A_VAULT: usize = 3;
    #[allow(dead_code)]
    pub const B_VAULT: usize = 4;
    #[allow(dead_code)]
    pub const A_TOKEN_VAULT: usize = 5;
    #[allow(dead_code)]
    pub const B_TOKEN_VAULT: usize = 6;
    #[allow(dead_code)]
    pub const A_VAULT_LP_MINT: usize = 7;
    #[allow(dead_code)]
    pub const B_VAULT_LP_MINT: usize = 8;
    #[allow(dead_code)]
    pub const A_VAULT_LP: usize = 9;
    #[allow(dead_code)]
    pub const B_VAULT_LP: usize = 10;
    #[allow(dead_code)]
    pub const PROTOCOL_TOKEN_FEE: usize = 11;
    #[allow(dead_code)]
    pub const USER: usize = 12;
    #[allow(dead_code)]
    pub const VAULT_PROGRAM: usize = 13;
    #[allow(dead_code)]
    pub const TOKEN_PROGRAM: usize = 14;
}

// 8-byte discriminators for Anchor instructions
// Value for initializePermissionlessConstantProductPoolWithConfig2 from transaction 5QWwTAMs...
pub const INIT_POOL_DISCRIM: [u8; 8] = [48, 149, 220, 130, 61, 11, 9, 178]; // Hex: [0x30, 0x95, 0xdc, 0x82, 0x3d, 0x0b, 0x09, 0xb2]
/// Discriminator for `initializePermissionlessConstantProductPoolWithConfig` (v1)
pub const INIT_POOL_DISCRIM_V1: [u8; 8] = [0x22, 0x80, 0x79, 0x2d, 0xab, 0x3e, 0xd2, 0x7e];
pub const SWAP_DISCRIM: [u8; 8] = [0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];

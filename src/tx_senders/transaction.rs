use crate::config::{PingThingsArgs, RpcType};
use crate::meteora::constants::METEORA_PROGRAM_ID;
use crate::meteora::types::{MeteoraSwapParams, TradeDirection};
use crate::tx_senders::constants::JITO_TIP_ADDR;
use crate::tx_senders::constants::TOKEN_PROGRAM_ADDR;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::hash::Hash;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::v0::Message;
use solana_sdk::message::VersionedMessage;
use solana_sdk::native_token::LAMPORTS_PER_SOL;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::system_instruction;
use solana_sdk::transaction::VersionedTransaction;
use spl_associated_token_account::instruction::create_associated_token_account_idempotent;
use spl_token::instruction as token_instruction;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Clone)]
pub struct TransactionConfig {
    pub keypair: Arc<Keypair>,
    pub compute_unit_limit: u32,
    pub compute_unit_price: u64,
    pub tip: u64,
    pub buy_amount: u64,
    pub min_amount_out: u64,
}

// Custom Debug implementation to redact sensitive keypair data
impl fmt::Debug for TransactionConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransactionConfig")
            .field("keypair_pubkey", &self.keypair.pubkey())
            .field("compute_unit_limit", &self.compute_unit_limit)
            .field("compute_unit_price", &self.compute_unit_price)
            .field("tip", &self.tip)
            .field("buy_amount", &self.buy_amount)
            .field("min_amount_out", &self.min_amount_out)
            .finish()
    }
}

impl From<PingThingsArgs> for TransactionConfig {
    fn from(args: PingThingsArgs) -> Self {
        let keypair = Keypair::from_base58_string(args.private_key.as_str());

        let tip: u64 = (args.tip * LAMPORTS_PER_SOL as f64) as u64;
        let buy_amount: u64 = (args.buy_amount * LAMPORTS_PER_SOL as f64) as u64;
        let min_amount_out: u64 = (args.min_amount_out * 1_000_000 as f64) as u64;

        TransactionConfig {
            keypair: Arc::new(keypair),
            compute_unit_limit: args.compute_unit_limit,
            compute_unit_price: args.compute_unit_price,
            tip: tip,
            buy_amount: buy_amount,
            min_amount_out: min_amount_out,
        }
    }
}

pub fn build_meteora_swap_tx(
    tx_config: &TransactionConfig,
    rpc_type: &RpcType,
    recent_blockhash: Hash,
    params: &MeteoraSwapParams,
) -> VersionedTransaction {
    let mut instructions: Vec<Instruction> = Vec::new();

    // Priority fee instructions
    if tx_config.compute_unit_limit > 0 {
        instructions.push(ComputeBudgetInstruction::set_compute_unit_limit(
            tx_config.compute_unit_limit,
        ));
    }
    if tx_config.compute_unit_price > 0 {
        instructions.push(ComputeBudgetInstruction::set_compute_unit_price(
            tx_config.compute_unit_price,
        ));
    }

    // Optional Jito tip
    if tx_config.tip > 0 {
        if let RpcType::Jito = rpc_type {
            instructions.push(system_instruction::transfer(
                &tx_config.keypair.pubkey(),
                &Pubkey::from_str(JITO_TIP_ADDR).unwrap(),
                tx_config.tip,
            ));
        }
    }

    // Pre-compute user WSOL ATA for later transfer/sync instructions.
    let user_source_wsol = spl_associated_token_account::get_associated_token_address(
        &tx_config.keypair.pubkey(),
        &Pubkey::from_str(crate::meteora::constants::WSOL_MINT).unwrap(),
    );

    // 1) Create WSOL ATA (idempotent)
    let create_src_ata_ix = create_associated_token_account_idempotent(
        &tx_config.keypair.pubkey(),
        &tx_config.keypair.pubkey(),
        &Pubkey::from_str(crate::meteora::constants::WSOL_MINT).unwrap(),
        &Pubkey::from_str(TOKEN_PROGRAM_ADDR).unwrap(),
    );
    instructions.push(create_src_ata_ix);

    // 2) Transfer the SOL we intend to swap into the WSOL ATA.
    let transfer_lamports_ix = system_instruction::transfer(
        &tx_config.keypair.pubkey(),
        &user_source_wsol,
        tx_config.buy_amount,
    );
    instructions.push(transfer_lamports_ix);

    // 3) Sync native account to turn lamports into WSOL balance.
    //    Data layout for SyncNative: command index 17 (u8) + three bytes padding (u8) per Token program spec.
    let sync_native_ix = Instruction {
        program_id: Pubkey::from_str(TOKEN_PROGRAM_ADDR).unwrap(),
        accounts: vec![AccountMeta::new(user_source_wsol, false)],
        data: vec![17, 0, 0, 0],
    };
    instructions.push(sync_native_ix);

    // 4) Ensure user destination ATA exists (must be ready before swap to receive tokens).
    let create_dst_ata_ix = create_associated_token_account_idempotent(
        &tx_config.keypair.pubkey(),
        &tx_config.keypair.pubkey(),
        &params.mint_target_token, // newly listed token mint
        &Pubkey::from_str(TOKEN_PROGRAM_ADDR).unwrap(),
    );
    instructions.push(create_dst_ata_ix);

    // Swap instruction data: [discriminator (8 bytes)] + [in_amount (u64)] + [minimum_out (u64)] + [trade_direction (1 byte)]
    let mut data = vec![];
    // Anchor discriminator for `swap`
    data.extend_from_slice(&crate::meteora::constants::SWAP_DISCRIM);
    data.extend_from_slice(&tx_config.buy_amount.to_le_bytes());
    data.extend_from_slice(&tx_config.min_amount_out.to_le_bytes());
    // append trade direction (0 = AtoB, 1 = BtoA)
    let trade_dir: u8 = match params.direction {
        TradeDirection::AtoB => 0,
        TradeDirection::BtoA => 1,
    };
    data.push(trade_dir);

    // Account list following hard-coded indices (see constants).
    let accounts = vec![
        AccountMeta::new(params.pool, false),               // 0 pool
        AccountMeta::new(params.user_source, false),        // 1 user source WSOL
        AccountMeta::new(params.user_destination, false),   // 2 user dest token
        AccountMeta::new(params.a_vault, false),            // 3
        AccountMeta::new(params.b_vault, false),            // 4
        AccountMeta::new(params.a_token_vault, false),      // 5
        AccountMeta::new(params.b_token_vault, false),      // 6
        AccountMeta::new(params.a_vault_lp_mint, false),    // 7
        AccountMeta::new(params.b_vault_lp_mint, false),    // 8
        AccountMeta::new(params.a_vault_lp, false),         // 9
        AccountMeta::new(params.b_vault_lp, false),         // 10
        AccountMeta::new(params.protocol_token_fee, false), // 11
        AccountMeta::new_readonly(tx_config.keypair.pubkey(), true), // 12 user signer
        AccountMeta::new_readonly(params.vault_program, false), // 13
        AccountMeta::new_readonly(params.token_program, false), // 14
    ];

    let swap_ix = Instruction {
        program_id: Pubkey::from_str(METEORA_PROGRAM_ID).unwrap(),
        accounts,
        data,
    };

    // 5) Finally push swap instruction.
    instructions.push(swap_ix);

    // 6) Close empty WSOL account back to payer to reclaim rent.
    let close_wsol_ix = token_instruction::close_account(
        &Pubkey::from_str(TOKEN_PROGRAM_ADDR).unwrap(),
        &user_source_wsol,
        &tx_config.keypair.pubkey(),
        &tx_config.keypair.pubkey(),
        &[],
    )
    .unwrap();
    instructions.push(close_wsol_ix);

    let message_v0 = Message::try_compile(
        &tx_config.keypair.pubkey(),
        &instructions,
        &[],
        recent_blockhash,
    )
    .unwrap();

    let versioned_message = VersionedMessage::V0(message_v0);
    VersionedTransaction::try_new(versioned_message, &[&tx_config.keypair]).unwrap()
}

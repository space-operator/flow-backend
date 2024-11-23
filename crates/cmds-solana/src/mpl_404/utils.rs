use mpl_hybrid::{accounts::EscrowV1, instructions::CaptureV2Builder};
use solana_sdk::{commitment_config::CommitmentConfig, system_program};

use crate::{prelude::*, utils::ui_amount_to_amount};

use super::constants::{FEE_WALLET, SLOT_HASHES};

pub struct InitEscrowV2Accounts {
    pub payer: Wallet,
    pub authority: Wallet,
}

pub struct InitRecipeAccounts {
    pub payer: Wallet,
    pub authority: Wallet,
    pub collection: Pubkey,
    pub token: Pubkey,
    pub fee_location: Pubkey,
}

pub struct InitRecipeArgs {
    pub fee_token_decimals: u8,
    pub name: String,
    pub uri: String,
    pub max: u64,
    pub min: u64,
    pub amount: Decimal,
    pub fee_amount_capture: Decimal,
    pub fee_amount_release: Decimal,
    pub sol_fee_amount_capture: Decimal,
    pub sol_fee_amount_release: Decimal,
    pub path: u16,
}

pub struct CreateCollectionV2Accounts {
    pub payer: Wallet,
    pub update_authority: Option<Pubkey>,
    pub collection: Wallet,
}

pub struct CreateCollectionV2Args {
    pub name: String,
    pub uri: String,
}

pub struct CreateAssetV1Accounts {
    pub payer: Wallet,
    pub asset: Wallet,
    pub collection: Option<Pubkey>,
    pub authority: Option<Wallet>,
    pub owner: Option<Pubkey>,
}

pub struct CreateAssetV1Args {
    pub name: String,
    pub uri: String,
}

pub struct CaptureV2Accounts {
    pub owner: Wallet,
    pub authority: Wallet,
    pub asset: Pubkey,
    pub collection: Pubkey,
    pub token: Pubkey,
    pub fee_project_account: Pubkey,
}

pub async fn init_escrow_v2(
    ctx: &Context,
    accounts: InitEscrowV2Accounts,
) -> crate::Result<(Pubkey, Signature)> {
    let (escrow, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"escrow", accounts.authority.pubkey().as_ref()],
        &mpl_hybrid::ID,
    );

    let init_escrow_v2_ix = mpl_hybrid::instructions::InitEscrowV2Builder::new()
        .escrow(escrow)
        .authority(accounts.authority.pubkey())
        .instruction();

    submit(
        ctx,
        &[init_escrow_v2_ix],
        &accounts.payer.clone(),
        &[accounts.payer.clone(), accounts.authority.clone()],
    )
    .await
    .map(|signature| (escrow, signature))
}

pub async fn init_recipe(
    ctx: &Context,
    accounts: InitRecipeAccounts,
    args: InitRecipeArgs,
) -> crate::Result<Signature> {
    let sol_token_decimals = 9;

    let (recipe, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"recipe", accounts.collection.as_ref()],
        &mpl_hybrid::ID,
    );

    let fee_ata = spl_associated_token_account::get_associated_token_address(
        &accounts.fee_location,
        &accounts.token,
    );

    let init_recipe_ix = mpl_hybrid::instructions::InitRecipeBuilder::new()
        .recipe(recipe)
        .authority(accounts.authority.pubkey())
        .collection(accounts.collection)
        .token(accounts.token)
        .fee_location(accounts.fee_location)
        .fee_ata(fee_ata)
        .name(args.name)
        .uri(args.uri)
        .max(args.max)
        .min(args.min)
        .amount(ui_amount_to_amount(args.amount, args.fee_token_decimals)?)
        .fee_amount_capture(ui_amount_to_amount(
            args.fee_amount_capture,
            args.fee_token_decimals,
        )?)
        .fee_amount_release(ui_amount_to_amount(
            args.fee_amount_release,
            args.fee_token_decimals,
        )?)
        .sol_fee_amount_capture(ui_amount_to_amount(
            args.sol_fee_amount_capture,
            sol_token_decimals,
        )?)
        .sol_fee_amount_release(ui_amount_to_amount(
            args.sol_fee_amount_release,
            sol_token_decimals,
        )?)
        .path(args.path)
        .associated_token_program(spl_associated_token_account::id())
        .instruction();

    submit(
        ctx,
        &[init_recipe_ix],
        &accounts.payer.clone(),
        &[accounts.payer, accounts.authority],
    )
    .await
}

pub async fn create_collection_v2(
    ctx: &Context,
    accounts: CreateCollectionV2Accounts,
    args: CreateCollectionV2Args,
) -> crate::Result<Signature> {
    let mut builder = mpl_core::instructions::CreateCollectionV2Builder::new();
    let mut create_collection_v2_ix = builder
        .payer(accounts.payer.pubkey())
        .collection(accounts.collection.pubkey())
        .name(args.name)
        .uri(args.uri);

    if let Some(update_authority) = accounts.update_authority {
        create_collection_v2_ix = create_collection_v2_ix.update_authority(Some(update_authority));
    }

    submit(
        ctx,
        &[create_collection_v2_ix.instruction()],
        &accounts.payer.clone(),
        &[accounts.payer, accounts.collection],
    )
    .await
}

pub async fn create_asset_v1(
    ctx: &Context,
    accounts: CreateAssetV1Accounts,
    args: CreateAssetV1Args,
) -> crate::Result<Signature> {
    let mut builder = mpl_core::instructions::CreateV1Builder::new();
    let mut create_asset_v1_ix = builder
        .payer(accounts.payer.pubkey())
        .asset(accounts.asset.pubkey())
        .collection(accounts.collection)
        .owner(accounts.owner)
        .name(args.name)
        .uri(args.uri);

    match accounts.authority {
        Some(authority) => {
            create_asset_v1_ix = create_asset_v1_ix.authority(Some(authority.pubkey()));
            submit(
                ctx,
                &[create_asset_v1_ix.instruction()],
                &accounts.payer.clone(),
                &[accounts.payer, accounts.asset, authority],
            )
            .await
        }
        None => {
            submit(
                ctx,
                &[create_asset_v1_ix.instruction()],
                &accounts.payer.clone(),
                &[accounts.payer, accounts.asset],
            )
            .await
        }
    }
}

pub async fn capture_v2(ctx: &Context, accounts: CaptureV2Accounts) -> crate::Result<Signature> {
    let (recipe, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"recipe", accounts.collection.as_ref()],
        &mpl_hybrid::ID,
    );

    let (escrow, _bump) = solana_sdk::pubkey::Pubkey::find_program_address(
        &[b"escrow", accounts.authority.pubkey().as_ref()],
        &mpl_hybrid::ID,
    );

    // must already be initialized
    let user_token_account = spl_associated_token_account::get_associated_token_address(
        &accounts.owner.pubkey(),
        &accounts.token,
    );

    let escrow_token_account =
        spl_associated_token_account::get_associated_token_address(&escrow, &accounts.token);

    let fee_token_account = spl_associated_token_account::get_associated_token_address(
        &accounts.fee_project_account,
        &accounts.token,
    );

    let capture_v2_ix = CaptureV2Builder::new()
        .owner(accounts.owner.pubkey())
        .authority(accounts.authority.pubkey())
        .recipe(recipe)
        .escrow(escrow)
        .asset(accounts.asset)
        .collection(accounts.collection)
        .token(accounts.token)
        .user_token_account(user_token_account)
        .escrow_token_account(escrow_token_account)
        .fee_token_account(fee_token_account)
        .fee_project_account(accounts.fee_project_account)
        .fee_sol_account(FEE_WALLET)
        .recent_blockhashes(SLOT_HASHES)
        .mpl_core(mpl_core::ID)
        .associated_token_program(spl_associated_token_account::ID)
        .instruction();

    submit(
        ctx,
        &[capture_v2_ix],
        &accounts.owner.clone(),
        &[accounts.owner, accounts.authority],
    )
    .await
}

pub async fn transfer_sol(
    ctx: &Context,
    from_pubkey: Wallet,
    to_pubkey: Pubkey,
    amount: Decimal,
) -> crate::Result<Signature> {
    let ix = solana_sdk::system_instruction::transfer(
        &from_pubkey.pubkey(),
        &to_pubkey,
        ui_amount_to_amount(amount, 9)?,
    );

    submit(ctx, &[ix], &from_pubkey.clone(), &[from_pubkey]).await
}

pub async fn init_ata_if_needed(
    ctx: &Context,
    fee_payer: Wallet,
    owner: Pubkey,
    token_mint: Pubkey,
) -> crate::Result<Option<Signature>> {
    let commitment = CommitmentConfig::confirmed();

    let mut instructions = vec![];
    let recipient_token_account =
        spl_associated_token_account::get_associated_token_address(&owner, &token_mint);

    let needs_funding = {
        if let Some(recipient_token_account_data) = ctx
            .solana_client
            .get_account_with_commitment(&recipient_token_account, commitment)
            .await?
            .value
        {
            match recipient_token_account_data.owner {
                x if x == system_program::ID => true,
                y if y == spl_token::ID => false,
                _ => return Err(crate::Error::UnsupportedRecipientAddress(owner.to_string())),
            }
        } else {
            true
        }
    };

    if needs_funding {
        instructions.push(
            spl_associated_token_account::instruction::create_associated_token_account(
                &fee_payer.pubkey(),
                &owner,
                &token_mint,
                &spl_token::ID,
            ),
        );
    } else {
        return Ok(None);
    }

    submit(ctx, &instructions, &fee_payer.clone(), &[fee_payer])
        .await
        .map(Some)
}

pub async fn get_escrow_v1(ctx: &Context, escrow: Pubkey) -> Result<EscrowV1, crate::Error> {
    let get_escrow_account_response = ctx
        .solana_client
        .get_account_with_commitment(&escrow, CommitmentConfig::confirmed())
        .await
        .map_err(|e| {
            tracing::error!("Error: {:?}", e);
            crate::Error::AccountNotFound(escrow)
        })?;

    let escrow_account = match get_escrow_account_response.value {
        Some(account) => account,
        None => return Err(crate::Error::AccountNotFound(escrow)),
    };

    let escrow_data: &[u8] = &escrow_account.data;
    let escrow_data: EscrowV1 = EscrowV1::from_bytes(escrow_data).map_err(|_| {
        tracing::error!(
            "Invalid data from EscrowV1: {:?}",
            crate::Error::InvalidAccountData(escrow)
        );
        crate::Error::InvalidAccountData(escrow)
    })?;

    Ok(escrow_data)
}

pub async fn submit(
    ctx: &Context,
    ixs: &[Instruction],
    payer: &Wallet,
    signers: &[Wallet],
) -> crate::Result<Signature> {
    let (mut tx, recent_blockhash) = execute(&ctx.solana_client, &payer.pubkey(), ixs).await?;

    let mut all_signers = vec![payer.keypair().unwrap()];
    all_signers.extend(signers.iter().map(|w| w.keypair().unwrap()));
    let _ = tx.try_sign(&all_signers, recent_blockhash).unwrap();

    submit_transaction(&ctx.solana_client, tx).await
}

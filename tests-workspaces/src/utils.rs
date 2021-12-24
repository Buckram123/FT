use near_units::{parse_gas, parse_near};
use serde_json::json;
use serde_json::Number;
use workspaces::prelude::*;
use workspaces::DevNetwork;
use workspaces::Worker;

const FT_WASM_FILEPATH: &str = "../res/fungible_token.wasm";
const DEFI_WASM_FILEPATH: &str = "../res/defi.wasm";

pub const STORAGE_PRICE_PER_BYTE: u128 = 10_000_000_000_000_000_000;

pub async fn register_user(
    user: &workspaces::Account,
    worker: &workspaces::Worker<impl DevNetwork>,
    contract_id: workspaces::AccountId,
) -> anyhow::Result<()> {
    user.call(worker, contract_id, "storage_deposit")
        .args_json(serde_json::json!({
            "account_id": user.id()
        }))?
        .gas(parse_gas!("150 Tgas") as u64)
        .deposit(STORAGE_PRICE_PER_BYTE * 125)
        .transact()
        .await?;
    Ok(())
}

pub async fn init_no_defi(
    worker: &workspaces::Worker<impl DevNetwork>,
    root_id: &workspaces::AccountId,
    initial_balance: u128,
) -> anyhow::Result<(workspaces::Contract, workspaces::Account)> {
    let wasm = std::fs::read(FT_WASM_FILEPATH)?;
    let contract = worker.dev_deploy(wasm).await?;
    contract
        .call(&worker, "new_default_meta")
        .args_json(serde_json::json!({
            "owner_id": root_id,
            "total_supply": initial_balance.to_string(),
        }))?
        .gas(parse_gas!("150 Tgas") as u64)
        .transact()
        .await?;

    let alice = worker.dev_create_account().await?;
    register_user(&alice, worker, contract.id().clone()).await?;

    Ok((contract, alice))
}

pub async fn init_defi(
    worker: &workspaces::Worker<impl DevNetwork>,
    root_id: &workspaces::AccountId,
    initial_balance: u128,
) -> anyhow::Result<(workspaces::Contract, workspaces::Contract, workspaces::Account)> {
    let wasm = std::fs::read(DEFI_WASM_FILEPATH)?;
    let contract = worker.dev_deploy(wasm).await?;
    contract
        .call(&worker, "new_default_meta")
        .args_json(serde_json::json!({
            "owner_id": root_id,
            "total_supply": initial_balance.to_string(),
        }))?
        .gas(parse_gas!("150 Tgas") as u64)
        .transact()
        .await?;

    let alice = worker.dev_create_account().await?;

    let wasm = std::fs::read(DEFI_WASM_FILEPATH)?;
    let defi = worker.dev_deploy(wasm).await?;
    Ok((contract, defi, alice))
}

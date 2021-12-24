use crate::utils::init_no_defi;
use near_units::{parse_gas, parse_near};

#[tokio::test]
async fn total_supply() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let initial_balance = parse_near!("100 N");
    let (ft, _) = init_no_defi(&worker, worker.root_account().id(), initial_balance).await?;

    let total_supply: String = ft.view(&worker, "ft_total_supply", vec![]).await?.json()?;
    assert_eq!(total_supply.parse::<u128>()?, initial_balance);
    Ok(())
}

#[tokio::test]
async fn simple_transfer() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let transfer_amount = parse_near!("100 N");
    let initial_balance = parse_near!("100000 N");

    let (ft, alice) = init_no_defi(&worker, worker.root_account().id(), initial_balance).await?;
    let owner = worker.root_account();
    owner
        .call(&worker, ft.id().clone(), "ft_transfer")
        .args_json(serde_json::json!({
        "receiver_id": alice.id(),
        "amount": transfer_amount.to_string(),
        }))?
        .gas(parse_gas!("300 Tgas") as u64)
        .deposit(1)
        .transact()
        .await?;

    let root_balance: String = ft
        .view(
            &worker,
            "ft_balance_of",
            serde_json::json!({
                "account_id": owner.id()
            })
            .to_string()
            .into_bytes(),
        )
        .await?
        .json()?;
    let alice_balance: String = ft
        .view(
            &worker,
            "ft_balance_of",
            serde_json::json!({
                "account_id": alice.id()
            })
            .to_string()
            .into_bytes(),
        )
        .await?
        .json()?;
    assert_eq!(initial_balance - transfer_amount, root_balance.parse::<u128>()?);
    assert_eq!(transfer_amount, alice_balance.parse::<u128>()?);
    Ok(())
}

use crate::utils::{init_defi, register_user};
use near_units::{parse_gas, parse_near};

#[tokio::test]
async fn total_supply_defi() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let initial_balance = parse_near!("100 N");
    let (ft, _, _) = init_defi(&worker, worker.root_account().id(), initial_balance).await?;

    let total_supply: String = ft.view(&worker, "ft_total_supply", vec![]).await?.json()?;
    assert_eq!(total_supply.parse::<u128>()?, initial_balance);
    Ok(())
}

#[tokio::test]
async fn simple_transfer_defi() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let transfer_amount = parse_near!("100 N");
    let initial_balance = parse_near!("100000 N");

    let (ft, _, alice) = init_defi(&worker, worker.root_account().id(), initial_balance).await?;
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

#[tokio::test]
async fn close_account_non_empty_balance() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let initial_balance = parse_near!("100000 N");

    let (ft, _, _) = init_defi(&worker, worker.root_account().id(), initial_balance).await?;
    let owner = worker.root_account();
    let outcome =
        owner.call(&worker, ft.id().clone(), "storage_unregister").deposit(1).transact().await?;
    match outcome.status {
        near_primitives::views::FinalExecutionStatus::Failure(err) => println!("err: {}", err),
        _ => panic!(),
    };

    let outcome = owner
        .call(&worker, ft.id().clone(), "storage_unregister")
        .args_json(serde_json::json!({
            "force": false
        }))?
        .deposit(1)
        .transact()
        .await?;
    match outcome.status {
        near_primitives::views::FinalExecutionStatus::Failure(err) => assert!(format!("{}", err)
            .contains("Can't unregister the account with the positive balance without force")),
        _ => panic!(),
    };
    Ok(())
}

#[tokio::test]
async fn close_account_force_non_empty_balance() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let initial_balance = parse_near!("100000 N");

    let (ft, _, _) = init_defi(&worker, worker.root_account().id(), initial_balance).await?;
    let owner = worker.root_account();
    let outcome = owner
        .call(&worker, ft.id().clone(), "storage_unregister")
        .args_json(serde_json::json!({
            "force": true
        }))?
        .deposit(1)
        .transact()
        .await?;
    // should be a way to read logs here
    match outcome.status {
        near_primitives::views::FinalExecutionStatus::SuccessValue(_) => (),
        _ => panic!(),
    };
    assert!(outcome.json::<bool>()?);
    let total_supply: String = ft.view(&worker, "ft_total_supply", vec![]).await?.json()?;
    assert_eq!(total_supply.parse::<u128>()?, 0);
    Ok(())
}

#[tokio::test]
async fn transfer_call_with_burned_amount() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let transfer_amount = parse_near!("100 N");
    let initial_balance = parse_near!("1000 N");

    let (ft, defi, _) = init_defi(&worker, worker.root_account().id(), initial_balance).await?;
    let owner = worker.root_account();
    register_user(defi.as_account(), &worker, ft.id().clone()).await?;
    let transfer_call_outcome = owner
        .call(&worker, ft.id().clone(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": defi.id(),
            "amount": transfer_amount.to_string(),
            "msg": "10",
        }))?
        .deposit(1)
        .gas(parse_gas!("150 Tgas") as u64)
        .transact()
        .await?;
    let outcome = owner
        .call(&worker, ft.id().clone(), "storage_unregister")
        .args_json(serde_json::json!({
            "force": true
        }))?
        .deposit(1)
        .transact()
        .await?;

    assert!(outcome.json::<bool>()?);
    let total_supply: String = transfer_call_outcome.json()?; // how we reach to callback result?
    assert_eq!(total_supply.parse::<u128>()?, transfer_amount - 10);

    let total_supply: String = ft.view(&worker, "ft_total_supply", vec![]).await?.json()?;
    assert_eq!(total_supply.parse::<u128>()?, transfer_amount - 10);

    let defi_balance: String = ft
        .view(
            &worker,
            "ft_balance_of",
            serde_json::json!({
                "account_id": defi.id()
            })
            .to_string()
            .into_bytes(),
        )
        .await?
        .json()?;
    assert_eq!(defi_balance.parse::<u128>()?, transfer_amount - 10);
    Ok(())
}

#[tokio::test]
async fn transfer_call_with_immediate_return_and_no_refund() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let transfer_amount = parse_near!("100 N");
    let initial_balance = parse_near!("1000 N");

    let (ft, defi, _) = init_defi(&worker, worker.root_account().id(), initial_balance).await?;
    let owner = worker.root_account();

    register_user(defi.as_account(), &worker, ft.id().clone()).await?;
    let outcome = owner
        .call(&worker, ft.id().clone(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": defi.id(),
            "amount": transfer_amount.to_string(),
            "msg": "take-my-money",
        }))?
        .deposit(1)
        .gas(parse_gas!("150 Tgas") as u64)
        .transact()
        .await?;
    assert!(matches!(
        outcome.status,
        near_primitives::views::FinalExecutionStatus::SuccessValue(_)
    ));

    let defi_balance: String = ft
        .view(
            &worker,
            "ft_balance_of",
            serde_json::json!({
                "account_id": defi.id()
            })
            .to_string()
            .into_bytes(),
        )
        .await?
        .json()?;

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
    assert_eq!(initial_balance - transfer_amount, root_balance.parse::<u128>()?);
    assert_eq!(transfer_amount, defi_balance.parse::<u128>()?);
    Ok(())
}

#[tokio::test]
async fn transfer_call_when_called_contract_not_registered_with_ft() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let transfer_amount = parse_near!("100 N");
    let initial_balance = parse_near!("1000 N");

    let (ft, defi, _) = init_defi(&worker, worker.root_account().id(), initial_balance).await?;
    let owner = worker.root_account();

    // call fails because DEFI contract is not registered as FT user
    owner
        .call(&worker, ft.id().clone(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": defi.id(),
            "amount": transfer_amount.to_string(),
            "msg": "take-my-money",
        }))?
        .deposit(1)
        .gas(parse_gas!("150 Tgas") as u64)
        .transact()
        .await?;

    let defi_balance: String = ft
        .view(
            &worker,
            "ft_balance_of",
            serde_json::json!({
                "account_id": defi.id()
            })
            .to_string()
            .into_bytes(),
        )
        .await?
        .json()?;

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
    assert_eq!(initial_balance, root_balance.parse::<u128>()?);
    assert_eq!(0, defi_balance.parse::<u128>()?);
    Ok(())
}

#[tokio::test]
async fn transfer_call_with_promise_and_refund() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let transfer_amount = parse_near!("100 N");
    let refund_amount = parse_near!("50 N");
    let initial_balance = parse_near!("1000 N");

    let (ft, defi, _) = init_defi(&worker, worker.root_account().id(), initial_balance).await?;
    let owner = worker.root_account();

    register_user(defi.as_account(), &worker, ft.id().clone()).await?;
    owner
        .call(&worker, ft.id().clone(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": defi.id(),
            "amount": transfer_amount.to_string(),
            "msg": refund_amount.to_string(),
        }))?
        .deposit(1)
        .gas(parse_gas!("150 Tgas") as u64)
        .transact()
        .await?;

    let defi_balance: String = ft
        .view(
            &worker,
            "ft_balance_of",
            serde_json::json!({
                "account_id": defi.id()
            })
            .to_string()
            .into_bytes(),
        )
        .await?
        .json()?;

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
    assert_eq!(initial_balance - transfer_amount + refund_amount, root_balance.parse::<u128>()?);
    assert_eq!(transfer_amount - refund_amount, defi_balance.parse::<u128>()?);
    Ok(())
}

#[tokio::test]
async fn transfer_call_promise_panics_for_a_full_refund() -> anyhow::Result<()> {
    let worker = workspaces::sandbox();
    let transfer_amount = parse_near!("100 N");
    let initial_balance = parse_near!("1000 N");

    let (ft, defi, _) = init_defi(&worker, worker.root_account().id(), initial_balance).await?;
    let owner = worker.root_account();

    register_user(defi.as_account(), &worker, ft.id().clone()).await?;
    let _outcome = owner
        .call(&worker, ft.id().clone(), "ft_transfer_call")
        .args_json(serde_json::json!({
            "receiver_id": defi.id(),
            "amount": transfer_amount.to_string(),
            "msg": "no parsey as integer big panic oh no",
        }))?
        .deposit(1)
        .gas(parse_gas!("150 Tgas") as u64)
        .transact()
        .await?;
    // cross-contract ExecutionDetails?
    let defi_balance: String = ft
        .view(
            &worker,
            "ft_balance_of",
            serde_json::json!({
                "account_id": defi.id()
            })
            .to_string()
            .into_bytes(),
        )
        .await?
        .json()?;

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
    assert_eq!(initial_balance, root_balance.parse::<u128>()?);
    assert_eq!(0, defi_balance.parse::<u128>()?);
    Ok(())
}

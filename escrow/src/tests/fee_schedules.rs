use super::{free_addresses, setup};
use crate::{FeeSchedule, FeeScheduleActivated, FeeScheduleSubmitted, StarfundEscrowClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events as _, Ledger as _},
    Address, Env, String,
};

fn init_fee_schedule_test<'a>(env: &'a Env) -> (StarfundEscrowClient<'a>, Address) {
    let (client, admin, sme) = setup(env);
    let (funding_token, treasury) = free_addresses(env);
    client.init(
        &admin,
        &String::from_str(env, "FEES01"),
        &sme,
        &100_000i128,
        &800i64,
        &0u64,
        &funding_token,
        &None,
        &treasury,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None,
        &None::<i64>,
        &None::<u32>,
    );
    (client, admin)
}

#[test]
fn fee_schedule_views_project_pending_activation_without_promotion() {
    let env = Env::default();
    let (client, _admin) = init_fee_schedule_test(&env);
    let contract_id = client.address.clone();

    let previous = FeeSchedule {
        fee_bps: 100,
        min_fee_bps: 50,
        max_fee_bps: 150,
        activation_ledger: 110,
    };
    client.submit_fee_schedule(&previous);
    let mut ledger = env.ledger().get();
    ledger.sequence_number = previous.activation_ledger;
    env.ledger().set(ledger);
    assert!(client.activate_fee_schedule());

    let pending = FeeSchedule {
        fee_bps: 200,
        min_fee_bps: 100,
        max_fee_bps: 250,
        activation_ledger: 130,
    };
    client.submit_fee_schedule(&pending);

    let storage_before = env.storage().get_contract_storage_snapshot(&contract_id);
    let events_before = env.events().all().events().len();

    // Immediately before activation: pending is still projected as pending.
    let mut ledger = env.ledger().get();
    ledger.sequence_number = pending.activation_ledger - 1;
    ledger.timestamp = 1_000;
    env.ledger().set(ledger);
    let active_before = client.get_active_fee_schedule();
    let pending_before = client.get_pending_fee_schedule();
    let previous_before = client.get_previous_fee_schedule();
    assert_eq!(active_before, Some(previous.clone()));
    assert_eq!(pending_before, Some(pending.clone()));
    assert_eq!(previous_before, None);

    // At activation ledger: projection promotes the pending schedule.
    let mut ledger = env.ledger().get();
    ledger.sequence_number = pending.activation_ledger;
    ledger.timestamp = 2_000;
    env.ledger().set(ledger);
    let active_at = client.get_active_fee_schedule();
    let pending_at = client.get_pending_fee_schedule();
    let previous_at = client.get_previous_fee_schedule();
    assert_eq!(active_at, Some(pending.clone()));
    assert_eq!(pending_at, None);
    assert_eq!(previous_at, Some(previous.clone()));

    // After activation ledger: projection stays promoted.
    let mut ledger = env.ledger().get();
    ledger.sequence_number = pending.activation_ledger + 1;
    ledger.timestamp = 3_000;
    env.ledger().set(ledger);
    let active_after = client.get_active_fee_schedule();
    let pending_after = client.get_pending_fee_schedule();
    let previous_after = client.get_previous_fee_schedule();
    assert_eq!(active_after, Some(pending.clone()));
    assert_eq!(pending_after, None);
    assert_eq!(previous_after, Some(previous.clone()));

    // Reads must not mutate storage or emit events.
    assert_eq!(
        env.storage().get_contract_storage_snapshot(&contract_id),
        storage_before
    );
    assert_eq!(env.events().all().events().len(), events_before);

    // Persisted promotion matches the projected views.
    assert!(client.activate_fee_schedule());
    assert_eq!(client.get_active_fee_schedule(), active_after);
    assert_eq!(client.get_pending_fee_schedule(), pending_after);
    assert_eq!(client.get_previous_fee_schedule(), previous_after);
    assert_eq!(
        env.events().all().events().last().unwrap().clone(),
        FeeScheduleActivated {
            name: symbol_short!("fee_act"),
            invoice_id: client.get_escrow().invoice_id,
            schedule: pending,
            previous: Some(previous),
        }
        .to_xdr(&env, &contract_id)
    );
}

#[test]
fn fee_schedule_changes_emit_events() {
    let env = Env::default();
    let (client, _admin) = init_fee_schedule_test(&env);
    let contract_id = client.address.clone();
    let first = FeeSchedule {
        fee_bps: 100,
        min_fee_bps: 50,
        max_fee_bps: 150,
        activation_ledger: 110,
    };

    client.submit_fee_schedule(&first);
    assert_eq!(
        env.events().all().events().last().unwrap().clone(),
        FeeScheduleSubmitted {
            name: symbol_short!("fee_sub"),
            invoice_id: client.get_escrow().invoice_id,
            schedule: first.clone(),
        }
        .to_xdr(&env, &contract_id)
    );

    let event_count = env.events().all().events().len();
    client.submit_fee_schedule(&first);
    assert_eq!(env.events().all().events().len(), event_count);

    let mut ledger = env.ledger().get();
    ledger.sequence_number = first.activation_ledger;
    env.ledger().set(ledger);
    assert!(client.activate_fee_schedule());
    assert_eq!(
        env.events().all().events().last().unwrap().clone(),
        FeeScheduleActivated {
            name: symbol_short!("fee_act"),
            invoice_id: client.get_escrow().invoice_id,
            schedule: first.clone(),
            previous: None,
        }
        .to_xdr(&env, &contract_id)
    );

    let second = FeeSchedule {
        fee_bps: 200,
        min_fee_bps: 100,
        max_fee_bps: 250,
        activation_ledger: 120,
    };
    client.submit_fee_schedule(&second);
    let mut ledger = env.ledger().get();
    ledger.sequence_number = second.activation_ledger;
    env.ledger().set(ledger);
    assert!(client.activate_fee_schedule());
    assert_eq!(
        env.events().all().events().last().unwrap().clone(),
        FeeScheduleActivated {
            name: symbol_short!("fee_act"),
            invoice_id: client.get_escrow().invoice_id,
            schedule: second,
            previous: Some(first),
        }
        .to_xdr(&env, &contract_id)
    );
}

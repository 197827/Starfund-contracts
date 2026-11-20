use super::{decode_events, setup, TestConfig};
use crate::{FeeSchedule, FeeScheduleActivated, FeeScheduleSubmitted, StarfundEscrowClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Ledger as _},
    Address, Env,
};

fn init_fee_schedule_test<'a>(env: &'a Env) -> (StarfundEscrowClient<'a>, Address) {
    let (client, admin, sme) = setup(env);
    TestConfig::new(env, "FEES01", &sme)
        .amount(100_000i128)
        .maturity(800i64)
        .init(&client, &admin);
    (client, admin)
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
    let submitted = decode_events::<FeeScheduleSubmitted>(
        &env,
        &contract_id,
        symbol_short!("fee_sub"),
    );
    assert_eq!(submitted.len(), 1);
    assert_eq!(
        submitted[0],
        FeeScheduleSubmitted {
            name: symbol_short!("fee_sub"),
            invoice_id: client.get_escrow().invoice_id,
            schedule: first.clone(),
        }
    );

    let event_count = env.events().all().events().len();
    client.submit_fee_schedule(&first);
    assert_eq!(env.events().all().events().len(), event_count);

    let mut ledger = env.ledger().get();
    ledger.sequence_number = first.activation_ledger;
    env.ledger().set(ledger);
    assert!(client.activate_fee_schedule());
    let activated = decode_events::<FeeScheduleActivated>(
        &env,
        &contract_id,
        symbol_short!("fee_act"),
    );
    assert_eq!(activated.len(), 1);
    assert_eq!(
        activated[0],
        FeeScheduleActivated {
            name: symbol_short!("fee_act"),
            invoice_id: client.get_escrow().invoice_id,
            schedule: first.clone(),
            previous: None,
        }
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
    let activated = decode_events::<FeeScheduleActivated>(
        &env,
        &contract_id,
        symbol_short!("fee_act"),
    );
    assert_eq!(activated.len(), 2);
    assert_eq!(
        activated[1],
        FeeScheduleActivated {
            name: symbol_short!("fee_act"),
            invoice_id: client.get_escrow().invoice_id,
            schedule: second,
            previous: Some(first),
        }
    );
}

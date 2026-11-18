use alloy_primitives::b256;
use alloy_sol_types::{SolEvent, SolValue};
use bitsafe_token::tokens::{beth::mint as beth, chype::mint as chype};

const NEW_TOPIC: chype::B256 =
    b256!("f3ca88974ec5963addc617fe6c0c359d7196759e318be6ab0cade940986e30e2");
const OLD_TOPIC: chype::B256 =
    b256!("acaea730ea841fae3326f72a7ff746b6e06c646840526bcaf5dabee5fd887ef8");

#[test]
fn chype_decodes_the_new_deposit_layout_through_its_public_api() {
    let account_id = chype::Bytes::from_static(b"chype-account");
    let account_hash = chype::deposit_id_topic("chype-account");
    let sender = chype::Address::repeat_byte(0x11);
    let amount = chype::U256::from(1_000_000_000_000_000_000_u64);
    let topics = [NEW_TOPIC, account_hash, sender.into_word()];

    assert_eq!(chype::DEPOSIT_EVENT_TOPIC, NEW_TOPIC);
    assert_eq!(beth::DEPOSIT_EVENT_TOPIC, NEW_TOPIC);

    for previous_block in [
        chype::U256::ZERO,
        chype::U256::from(12345),
        chype::U256::MAX,
    ] {
        let data = (account_id.clone(), amount, previous_block).abi_encode_params();
        let deposit = chype::Deposit::decode_raw_log(topics, &data).unwrap();

        assert_eq!(deposit.depositAccountIdHash, account_hash);
        assert_eq!(deposit.depositAccountId, account_id);
        assert_eq!(deposit.sender, sender);
        assert_eq!(deposit.amount, amount);
        assert_eq!(deposit.prevDepositBlock, previous_block);

        let beth_deposit: beth::Deposit = deposit;
        assert_eq!(beth_deposit.prevDepositBlock, previous_block);
    }
}

#[test]
fn chype_rejects_the_old_deposit_topic() {
    let account_id = chype::Bytes::from_static(b"chype-account");
    let topics = [
        OLD_TOPIC,
        chype::deposit_id_topic("chype-account"),
        chype::Address::repeat_byte(0x11).into_word(),
    ];
    let data = (account_id, chype::U256::from(1_000_000_000_000_000_000_u64)).abi_encode_params();

    assert!(chype::Deposit::decode_raw_log(topics, &data).is_err());
}

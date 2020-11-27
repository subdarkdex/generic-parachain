// Copyright 2019-2020
//     by  Centrality Investments Ltd.
//     and Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Tests for the module.

#![cfg(test)]
use super::*;
use crate::mock::{
    AccountId, Assets, Balances, ExtBuilder, Origin, System, TestEvent, TokenDealer,
};
use frame_support::assert_ok;
use sp_std::convert::TryInto;

fn encoded_to_remark(v: Vec<u8>) -> [u8; 32] {
    let boxed_slice = v.into_boxed_slice();
    let boxed_array: Box<[u8; 32]> = match boxed_slice.try_into() {
        Ok(ba) => ba,
        Err(o) => panic!("Expected a Vec of length {} but it was {}", 32, o.len()),
    };
    *boxed_array
}

#[test]
fn transfer_token_to_relay_settles_on_parachain_with_event() {
    let initial_amount = 10000;
    let transfer_amount = 1000;
    let from = [0u8; 32];
    let to = [1u8; 32];
    let relay_account: [u8; 32] = RelayAccount::default().into_account();
    let asset_id = None;
    let expected_event = TestEvent::token_dealer(RawEvent::TransferredTokensToRelayChain(
        from.into(),
        asset_id,
        to.into(),
        transfer_amount,
    ));

    ExtBuilder::default()
        .free_balance(vec![(from.into(), initial_amount)])
        .build()
        .execute_with(|| {
            assert_ok!(TokenDealer::transfer_tokens_to_relay_chain(
                Origin::signed(from.into()),
                to.into(),
                transfer_amount,
                asset_id
            ));
            let relay_account: AccountId = relay_account.into();
            let from: AccountId = from.into();
            assert_eq!(Balances::free_balance(relay_account), transfer_amount);
            assert_eq!(
                Balances::free_balance(from),
                initial_amount - transfer_amount
            );
            assert!(System::events()
                .iter()
                .any(|record| record.event == expected_event));
        });
}

#[test]
fn transfer_assets_to_relay_settles_on_parachain_with_event() {
    let initial_amount = 10000;
    let transfer_amount = 1000;
    let from = [0u8; 32];
    let to = [1u8; 32];
    let relay_account: [u8; 32] = RelayAccount::default().into_account();
    let asset_id = Some(0);
    let expected_event = TestEvent::token_dealer(RawEvent::TransferredTokensToRelayChain(
        from.into(),
        asset_id,
        to.into(),
        transfer_amount,
    ));

    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Assets::issue(Origin::signed(from.into()), initial_amount));
        assert_ok!(TokenDealer::transfer_tokens_to_relay_chain(
            Origin::signed(from.into()),
            to.into(),
            transfer_amount,
            asset_id
        ));
        assert_eq!(
            Assets::balance(0, from.into()),
            initial_amount - transfer_amount
        );
        assert_eq!(Assets::balance(0, relay_account.into()), transfer_amount);
        assert!(System::events()
            .iter()
            .any(|record| record.event == expected_event));
    });
}

#[test]
fn downward_message_tokens_settles_accounts_on_parachain_with_event() {
    let initial_amount = 10000;
    let transfer_amount = 9000;
    let dest = [0u8; 32];
    let remark = [0u8; 32];
    let relay_account: [u8; 32] = RelayAccount::default().into_account();
    let downward_message = DownwardMessage::TransferInto(dest.into(), transfer_amount, remark);
    let expected_event = TestEvent::token_dealer(RawEvent::TransferredTokensFromRelayChain(
        dest.into(),
        transfer_amount,
        None,
        Ok(()),
    ));
    ExtBuilder::default()
        .free_balance(vec![(relay_account.into(), initial_amount)])
        .build()
        .execute_with(|| {
            TokenDealer::handle_downward_message(&downward_message);
            let dest: AccountId = dest.into();
            let relay_account: AccountId = relay_account.into();
            assert_eq!(Balances::free_balance(dest), transfer_amount);
            assert_eq!(
                Balances::free_balance(relay_account),
                initial_amount - transfer_amount
            );
            assert!(System::events()
                .iter()
                .any(|record| record.event == expected_event));
        });
}

#[test]
fn downward_message_assets_settles_accounts_on_parachain_with_event() {
    let initial_amount = 10000;
    let transfer_amount = 9000;
    let dest = [0u8; 32];
    let mut remark = Some(0).encode();
    remark.resize(32, 0);
    let relay_account: [u8; 32] = RelayAccount::default().into_account();
    let downward_message =
        DownwardMessage::TransferInto(dest.into(), transfer_amount, encoded_to_remark(remark));
    let expected_event = TestEvent::token_dealer(RawEvent::TransferredTokensFromRelayChain(
        dest.into(),
        transfer_amount,
        Some(0),
        Ok(()),
    ));
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Assets::issue(
            Origin::signed(relay_account.into()),
            initial_amount
        ));
        TokenDealer::handle_downward_message(&downward_message);
        assert_eq!(
            Assets::balance(0, relay_account.into()),
            initial_amount - transfer_amount
        );
        assert_eq!(Assets::balance(0, dest.into()), transfer_amount);
        assert!(System::events()
            .iter()
            .any(|record| record.event == expected_event));
    });
}

#[test]
fn transfer_tokens_to_para_settles_accounts_on_parachain_with_event() {
    let from = [0u8; 32];
    let initial_amount = 10000;
    let transfer_amount = 9000;
    let asset_id_local = Some(1);
    let para_id: ParaId = 200.into();
    let dest = [0u8; 32];

    let expected_event = TestEvent::token_dealer(RawEvent::TransferredTokensToParachain(
        from.into(),
        asset_id_local,
        para_id,
        dest.into(),
        asset_id_local,
        transfer_amount,
    ));
    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Assets::issue(Origin::signed(from.into()), initial_amount));
        assert_ok!(Assets::issue(Origin::signed(from.into()), initial_amount));
        assert_ok!(TokenDealer::transfer_assets_to_parachain_chain(
            Origin::signed(from.into()),
            para_id.into(),
            dest.into(),
            transfer_amount,
            asset_id_local,
        ));
        assert_eq!(
            Assets::balance(asset_id_local.unwrap(), para_id.into_account()),
            transfer_amount
        );
        assert_eq!(
            Assets::balance(asset_id_local.unwrap(), from.into()),
            initial_amount - transfer_amount
        );
        assert!(System::events()
            .iter()
            .any(|record| record.event == expected_event));
    });
}

#[test]
fn make_transfer_to_relay_settles_accounts() {
    let from = [0u8; 32];
    let from: AccountId = from.into();
    let initial_amount = 10000;
    let transfer_amount = 9000;
    let asset_id_local = None;
    let dest = [0u8; 32];
    let relay_account: [u8; 32] = RelayAccount::default().into_account();
    let relay_account: AccountId = relay_account.into();
    ExtBuilder::default()
        .free_balance(vec![(from.clone(), initial_amount)])
        .build()
        .execute_with(|| {
            assert_ok!(TokenDealer::make_transfer_to_relay_chain(
                &asset_id_local,
                &from,
                &dest.into(),
                transfer_amount,
            ));
            assert_eq!(Balances::free_balance(relay_account), transfer_amount);
            assert_eq!(
                Balances::free_balance(from),
                initial_amount - transfer_amount
            );
        });
}

#[test]
fn make_transfer_to_para_settles_accounts() {
    let from = [0u8; 32];
    let initial_amount = 10000;
    let transfer_amount = 9000;
    let asset_id_local = Some(1);
    let asset_id_dest = Some(3);
    let para_id: ParaId = 200.into();
    let dest = [0u8; 32];

    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Assets::issue(Origin::signed(from.into()), initial_amount));
        assert_ok!(Assets::issue(Origin::signed(from.into()), initial_amount));
        assert_ok!(TokenDealer::make_transfer_to_parachain(
            &from.into(),
            &asset_id_local,
            para_id.into(),
            &dest.into(),
            &asset_id_dest,
            transfer_amount,
        ));
        assert_eq!(
            Assets::balance(asset_id_local.unwrap(), para_id.into_account()),
            transfer_amount
        );
        assert_eq!(
            Assets::balance(asset_id_local.unwrap(), from.into()),
            initial_amount - transfer_amount
        );
    });
}

#[test]
fn handle_xcmp_transfer_token_message_settles_accounts_on_parachain_with_event() {
    let dest = [0u8; 32];
    let initial_amount = 10000;
    let transfer_amount = 9000;
    let asset_id = None;
    let msg = XCMPMessage::TransferToken(dest.into(), transfer_amount, asset_id);
    let para_id: ParaId = 200.into();
    let para_account: [u8; 32] = para_id.into_account();
    let expected_event = TestEvent::token_dealer(RawEvent::TransferredTokensViaXCMP(
        para_id,
        dest.into(),
        transfer_amount,
        asset_id,
        Ok(()),
    ));

    ExtBuilder::default()
        .free_balance(vec![(para_id.into_account(), initial_amount)])
        .build()
        .execute_with(|| {
            let dest_account: AccountId = dest.into();
            let para_account_id: AccountId = para_account.into();
            TokenDealer::handle_xcmp_message(para_id, &msg);
            assert_eq!(Balances::free_balance(dest_account), transfer_amount);
            assert_eq!(
                Balances::free_balance(para_account_id),
                initial_amount - transfer_amount
            );
            assert!(System::events()
                .iter()
                .any(|record| record.event == expected_event));
        });
}

// Nope
#[test]
fn handle_xcmp_transfer_assets_message_settles_accounts_on_parachain_with_event() {
    let dest = [0u8; 32];
    let initial_amount = 10000;
    let transfer_amount = 9000;
    let asset_id = Some(0);
    let msg = XCMPMessage::TransferToken(dest.into(), transfer_amount, asset_id);
    let para_id: ParaId = 200.into();
    let para_account: [u8; 32] = para_id.into_account();
    let expected_event = TestEvent::token_dealer(RawEvent::TransferredTokensViaXCMP(
        para_id,
        dest.into(),
        transfer_amount,
        asset_id,
        Ok(()),
    ));

    ExtBuilder::default().build().execute_with(|| {
        assert_ok!(Assets::issue(
            Origin::signed(para_account.into()),
            initial_amount
        ));
        TokenDealer::handle_xcmp_message(para_id, &msg);
        assert_eq!(
            Assets::balance(asset_id.unwrap(), para_id.into_account()),
            initial_amount - transfer_amount
        );
        assert_eq!(
            Assets::balance(asset_id.unwrap(), dest.into()),
            transfer_amount
        );
        assert!(System::events()
            .iter()
            .any(|record| record.event == expected_event));
    });
}

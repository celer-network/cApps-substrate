use super::*;
use mock::*;
use sp_core::{sr25519, Pair, H256};
use frame_support::{assert_ok, assert_noop};


#[test]
fn test_pass_initiate() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, _) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players.clone(),
            timeout: 2,
            min_stone_offchain: 5,
            max_stone_onchain: 5,
        };

        assert_ok!(SingleGomoku::app_initiate(
            Origin::signed(players[0]),
            initiate_request)
        );
    })
}

#[test]
fn test_pass_update_by_state_and_player_2_win() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players.clone(),
            timeout: 2,
            min_stone_offchain: 5,
            max_stone_onchain: 5,
        };

        assert_ok!(SingleGomoku::app_initiate(
            Origin::signed(players[0]),
            initiate_request.clone())
        );

        let app_id = SingleGomoku::get_app_id(initiate_request.nonce, initiate_request.players.clone());
        let mut board_state = vec![0; 227];
        board_state[0] = 2; // winner
        board_state[1] = 0; // turn
        let state_proof = get_state_proof(0, 1, board_state, 0, app_id, players_pair);
        assert_ok!(
            SingleGomoku::update_by_state(
                Origin::signed(players[0]),
                state_proof
            )
        );
        assert_ok!(
            SingleGomoku::is_finalized(
                Origin::signed(players[0]),
                app_id
            )
        );
        assert_ok!(
            SingleGomoku::get_outcome(
                Origin::signed(players[0]),
                app_id,
                2
            )
        );
    })
}

#[test]
fn test_pass_state_new_game_and_update_by_state() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players.clone(),
            timeout: 2,
            min_stone_offchain: 5,
            max_stone_onchain: 5,
        };

        assert_ok!(SingleGomoku::app_initiate(
            Origin::signed(players[0]),
            initiate_request.clone())
        );

        let app_id = SingleGomoku::get_app_id(initiate_request.nonce, initiate_request.players.clone());
        let mut board_state = vec![0; 227];
        board_state[0] = 0;
        board_state[1] = 1;
        board_state[2] = 2;
        board_state[3] = 2;
        board_state[4] = 1;
        board_state[5] = 1;
        board_state[6] = 2;
        board_state[7] = 2;
        board_state[8] = 1;
        let state_proof = get_state_proof(0, 1, board_state, 0, app_id, players_pair);
        assert_ok!(
            SingleGomoku::update_by_state(
                Origin::signed(players[0]),
                state_proof
            )
        );

        let onchain_state = SingleGomoku::get_state(app_id, 2).unwrap();
        assert_eq!(onchain_state[0], 0);
        assert_eq!(onchain_state[1], 1);
        assert_eq!(onchain_state[2], 2);
        assert_eq!(onchain_state[3], 2);
        assert_eq!(onchain_state[4], 1);
        assert_eq!(onchain_state[5], 1);
        assert_eq!(onchain_state[6], 2);
        assert_eq!(onchain_state[7], 2);
        assert_eq!(onchain_state[8], 1);
    })
}

#[test]
fn test_fail_update_by_state_with_invalid_seq_num() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players.clone(),
            timeout: 2,
            min_stone_offchain: 5,
            max_stone_onchain: 5,
        };

        assert_ok!(SingleGomoku::app_initiate(
            Origin::signed(players[0]),
            initiate_request.clone())
        );

        let app_id = SingleGomoku::get_app_id(initiate_request.nonce, initiate_request.players.clone());
        let board_state = vec![0; 227];
        let state_proof = get_state_proof(0, 0, board_state, 0, app_id, players_pair);
        assert_noop!(
            SingleGomoku::update_by_state(
                Origin::signed(players[0]),
                state_proof
            ),
            "invalid sequence number"
        );
    })
}

#[test]
fn test_pass_update_by_state_with_higher_seq() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players.clone(),
            timeout: 2,
            min_stone_offchain: 5,
            max_stone_onchain: 5,
        };

        assert_ok!(SingleGomoku::app_initiate(
            Origin::signed(players[0]),
            initiate_request.clone())
        );

        let app_id = SingleGomoku::get_app_id(initiate_request.nonce, initiate_request.players.clone());
        let mut board_state_1 = vec![0; 227];
        board_state_1[0] = 0;
        board_state_1[1] = 1;
        board_state_1[2] = 2;
        board_state_1[3] = 2;
        board_state_1[4] = 1;
        board_state_1[5] = 1;
        board_state_1[6] = 2;
        board_state_1[7] = 2;
        board_state_1[8] = 1;
        let state_proof = get_state_proof(0, 1, board_state_1.clone(), 0, app_id, players_pair.clone());
        assert_ok!(
            SingleGomoku::update_by_state(
                Origin::signed(players[0]),
                state_proof
            )
        );

        let mut board_state_2 = vec![0; 227];
        board_state_2[0] = 0; // winner
        board_state_2[1] = 2; // turn
        board_state_2[2] = 1; // (0, 0)
        board_state_2[3] = 1; // (0, 1)
        board_state_2[4] = 1; // (0, 2)
        board_state_2[5] = 1; // (0, 3)
        board_state_2[101] = 2; 
        board_state_2[102] = 2;
        board_state_2[103] = 2;
        let state_proof = get_state_proof(0, 2, board_state_2, 0, app_id, players_pair);
        assert_ok!(
            SingleGomoku::update_by_state(
                Origin::signed(players[0]),
                state_proof
            )
        );

        let onchain_state = SingleGomoku::get_state(app_id, 2).unwrap();
        assert_eq!(onchain_state[0], 0);
        assert_eq!(onchain_state[1], 2);
        assert_eq!(onchain_state[2], 1);
        assert_eq!(onchain_state[3], 1);
        assert_eq!(onchain_state[4], 1);
        assert_eq!(onchain_state[5], 1);
        assert_eq!(onchain_state[6], 0);
        assert_eq!(onchain_state[7], 0);
        assert_eq!(onchain_state[8], 0);
        assert_eq!(onchain_state[101], 2);
        assert_eq!(onchain_state[102], 2);
        assert_eq!(onchain_state[103], 2);
    })
}

#[test]
fn test_player2_palces_stone_at_3_12_and_player1_takes_the_turn() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players.clone(),
            timeout: 2,
            min_stone_offchain: 5,
            max_stone_onchain: 5,
        };

        assert_ok!(SingleGomoku::app_initiate(
            Origin::signed(players[0]),
            initiate_request.clone())
        );

        let app_id = SingleGomoku::get_app_id(initiate_request.nonce, initiate_request.players.clone());
       
        // place stone 
        place_stone(app_id, players.clone(), players_pair);

        let settle_finalized_time = SingleGomoku::get_settle_finalized_time(app_id).unwrap();
        System::set_block_number(settle_finalized_time + 1);
        assert_ok!(
            SingleGomoku::update_by_action(
                Origin::signed(players[1]),
                app_id,
                vec![3, 12]
            )
        );
        let turn = SingleGomoku::get_state(app_id, 0).unwrap();
        assert_eq!(turn, vec![1]);
    })
}

#[test]
fn test_fail_player2_tries_to_place_another_stone() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players.clone(),
            timeout: 2,
            min_stone_offchain: 5,
            max_stone_onchain: 5,
        };

        assert_ok!(SingleGomoku::app_initiate(
            Origin::signed(players[0]),
            initiate_request.clone())
        );

        let app_id = SingleGomoku::get_app_id(initiate_request.nonce, initiate_request.players.clone());
        
        // place stone
        place_stone(app_id, players.clone(), players_pair);
    
        let settle_finalized_time = SingleGomoku::get_settle_finalized_time(app_id).unwrap();
        System::set_block_number(settle_finalized_time + 1);
        assert_ok!(
            SingleGomoku::update_by_action(
                Origin::signed(players[1]),
                app_id,
                vec![3, 12]
            )
        );

        assert_noop!(
            SingleGomoku::update_by_action(
                Origin::signed(players[1]),
                app_id,
                vec![4, 12]
            ),
            "not your turn"
        );
    })
}

#[test]
fn test_fail_player1_place_a_stone_at_occupied_slot_3_12() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players.clone(),
            timeout: 2,
            min_stone_offchain: 5,
            max_stone_onchain: 5,
        };

        assert_ok!(SingleGomoku::app_initiate(
            Origin::signed(players[0]),
            initiate_request.clone())
        );

        let app_id = SingleGomoku::get_app_id(initiate_request.nonce, initiate_request.players.clone());
        
        // place stone
        place_stone(app_id, players.clone(), players_pair);
    
        let settle_finalized_time = SingleGomoku::get_settle_finalized_time(app_id).unwrap();
        System::set_block_number(settle_finalized_time + 1);
        assert_ok!(
            SingleGomoku::update_by_action(
                Origin::signed(players[1]),
                app_id,
                vec![3, 12]
            )
        );

        assert_noop!(
            SingleGomoku::update_by_action(
                Origin::signed(players[0]),
                app_id,
                vec![3, 12]
            ),
            "slot is occupied"
        );
    })
}

#[test]
fn test_player1_places_a_stone_at_0_4_and_wins() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players.clone(),
            timeout: 2,
            min_stone_offchain: 5,
            max_stone_onchain: 5,
        };

        assert_ok!(SingleGomoku::app_initiate(
            Origin::signed(players[0]),
            initiate_request.clone())
        );

        let app_id = SingleGomoku::get_app_id(initiate_request.nonce, initiate_request.players.clone());
        
        // place stone
        place_stone(app_id, players.clone(), players_pair);
    
        let settle_finalized_time = SingleGomoku::get_settle_finalized_time(app_id).unwrap();
        System::set_block_number(settle_finalized_time + 1);
        assert_ok!(
            SingleGomoku::update_by_action(
                Origin::signed(players[1]),
                app_id,
                vec![3, 12]
            )
        );

        assert_ok!(
            SingleGomoku::update_by_action(
                Origin::signed(players[0]),
                app_id,
                vec![0, 4]
            )
        );
        let turn = SingleGomoku::get_state(app_id, 0).unwrap();
        assert_eq!(turn, vec![0]);
        assert_ok!(
            SingleGomoku::is_finalized(
                Origin::signed(players[0]),
                app_id
            )
        );
        assert_ok!(
            SingleGomoku::get_outcome(
                Origin::signed(players[0]),
                app_id,
                1
            )
        );
    })
}

#[test]
fn test_fail_finalize_on_action_timeout_before_action_deadline() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players.clone(),
            timeout: 2,
            min_stone_offchain: 5,
            max_stone_onchain: 5,
        };

        assert_ok!(SingleGomoku::app_initiate(
            Origin::signed(players[0]),
            initiate_request.clone())
        );

        let app_id = SingleGomoku::get_app_id(initiate_request.nonce, initiate_request.players.clone());
        
        let mut board_state = vec![0; 227];
        board_state[0] = 0; // winner
        board_state[1] = 2; // turn
        board_state[2] = 1; // (0, 0)
        board_state[3] = 1; // (0, 1)
        board_state[4] = 1; // (0, 2)
        board_state[5] = 1; // (0, 3)
        board_state[101] = 2;
        board_state[102] = 2;
        board_state[103] = 2;
        let state_proof = get_state_proof(0, 3, board_state, 0, app_id, players_pair);
        assert_ok!(
            SingleGomoku::update_by_state(
                Origin::signed(players[0]),
                state_proof
            )
        );

        assert_noop!(
            SingleGomoku::finalize_on_action_timeout(
                Origin::signed(players[0]),
                app_id
            ),
            "while settling"
        );
    })
}

#[test]
fn test_pass_finalize_on_action_timeout_after_action_deadline() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players.clone(),
            timeout: 2,
            min_stone_offchain: 5,
            max_stone_onchain: 5,
        };

        assert_ok!(SingleGomoku::app_initiate(
            Origin::signed(players[0]),
            initiate_request.clone())
        );

        let app_id = SingleGomoku::get_app_id(initiate_request.nonce, initiate_request.players.clone());
        
        let mut board_state = vec![0; 227];
        board_state[0] = 0; // winner
        board_state[1] = 2; // turn
        board_state[2] = 1; // (0, 0)
        board_state[3] = 1; // (0, 1)
        board_state[4] = 1; // (0, 2)
        board_state[5] = 1; // (0, 3)
        board_state[101] = 2;
        board_state[102] = 2;
        board_state[103] = 2;
        let state_proof = get_state_proof(0, 3, board_state, 0, app_id, players_pair);
        assert_ok!(
            SingleGomoku::update_by_state(
                Origin::signed(players[0]),
                state_proof
            )
        );

        let deadline = SingleGomoku::get_action_deadline(app_id).unwrap();
        System::set_block_number(deadline + 1);
        assert_ok!(
            SingleGomoku::finalize_on_action_timeout(
                Origin::signed(players[0]),
                app_id
            )
        );
        assert_ok!(
            SingleGomoku::is_finalized(
                Origin::signed(players[0]),
                app_id
            )
        );
        assert_ok!(
            SingleGomoku::get_outcome(
                Origin::signed(players[0]),
                app_id,
                1
            )
        );
    })
}

fn get_state_proof(
    nonce: u128,
    seq: u128,
    board_state: Vec<u8>,
    timeout: BlockNumber,
    app_id: H256,
    players_pair: Vec<sr25519::Pair>,
) -> StateProof<BlockNumber, H256, Signature> {
    let app_state = AppState {
        nonce: nonce,
        seq_num: seq,
        board_state: board_state,
        timeout: timeout,
        app_id: app_id,
    };
    let encoded = SingleGomoku::encode_app_state(app_state.clone());
    let sig_1 = players_pair[0].sign(&encoded);
    let sig_2 = players_pair[1].sign(&encoded);
    let state_proof = StateProof {
        app_state: app_state,
        sigs: vec![sig_1, sig_2]
    };

    return state_proof;
}

fn place_stone(app_id: H256, players: Vec<AccountId>, players_pair: Vec<sr25519::Pair>) {
    let mut board_state_1 = vec![0; 227];
    board_state_1[0] = 0;
    board_state_1[1] = 1;
    board_state_1[2] = 2;
    board_state_1[3] = 2;
    board_state_1[4] = 1;
    board_state_1[5] = 1;
    board_state_1[6] = 2;
    board_state_1[7] = 2;
    board_state_1[8] = 1;
    let state_proof = get_state_proof(0, 1, board_state_1.clone(), 0, app_id, players_pair.clone());
    assert_ok!(
        SingleGomoku::update_by_state(
            Origin::signed(players[0]),
            state_proof
        )
    );

    let mut board_state_2 = vec![0; 227];
    board_state_2[0] = 0; // winner
    board_state_2[1] = 2; // turn
    board_state_2[2] = 1; // (0, 0)
    board_state_2[3] = 1; // (0, 1)
    board_state_2[4] = 1; // (0, 2)
    board_state_2[5] = 1; // (0, 3)
    board_state_2[101] = 2; 
    board_state_2[102] = 2;
    board_state_2[103] = 2;
    let state_proof = get_state_proof(0, 2, board_state_2, 0, app_id, players_pair);
    assert_ok!(
        SingleGomoku::update_by_state(
            Origin::signed(players[0]),
            state_proof
        )
    );

}
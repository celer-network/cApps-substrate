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
            player_num: 2,
            timeout: 2,
            min_stone_offchain: 5,
            max_stone_onchain: 5,
        };

        assert_ok!(MultiGomoku::app_initiate(
            Origin::signed(players[0]),
            initiate_request)
        );
    })
}

#[test]
fn test_pass_player1_submits_state_proof() {
    ExtBuilder::build().execute_with(|| {
        let none: u8 = 0;
        let black: u8 = 1;
        let white: u8 = 2;
        let black_player_id1 = 2;
        let nonce1 = 1;

        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let session_id = app_initiate(nonce1, players.clone(), 2, 2, 5, 5);
        
        let mut board_state = vec![0; 228];
        board_state[0] = none; 
        board_state[1] = black; // turn color
        board_state[2] = black_player_id1;
        board_state[3] = white;
        board_state[4] = white;
        board_state[5] = black;
        board_state[6] = black;
        board_state[7] = white;
        board_state[8] = white;
        board_state[9] = black;

        let state_proof = get_state_proof(3, board_state, 2, session_id, players_pair);
        assert_ok!(
            MultiGomoku::update_by_state(
                Origin::signed(players[0]),
                state_proof
            )
        );
        let onchain_state = MultiGomoku::get_state(session_id, 2).unwrap();
        assert_eq!(onchain_state[0], none);
        assert_eq!(onchain_state[1], black);
        assert_eq!(onchain_state[3], white);
        assert_eq!(onchain_state[4], white);
        assert_eq!(onchain_state[5], black);
        assert_eq!(onchain_state[6], black);
        assert_eq!(onchain_state[7], white);
        assert_eq!(onchain_state[8], white);
        assert_eq!(onchain_state[9], black);
    })
}

#[test]
fn test_fail_update_by_state_with_invalid_seq() {
    ExtBuilder::build().execute_with(|| {
        let nonce1 = 2;
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let session_id = app_initiate(nonce1, players.clone(), 2, 2, 5, 5);
        
        place_stone_and_update_by_state(session_id, players.clone(), players_pair.clone());

        let board_state = vec![0; 228];
        let state_proof = get_state_proof(0, board_state, 2, session_id, players_pair);
        assert_noop!(
            MultiGomoku::update_by_state(
                Origin::signed(players[0]),
                state_proof
            ),
            "invalid sequence number"
        );
    })
}

#[test]
fn test_pass_intend_settle_with_higher_seq() {
    ExtBuilder::build().execute_with(|| {
        let nonce1 = 2;
        let black_player_id1 = 2;
        let none: u8 = 0;
        let black: u8 = 1;
        let white: u8 = 2;
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let session_id = app_initiate(nonce1, players.clone(), 2, 2, 5, 5);
        
        place_stone_and_update_by_state(session_id, players.clone(), players_pair.clone());

        let mut board_state = vec![0; 228];
        board_state[0] = none; // winner
        board_state[1] = white; // turn color
        board_state[2] = black_player_id1;
        board_state[3] = black; // (0, 0)
        board_state[4] = black; // (0, 1)
        board_state[5] = black; // (0, 2)
        board_state[6] = black; // (0, 3)
        board_state[101] = white;
        board_state[102] = white;
        board_state[103] = white;
        let state_proof = get_state_proof(4, board_state, 2, session_id, players_pair);
        assert_ok!(
            MultiGomoku::update_by_state(
                Origin::signed(players[0]),
                state_proof
            )
        );
        let onchain_state = MultiGomoku::get_state(session_id, 2).unwrap();
        assert_eq!(onchain_state[0], none);
        assert_eq!(onchain_state[1], white);
        assert_eq!(onchain_state[3], black);
        assert_eq!(onchain_state[4], black);
        assert_eq!(onchain_state[5], black);
        assert_eq!(onchain_state[6], black);
        assert_eq!(onchain_state[7], none);
        assert_eq!(onchain_state[8], none);
        assert_eq!(onchain_state[9], none);
        assert_eq!(onchain_state[101], white);
        assert_eq!(onchain_state[102], white);
        assert_eq!(onchain_state[103], white);
    })
}

#[test]
fn test_fail_player1_places_stone_at_3_12_before_settle_finalized_time() {
    ExtBuilder::build().execute_with(|| {
        let nonce1 = 2;
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let session_id = app_initiate(nonce1, players.clone(), 2, 2, 5, 5);
        
        place_stone_and_update_by_state_two_times(session_id, players.clone(), players_pair);

        assert_noop!(
            MultiGomoku::update_by_action(
                Origin::signed(players[0]),
                session_id,
                vec![3, 12]
            ),
            "app not in action mode"
        );
    })
}

#[test]
fn test_pass_player1_places_stone_at_3_12_after_settle_finalized_time() {
    ExtBuilder::build().execute_with(|| {
        let nonce1 = 2;
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let session_id = app_initiate(nonce1, players.clone(), 2, 2, 5, 5);
        
        place_stone_and_update_by_state_two_times(session_id, players.clone(), players_pair);

        let settle_finalized_time = MultiGomoku::get_settle_finalized_time(session_id).unwrap();
        System::set_block_number(settle_finalized_time + 1);

        assert_ok!(
            MultiGomoku::update_by_action(
                Origin::signed(players[0]),
                session_id,
                vec![3, 12]
            )
        );

        let turn = MultiGomoku::get_state(session_id, 0).unwrap();
        assert_eq!(turn, vec![1]);
    })
}

#[test]
fn test_fail_player1_tries_to_place_another_stone() {
    ExtBuilder::build().execute_with(|| {
        let nonce1 = 2;
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let session_id = app_initiate(nonce1, players.clone(), 2, 2, 5, 5);
        
        place_stone_and_update_by_state_two_times(session_id, players.clone(), players_pair);

        let settle_finalized_time = MultiGomoku::get_settle_finalized_time(session_id).unwrap();
        System::set_block_number(settle_finalized_time + 1);

        assert_ok!(
            MultiGomoku::update_by_action(
                Origin::signed(players[0]),
                session_id,
                vec![3, 12]
            )
        );

        assert_noop!(
            MultiGomoku::update_by_action(
                Origin::signed(players[0]),
                session_id,
                vec![4, 12]
            ),
            "Not your turn"
        );
    })
}

#[test]
fn test_fail_player2_tries_to_place_stone_at_occupied_slot_3_12() {
    ExtBuilder::build().execute_with(|| {
        let nonce1 = 2;
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let session_id = app_initiate(nonce1, players.clone(), 2, 2, 5, 5);
        
        place_stone_and_update_by_state_two_times(session_id, players.clone(), players_pair);

        let settle_finalized_time = MultiGomoku::get_settle_finalized_time(session_id).unwrap();
        System::set_block_number(settle_finalized_time + 1);

        assert_ok!(
            MultiGomoku::update_by_action(
                Origin::signed(players[0]),
                session_id,
                vec![3, 12]
            )
        );

        assert_noop!(
            MultiGomoku::update_by_action(
                Origin::signed(players[1]),
                session_id,
                vec![3, 12]
            ),
            "slot is occupied"
        );
    })
}

#[test]
fn test_player2_places_stone_at_0_4_and_wins() {
    ExtBuilder::build().execute_with(|| {
        let nonce = 2;
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let session_id = app_initiate(nonce, players.clone(), 2, 2, 5, 5);
        
        place_stone_and_update_by_state_two_times(session_id, players.clone(), players_pair);

        let settle_finalized_time = MultiGomoku::get_settle_finalized_time(session_id).unwrap();
        System::set_block_number(settle_finalized_time + 1);

        assert_ok!(
            MultiGomoku::update_by_action(
                Origin::signed(players[0]),
                session_id,
                vec![3, 12]
            )
        );

        assert_ok!(
            MultiGomoku::update_by_action(
                Origin::signed(players[1]),
                session_id,
                vec![0, 4]
            )
        ); 
        let turn = MultiGomoku::get_state(session_id, 0).unwrap();
        assert_eq!(turn, vec![0]);
        assert_ok!(
            MultiGomoku::is_finalized(
                Origin::signed(players[0]),
                session_id
            )
        );
        assert_ok!(
            MultiGomoku::get_outcome(
                Origin::signed(players[0]),
                session_id,
                1
            )
        );
    })
}

#[test]
fn test_fail_not_player_places_stone() {
    ExtBuilder::build().execute_with(|| {
        let nonce = 2;
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let session_id = app_initiate(nonce, players.clone(), 2, 2, 5, 5);
        
        place_stone_and_update_by_state_two_times(session_id, players.clone(), players_pair);

        let settle_finalized_time = MultiGomoku::get_settle_finalized_time(session_id).unwrap();
        System::set_block_number(settle_finalized_time + 1);

        let risa = account_key("Risa"); // not app player
        assert_noop!(
            MultiGomoku::update_by_action(
                Origin::signed(risa),
                session_id,
                vec![3, 12]
            ),
            "Not your turn"
        );
    })
}

#[test]
fn test_fail_finalize_on_action_timeout_before_action_deadline() {
    ExtBuilder::build().execute_with(|| {
        let nonce = 2;
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let session_id = app_initiate(nonce, players.clone(), 2, 2, 5, 5);
        
        place_stone_and_update_by_state_two_times(session_id, players.clone(), players_pair);

        let settle_finalized_time = MultiGomoku::get_settle_finalized_time(session_id).unwrap();
        System::set_block_number(settle_finalized_time + 1);

        assert_ok!(
            MultiGomoku::update_by_action(
                Origin::signed(players[0]),
                session_id,
                vec![3, 12]
            )
        );

        assert_noop!(
            MultiGomoku::finalize_on_action_timeout(
                Origin::signed(players[0]),
                session_id
            ),
            "deadline does not passes"
        );
    })
}

#[test]
fn test_pass_finalize_on_action_timeout_after_action_deadline() {
    ExtBuilder::build().execute_with(|| {
        let nonce = 2;
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players, players_pair) 
            = get_sorted_peer(alice_pair, bob_pair);
        
        let session_id = app_initiate(nonce, players.clone(), 2, 2, 5, 5);
        
        let none: u8 = 0;
        let black: u8 = 1;
        let white: u8 = 2;
        let black_player_id2 = 1;

        let mut board_state = vec![0; 228];
        board_state[0] = none; 
        board_state[1] = white; // turn color
        board_state[2] = black_player_id2;
        board_state[3] = black; // (0, 0)
        board_state[4] = black; // (0, 1)
        board_state[5] = black; // (0, 2)
        board_state[6] = black; // (0, 3)
        board_state[101] = white;
        board_state[102] = white;
        board_state[103] = white;

        let state_proof = get_state_proof(3, board_state, 2, session_id, players_pair);
        assert_ok!(
            MultiGomoku::update_by_state(
                Origin::signed(players[0]),
                state_proof
            )
        );

        let settle_finalized_time = MultiGomoku::get_settle_finalized_time(session_id).unwrap();
        System::set_block_number(settle_finalized_time + 1);

        assert_ok!(
            MultiGomoku::update_by_action(
                Origin::signed(players[1]),
                session_id,
                vec![3, 12]
            )
        );

        let deadline = MultiGomoku::get_action_deadline(session_id).unwrap();
        System::set_block_number(deadline + 1);

        assert_ok!(
            MultiGomoku::finalize_on_action_timeout(
                Origin::signed(players[0]),
                session_id
            )
        );
        assert_ok!(
            MultiGomoku::is_finalized(
                Origin::signed(players[0]),
                session_id
            )
        );
        assert_ok!(
            MultiGomoku::get_outcome(
                Origin::signed(players[0]),
                session_id,
                2
            )
        );
    })
}

fn app_initiate(
    nonce: u128,
    players: Vec<AccountId>,
    player_num: u8,
    timeout: BlockNumber,
    min_stone_offchain: u8,
    max_stone_onchain: u8
) -> H256 {
    let initiate_request = AppInitiateRequest {
        nonce: nonce,
        players: players.clone(),
        player_num: player_num,
        timeout: timeout,
        min_stone_offchain: min_stone_offchain,
        max_stone_onchain: max_stone_onchain,
    };

    assert_ok!(MultiGomoku::app_initiate(
        Origin::signed(players[0]),
        initiate_request.clone())
    );

    let session_id = MultiGomoku::get_session_id(initiate_request.nonce, initiate_request.players);
    return session_id;
}

fn get_state_proof(
    seq: u128,
    board_state: Vec<u8>,
    timeout: BlockNumber,
    session_id: H256,
    players_pair: Vec<sr25519::Pair>,
) -> StateProof<BlockNumber, H256, Signature> {
    let app_state = AppState {
        seq_num: seq,
        board_state: board_state,
        timeout: timeout,
        session_id: session_id,
    };
    let encoded = MultiGomoku::encode_app_state(app_state.clone());
    let sig_1 = players_pair[0].sign(&encoded);
    let sig_2 = players_pair[1].sign(&encoded);
    let state_proof = StateProof {
        app_state:  app_state,
        sigs: vec![sig_1, sig_2]
    };

    return state_proof;
}

fn place_stone_and_update_by_state(
    session_id: H256, 
    players: Vec<AccountId>, 
    players_pair: Vec<sr25519::Pair>
) {
    let none: u8 = 0;
    let black: u8 = 1;
    let white: u8 = 2;
    let black_player_id1 = 2;

    let mut board_state = vec![0; 228];
    board_state[0] = none; 
    board_state[1] = black; // turn color
    board_state[2] = black_player_id1;
    board_state[3] = white;
    board_state[4] = white;
    board_state[5] = black;
    board_state[6] = black;
    board_state[7] = white;
    board_state[8] = white;
    board_state[9] = black;

    let state_proof = get_state_proof(3, board_state, 2, session_id, players_pair);
    assert_ok!(
        MultiGomoku::update_by_state(
            Origin::signed(players[0]),
            state_proof
        )
    );
}

fn place_stone_and_update_by_state_two_times(
    session_id: H256,
    players: Vec<AccountId>,
    players_pair: Vec<sr25519::Pair>
) {
    let none: u8 = 0;
    let black: u8 = 1;
    let white: u8 = 2;
    let black_player_id1 = 2;

    place_stone_and_update_by_state(session_id, players.clone(), players_pair.clone());

    let mut board_state = vec![0; 228];
    board_state[0] = none; // winner
    board_state[1] = white; // turn color
    board_state[2] = black_player_id1;
    board_state[3] = black; // (0, 0)
    board_state[4] = black; // (0, 1)
    board_state[5] = black; // (0, 2)
    board_state[6] = black; // (0, 3)
    board_state[101] = white;
    board_state[102] = white;
    board_state[103] = white;
    let state_proof = get_state_proof(4, board_state, 2, session_id, players_pair);
    assert_ok!(
        MultiGomoku::update_by_state(
            Origin::signed(players[0]),
            state_proof
        )
    );
}
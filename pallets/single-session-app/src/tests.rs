use super::*;
use mock::*;
use sp_core::{sr25519, Pair, H256};
use frame_support::{assert_ok, assert_noop};

#[test]
fn test_pass_initiate() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");        
        let (players_peers, _) 
            = get_sorted_peer(alice_pair, bob_pair);

        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players_peers.clone(),
            timeout: 2,
        };
        
        assert_ok!(SingleSessionApp::app_initiate(
            Origin::signed(players_peers[0]),
            initiate_request)
        );
    })
}

#[test]
fn test_fail_update_by_action() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");
        let (players_peers, _)
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players_peers.clone(),
            timeout: 2,
        };
        
        assert_ok!(SingleSessionApp::app_initiate(
            Origin::signed(players_peers[0]),
            initiate_request.clone()
        ));

        let session_id = SingleSessionApp::get_session_id(initiate_request.nonce, initiate_request.players.clone());
        assert_noop!(
            SingleSessionApp::update_by_action(
            Origin::signed(players_peers[0]),
            session_id,
            1),
            "app not in action mode"
        );
    })
}

#[test]
fn test_pass_update_by_state_state_is_5() {
    ExtBuilder::build().execute_with(|| {
        System::set_block_number(1);
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");
        let (players_peers, players_pair)
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players_peers.clone(),
            timeout: 2,
        };
        assert_ok!(SingleSessionApp::app_initiate(
            Origin::signed(players_peers[0]),
            initiate_request.clone()
        ));

        let session_id = SingleSessionApp::get_session_id(initiate_request.nonce, initiate_request.players.clone());
        let state_proof = get_state_proof(0, 2, 5, 2, session_id, players_pair);
        assert_ok!(
            SingleSessionApp::update_by_state(
                Origin::signed(players_peers[0]),
                state_proof
            )
        );

        let expected_event = TestEvent::single_app(RawEvent::IntendSettle(session_id, 2));       
        assert!(System::events().iter().any(|a| a.event == expected_event)); 

        let app_info = SingleSessionApp::app_info(session_id).unwrap();
        let expected_app_info = AppInfo {
            state: 5,
            nonce: 0,
            players: players_peers.clone(),
            seq_num: 2,
            timeout: 2,
            deadline: 3,
            status: AppStatus::Settle,
        };
        assert_eq!(expected_app_info, app_info);

        assert_eq!(
            SingleSessionApp::is_finalized(session_id.encode()).unwrap(), 
            false    
        );

        let args_query_outcome = SingleSessionArgsQueryOutcome {
            session_id: session_id,
            query_data: 5
        };
        assert_eq!(
            SingleSessionApp::get_outcome(args_query_outcome.encode()).unwrap(),
            true.encode()
        );
    })
}

#[test]
fn test_fail_update_by_action_before_settle_finalized_time_should_fail() {
    ExtBuilder::build().execute_with(|| {
        System::set_block_number(1);
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");
        let (players_peers, players_pair)
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players_peers.clone(),
            timeout: 2,
        };
        assert_ok!(SingleSessionApp::app_initiate(
            Origin::signed(players_peers[0]),
            initiate_request.clone()
        ));

        let session_id = SingleSessionApp::get_session_id(initiate_request.nonce, initiate_request.players.clone());
        let state_proof = get_state_proof(0, 2, 5, 2, session_id, players_pair);
        assert_ok!(
            SingleSessionApp::update_by_state(
                Origin::signed(players_peers[0]),
                state_proof
            )
        );

        assert_noop!(
            SingleSessionApp::update_by_action(
                Origin::signed(players_peers[0]),
                session_id,
                1
            ),
            "app not in action mode"
        );
    })
}

#[test]
fn test_pass_update_by_action_after_settle_finalized_time() {
    ExtBuilder::build().execute_with(|| {
        System::set_block_number(1);
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");
        let (players_peers, players_pair)
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players_peers.clone(),
            timeout: 2,
        };
        assert_ok!(SingleSessionApp::app_initiate(
            Origin::signed(players_peers[0]),
            initiate_request.clone()
        ));

        let session_id = SingleSessionApp::get_session_id(initiate_request.nonce, initiate_request.players.clone());
        let state_proof = get_state_proof(0, 2, 5, 2, session_id, players_pair);
        assert_ok!(
            SingleSessionApp::update_by_state(
                Origin::signed(players_peers[0]),
                state_proof
            )
        );

        let settle_finalized_time = SingleSessionApp::get_settle_finalized_time(session_id).unwrap();
        System::set_block_number(settle_finalized_time + 1);
        assert_ok!(
            SingleSessionApp::update_by_action(
                Origin::signed(players_peers[0]),
                session_id,
                1
            )
        );

        let args_query_outcome = SingleSessionArgsQueryOutcome {
            session_id: session_id,
            query_data: 5
        };
        assert_eq!(
            SingleSessionApp::get_outcome(args_query_outcome.encode()).unwrap(),
            true.encode()
        );    
    })
}

#[test]
fn test_fail_update_by_state_with_invlaid_seq_num() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");
        let (players_peers, players_pair)
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players_peers.clone(),
            timeout: 2,
        };
        assert_ok!(SingleSessionApp::app_initiate(
            Origin::signed(players_peers[0]),
            initiate_request.clone()
        ));

        let session_id = SingleSessionApp::get_session_id(initiate_request.nonce, initiate_request.players.clone());
        let state_proof = get_state_proof(0, 0, 5, 2, session_id, players_pair);
        assert_noop!(
            SingleSessionApp::update_by_state(
                Origin::signed(players_peers[0]),
                state_proof
            ),
            "invalid sequence number"
        );
    })
}

#[test]
fn test_pass_update_by_state_state_is_2() {
    ExtBuilder::build().execute_with(|| {
        System::set_block_number(1);
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");
        let (players_peers, players_pair)
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players_peers.clone(),
            timeout: 2,
        };
        assert_ok!(SingleSessionApp::app_initiate(
            Origin::signed(players_peers[0]),
            initiate_request.clone()
        ));

        let session_id = SingleSessionApp::get_session_id(initiate_request.nonce, initiate_request.players.clone());
        let state_proof = get_state_proof(0, 2, 2, 2, session_id, players_pair);
        assert_ok!(
            SingleSessionApp::update_by_state(
                Origin::signed(players_peers[0]),
                state_proof
            )
        );
        let expected_event = TestEvent::single_app(RawEvent::IntendSettle(session_id, 2));       
        assert!(System::events().iter().any(|a| a.event == expected_event)); 

        assert_eq!(
            SingleSessionApp::is_finalized(session_id.encode()).unwrap(), 
            true
        );
        let args_query_outcome = SingleSessionArgsQueryOutcome {
            session_id: session_id,
            query_data: 2
        };
        assert_eq!(
            SingleSessionApp::get_outcome(args_query_outcome.encode()).unwrap(),
            true.encode()
        );
    })
}

#[test]
fn test_fail_update_by_action_after_finalized() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");
        let (players_peers, players_pair)
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players_peers.clone(),
            timeout: 2,
        };
        assert_ok!(SingleSessionApp::app_initiate(
            Origin::signed(players_peers[0]),
            initiate_request.clone()
        ));

        let session_id = SingleSessionApp::get_session_id(initiate_request.nonce, initiate_request.players.clone());
        let state_proof = get_state_proof(0, 2, 2, 2, session_id, players_pair);
        assert_ok!(
            SingleSessionApp::update_by_state(
                Origin::signed(players_peers[0]),
                state_proof
            )
        );

        assert_noop!(
            SingleSessionApp::update_by_action(
                Origin::signed(players_peers[0]),
                session_id,
                1
            ),
            "app state is finalized"
        );
    })
}

#[test]
fn test_fail_update_by_state_after_finalized() {
    ExtBuilder::build().execute_with(|| {
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");
        let (players_peers, players_pair)
            = get_sorted_peer(alice_pair, bob_pair);
        
        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players_peers.clone(),
            timeout: 2,
        };
        assert_ok!(SingleSessionApp::app_initiate(
            Origin::signed(players_peers[0]),
            initiate_request.clone()
        ));

        let session_id = SingleSessionApp::get_session_id(initiate_request.nonce, initiate_request.players.clone());
        let mut state_proof = get_state_proof(0, 2, 2, 2, session_id, players_pair.clone());
        assert_ok!(
            SingleSessionApp::update_by_state(
                Origin::signed(players_peers[0]),
                state_proof
            )
        );

        state_proof = get_state_proof(0, 3, 2, 2, session_id, players_pair);
        assert_noop!(
            SingleSessionApp::update_by_state(
                Origin::signed(players_peers[0]),
                state_proof
            ),
            "app state is finalized"
        );
    })
}

#[test]
fn test_pass_finalize_on_action_timeout() {
    ExtBuilder::build().execute_with(|| {
        System::set_block_number(1);
        let alice_pair = account_pair("Alice");
        let bob_pair = account_pair("Bob");
        let (players, players_pair) = get_sorted_peer(alice_pair.clone(), bob_pair.clone());

        let initiate_request = AppInitiateRequest {
            nonce: 0,
            players: players.clone(),
            timeout: 2
        };
        assert_ok!(
            SingleSessionApp::app_initiate(
                Origin::signed(players[0]),
                initiate_request.clone()
            )
        );

        let session_id = SingleSessionApp::get_session_id(initiate_request.nonce, initiate_request.players.clone());
        let state_proof = get_state_proof(0, 1, 2, 2, session_id, players_pair.clone());
        assert_ok!(
            SingleSessionApp::update_by_state(
                Origin::signed(players[0]),
                state_proof
            )
        );   

        System::set_block_number(5);
        assert_ok!(
            SingleSessionApp::finalize_on_action_timeout(
                Origin::signed(players[0]),
                session_id
            )
        );
    })
}


fn get_state_proof(
    nonce: u128, 
    seq: u128, 
    state: u8, 
    timeout: BlockNumber,
    session_id: H256,
    players_pair: Vec<sr25519::Pair>
) -> StateProof<BlockNumber, H256, Signature> {
    let app_state = AppState {
        nonce: nonce,
        seq_num: seq,
        state: state,
        timeout: timeout,
        session_id: session_id,
    };
    let encoded = SingleSessionApp::encode_app_state(app_state.clone());
    let sig_1 = players_pair[0].sign(&encoded);
    let sig_2 = players_pair[1].sign(&encoded);
    let state_proof = StateProof {
        app_state: app_state,
        sigs: vec![sig_1, sig_2]
    };

    return state_proof;
}


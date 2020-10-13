#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
#[cfg(test)]
mod tests;

use codec::{Decode, Encode};
use frame_support::{
    decl_module, decl_storage, decl_event, decl_error, ensure,
    storage::StorageMap,
    traits::Get,
};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{DispatchResult, DispatchError};
use sp_runtime::traits::{
    Hash, IdentifyAccount, AccountIdConversion, 
    Member, Verify, Zero,
};
use sp_runtime::{ModuleId, RuntimeDebug};
use sp_std::{prelude::*, vec::Vec};

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct SessionInitiateRequest<AccountId, BlockNumber> {
    nonce: u128,
    player_num: u8,
    players: Vec<AccountId>,
    timeout: BlockNumber,
}

pub type SessionInitiateRequestOf<T> = SessionInitiateRequest<
    <T as system::Trait>::AccountId,
    <T as system::Trait>::BlockNumber,
>;

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct AppState<BlockNumber, Hash> {
    seq_num: u128,
    state: u8,
    timeout: BlockNumber,
    session_id: Hash,
}

pub type AppStateOf<T> = AppState<
    <T as system::Trait>::BlockNumber,
    <T as system::Trait>::Hash,
>;

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct StateProof<BlockNumber, Hash, Signature> {
    app_state: AppState<BlockNumber, Hash>,
    sigs: Vec<Signature>,
}

pub type StateProofOf<T> = StateProof<
    <T as system::Trait>::BlockNumber,
    <T as system::Trait>::Hash,
    <T as Trait>::Signature,
>;

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub enum SessionStatus {
    Idle = 0,
    Settle = 1,
    Action = 2,
    Finalized = 3,
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct SessionInfo<AccountId, BlockNumber> {
    state: u8,
    players: Vec<AccountId>,
    player_num: u8,
    seq_num: u128,
    timeout: BlockNumber,
    deadline: BlockNumber,
    status: SessionStatus,
}

pub type SessionInfoOf<T> = SessionInfo<
    <T as system::Trait>::AccountId,
    <T as system::Trait>::BlockNumber,
>;

pub const MULTI_SESSION_APP_ID: ModuleId = ModuleId(*b"_multi__");

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
    type Public: IdentifyAccount<AccountId = Self::AccountId>;
    type Signature: Verify<Signer = <Self as Trait>::Public> + Member + Decode + Encode; 
}

decl_storage! {
    trait Store for Module<T: Trait> as MultiSessionApp {
        pub SessionInfoMap get(fn session_info):
            map hasher(blake2_128_concat) T::Hash => Option<SessionInfoOf<T>>;
    }
}

decl_module!  {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Initiate multi session app
        ///
        /// Parameters:
        /// - `initiate_request`: Session initiate request message
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage insertion `SessionInfoMap`
        ///   - 1 storage reads `SessionInfoMap`
        /// - Based on benchmark;
        ///     19.78　µs
        /// # </weight>
        #[weight = 20_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn session_initiate(
            origin,
            initiate_request: SessionInitiateRequestOf<T>
        ) -> DispatchResult {
            let session_id = Self::get_session_id(initiate_request.nonce, initiate_request.players.clone());
            ensure!(
                SessionInfoMap::<T>::contains_key(&session_id) == false,
                "session_id is used"
            );
            
            // check whether account is asscending order
            Self::is_ordered_account(initiate_request.players.clone())?;

            let session_info = SessionInfoOf::<T> {
                state: 0,
                players: initiate_request.players,
                player_num: initiate_request.player_num,
                seq_num: 0,
                timeout: initiate_request.timeout,
                deadline: Zero::zero(),
                status: SessionStatus::Idle,
            };
            SessionInfoMap::<T>::insert(session_id, session_info);
        
            Ok(())
        }

        /// Update state according to an off-chain state proof
        ///
        /// Parameters:
        /// - `state_proof`: Signed off-chain session state
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity `O(1)`
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage mutation `SessionInfoMap`
        ///   - 1 storage read `SessionInfoMap`
        /// - Based on benchmark;
        ///     48.44　µs
        /// # </weight>
        #[weight = 49_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn update_by_state(
            origin,
            state_proof: StateProofOf<T>
        ) -> DispatchResult {
            ensure_signed(origin)?; 

            let session_id = state_proof.app_state.session_id;
            let session_info = match SessionInfoMap::<T>::get(session_id) {
                Some(session) => session,
                None => Err(Error::<T>::SessionInfoNotExist)?,
            };
            
            // submit and settle off-chain state
            let mut new_session_info = Self::intend_settle(session_info, state_proof.clone())?;
            
            let state = state_proof.app_state.state;
            if state == 1 || state == 2 {
                new_session_info.state = state;
                new_session_info.status = SessionStatus::Finalized;
            } else {
                new_session_info.state = state;
            }
            
            SessionInfoMap::<T>::mutate(&session_id, |session_info| *session_info = Some(new_session_info.clone()));

            // emit IntendSettle event
            Self::deposit_event(Event::<T>::IntendSettle(session_id, new_session_info.seq_num));

            Ok(())
        }
        

        /// Update state according to an on-chain action
        ///
        /// Parameters:
        /// - `session_id`: Id of session
        /// - `action`: Action data
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage mutation `SessionInfoMap`
        ///   - 1 storage read `SessionInfoMap`
        /// - Based on benchmark;
        ///     24.89　µs
        /// # </weight>
        #[weight = 25_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn update_by_action(
            origin,
            session_id: T::Hash,
            action: u8
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let session_info = match SessionInfoMap::<T>::get(session_id) {
                Some(session) => session,
                None => Err(Error::<T>::SessionInfoNotExist)?,
            };

            let mut new_session_info = Self::apply_action(session_info)?;
        
            if action == 1 || action == 2 {
                new_session_info.status = SessionStatus::Finalized;
            } 
            SessionInfoMap::<T>::mutate(&session_id, |session_info| *session_info = Some(new_session_info));

            Ok(())
        }

        /// Finalize in case of on-chain action timeout
        ///
        /// Parameters:
        /// - `session_id`: Id of session
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage mutation `SessionInfoMap`
        ///   - 1 storage read `SessionInfoMap`
        /// - Based on benchmark;
        ///     16.35　µs
        /// # </weight>
        #[weight = 17_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn finalize_on_action_timeout(
            origin,
            session_id: T::Hash
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let session_info = match SessionInfoMap::<T>::get(session_id) {
                Some(session) => session,
                None => Err(Error::<T>::SessionInfoNotExist)?,
            };
            
            let block_number = frame_system::Module::<T>::block_number();
            if session_info.status == SessionStatus::Action {
                ensure!(
                    block_number >  session_info.deadline,
                    "deadline does not passes"
                );
            } else if session_info.status == SessionStatus::Settle {
                ensure!(
                    block_number > session_info.deadline + session_info.timeout,
                    "while setting"
                );
            } else {
                return Ok(());
            }

            let new_session_info = SessionInfoOf::<T> {
                state: session_info.state,
                players: session_info.players,
                player_num: session_info.player_num,
                seq_num: session_info.seq_num,
                timeout: session_info.timeout,
                deadline: session_info.deadline,
                status: SessionStatus::Finalized,
            };
            SessionInfoMap::<T>::mutate(&session_id, |session_info| *session_info = Some(new_session_info));

            Ok(())
        }

        /// Check whether session is finalized
        ///
        /// Parameters:
        /// - `session_id`: Id of session
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage read `SessionInfoMap`
        /// - Based on benchmark;
        ///     9.118　µs
        /// # </weight>
        #[weight = 10_000_000 + T::DbWeight::get().reads(1)]
        pub fn is_finalized(
            origin,
            session_id: T::Hash,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let session_info = match SessionInfoMap::<T>::get(session_id) {
                Some(session) => session,
                None => return Err(Error::<T>::SessionInfoNotExist)?,
            };

            // If session is not finlized, return DispatchError::Other("NotFinalized")
            ensure!(
                session_info.status == SessionStatus::Finalized,
                "NotFinalized"
            );

            // If session is finalized, return Ok(())
            Ok(())
        }

        /// Get the session outcome
        /// 
        /// Parameters:
        /// - `session_id`: Id of session
        /// - `query`: query param
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage read `SessionInfoMap`
        /// - Based on benchmark;
        ///     10.27　µs
        /// # </weight>
        #[weight = 11_000_000 + T::DbWeight::get().reads(1)]
        pub fn get_outcome(
            origin,
            session_id: T::Hash,
            query: u8
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let session_info = match SessionInfoMap::<T>::get(session_id) {
                Some(session) => session,
                None => Err(Error::<T>::SessionInfoNotExist)?,
            };

            // If outcome is false, return DispatchError::Other("FalseOutcome")
            ensure!(
                session_info.state == query,
                "FalseOutcome"
            );

            // If outcome is true, return Ok(())
            Ok(())
        }
    }
}

decl_event! (
    pub enum Event<T> where
        <T as system::Trait>::Hash
    {
        /// IntendSettle(session_id, seq_num)
        IntendSettle(Hash, u128),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        // SessionInfo is not exist
        SessionInfoNotExist,
    }
}

impl<T: Trait> Module<T> {
    /// Get Id of session
    ///
    /// Parameters:
    /// `nonce`: Nonce of session
    /// `players`: AccountId of players
    pub fn get_session_id(
        nonce: u128,
        players: Vec<T::AccountId>,
    ) -> T::Hash {
        let multi_session_app_account = Self::app_account();
        let mut encoded = multi_session_app_account.encode();
        encoded.extend(nonce.encode());
        players.into_iter()
            .for_each(|players| { encoded.extend(players.encode()); });
        let session_id = T::Hashing::hash(&encoded);
        return session_id;
    }

    /// Get session state
    ///
    /// Parameter:
    /// `session_id`: Id of session
    pub fn get_state(session_id: T::Hash) -> Option<u8> {
        let session_info = match SessionInfoMap::<T>::get(session_id) {
            Some(session) => session,
            None => return None,
        };

        return Some(session_info.state);
    }

    /// Get session status
    ///
    /// Parameter:
    /// `session_id`: Id of session
    pub fn get_status(session_id: T::Hash) -> Option<SessionStatus> {
        let session_info = match SessionInfoMap::<T>::get(session_id) {
            Some(session) => session,
            None => return None,
        };

        return Some(session_info.status);
    }

    /// Get state settle finalized time
    ///
    /// Parameter:
    /// `session_id`: Id of session
    pub fn get_settle_finalized_time(session_id: T::Hash) -> Option<T::BlockNumber> {
        let session_info = match SessionInfoMap::<T>::get(session_id) {
            Some(session) => session,
            None => return None,
        };

        if session_info.status == SessionStatus::Settle {
            return Some(session_info.deadline);
        }

        return None;
    }

    /// Get action deadline
    ///
    /// Parameter:
    /// `session_id`: Id of session
    pub fn get_action_deadline(session_id: T::Hash) -> Option<T::BlockNumber> {
        let session_info = match SessionInfoMap::<T>::get(session_id) {
            Some(session) => session,
            None => return None,
        };
        if session_info.status == SessionStatus::Action {
            return Some(session_info.deadline);
        } else if session_info.status == SessionStatus::Settle {
            return Some(session_info.deadline + session_info.timeout);
        } else {
            return None;
        }
    }

    /// Get session sequence number
    ///
    /// Parameter:
    /// `session_id`: Id of session
    pub fn get_seq_num(session_id: T::Hash) -> Option<u128> {
        let session_info = match SessionInfoMap::<T>::get(session_id) {
            Some(session) => session,
            None => return None,
        };     
        return Some(session_info.seq_num);
    }


    /// Get multi session app account id
    pub fn app_account() -> T::AccountId {
        MULTI_SESSION_APP_ID.into_account()
    }

    /// Submit and settle offchain state
    ///
    /// Parameter:
    /// `session_info`: Info of session state
    /// `state_proof`: Signed off-chain app state
    fn intend_settle(
        mut session_info: SessionInfoOf<T>,
        state_proof: StateProofOf<T>
    ) -> Result<SessionInfoOf<T>, DispatchError> {
        let app_state = state_proof.app_state;
        ensure!(
            state_proof.sigs.len() as u8 == session_info.player_num,
            "invalid number of players"
        );
        let encoded = Self::encode_app_state(app_state.clone());
        Self::valid_signers(state_proof.sigs, &encoded, session_info.players.clone())?;
        ensure!(
            session_info.status != SessionStatus::Finalized,
            "app state is finalized"
        );
    
        ensure!(
            session_info.seq_num < app_state.seq_num,
            "invalid sequence number"
        );

        session_info.seq_num = app_state.seq_num;
        session_info.deadline = frame_system::Module::<T>::block_number() + session_info.timeout;
        session_info.status = SessionStatus::Settle;

        Ok(session_info)
    }

    /// Apply an action to the on-chain state
    ///
    /// Parameter:
    /// `session_info`: Info of session state
    fn apply_action(
        mut session_info: SessionInfoOf<T>,
    ) -> Result<SessionInfoOf<T>, DispatchError> {
        ensure!(
            session_info.status != SessionStatus::Finalized,
            "app state is finalized"
        );

        let block_number = frame_system::Module::<T>::block_number();
        if session_info.status == SessionStatus::Settle && block_number > session_info.deadline {
            session_info.seq_num = session_info.seq_num + 1;
            session_info.deadline = block_number + session_info.timeout;
            session_info.status = SessionStatus::Action;
        } else {
            ensure!(
                session_info.status ==  SessionStatus::Action,
                "app not in action mode"
            );
            session_info.seq_num = session_info.seq_num + 1;
            session_info.deadline = block_number + session_info.timeout;
            session_info.status = SessionStatus::Action;
        }

        Ok(session_info)
    }

    /// Verify off-chain state signatures
    ///
    /// Parameters:
    /// `signatures`: Signaturs from the players
    /// `encoded`: Encoded app state
    /// `signers`: AccountId of player
    fn valid_signers(
        signatures: Vec<<T as Trait>::Signature>,
        encoded: &[u8],
        signers: Vec<T::AccountId>,
    ) -> Result<(), DispatchError> {
        for i in 0..signers.len() {
            let signature = &signatures[i];
            ensure!(
                signature.verify(encoded, &signers[i]),
                "Check co-sigs failed"
            );
        }

        Ok(())
    }

    /// Check whether account is asscending order
    ///
    /// Parameter:
    /// `palyers`: AccountId of players
    fn is_ordered_account(
        players: Vec<T::AccountId>
    ) -> Result<(), DispatchError> {
        let mut prev = &players[0];
        for i in 1..players.len() {
            ensure!(
                prev < &players[1],
                "player is not ascending order"
            );
            prev = &players[i];
        }

        Ok(())
    }

    /// Encode app state
    ///
    /// Parameter:
    /// `app_state`: app state
    fn encode_app_state(
        app_state: AppStateOf<T>
    ) -> Vec<u8> {
        let mut encoded = app_state.seq_num.encode();
        encoded.extend(app_state.state.encode());
        encoded.extend(app_state.timeout.encode());
        encoded.extend(app_state.session_id.encode());

        return encoded;
    }
}

   
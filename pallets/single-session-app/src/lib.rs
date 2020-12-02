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
use sp_runtime::traits::{
    Hash, IdentifyAccount, 
    Member, Verify, Zero, AccountIdConversion, 
};
use sp_runtime::{ModuleId, RuntimeDebug, DispatchResult, DispatchError};
use sp_std::{prelude::*, vec::Vec};

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct AppInitiateRequest<AccountId, BlockNumber> {
    nonce: u128,
    players: Vec<AccountId>,
    timeout: BlockNumber,
}

pub type AppInitiateRequestOf<T> = AppInitiateRequest<
    <T as system::Trait>::AccountId,
    <T as system::Trait>::BlockNumber,
>;

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct AppState<BlockNumber, Hash> {
    nonce: u128,
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
pub enum AppStatus {
    Idle = 0,
    Settle = 1,
    Action = 2,
    Finalized = 3,
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct AppInfo<AccountId, BlockNumber> {
    state: u8,
    nonce: u128,
    players: Vec<AccountId>,
    seq_num: u128,
    timeout: BlockNumber,
    deadline: BlockNumber,
    status: AppStatus,
}

pub type AppInfoOf<T> = AppInfo<
    <T as system::Trait>::AccountId,
    <T as system::Trait>::BlockNumber,
>;

#[derive(PartialEq, Eq, Encode, Decode, RuntimeDebug)]
pub struct SingleSessionArgsQueryOutcome<Hash> {
    pub session_id: Hash,
    pub query_data: u8
}

pub type SingleSessionArgsQueryOutcomeOf<T> = SingleSessionArgsQueryOutcome<<T as system::Trait>::Hash>;

pub const SINGLE_SESSION_APP_ID: ModuleId = ModuleId(*b"_single_");

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    type Public: IdentifyAccount<AccountId = Self::AccountId>;
    type Signature: Verify<Signer = <Self as Trait>::Public> + Member + Decode + Encode; 
}

decl_storage! {
    trait Store for Module<T: Trait> as SingleSessionApp {
        pub AppInfoMap get(fn app_info): 
            map hasher(blake2_128_concat) T::Hash => Option<AppInfoOf<T>>;
    }
}

decl_module!  {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Initiate single session app
        ///
        /// Parameters:
        /// - `initiate_request`: App initiate request message
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage insertion `AppInfoMap`
        ///   - 1 storage reads `AppInfoMap`
        /// - Based on benchmark;
        ///     18.44　µs
        /// # </weight>
        #[weight = 19_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn app_initiate(
            origin,
            initiate_request: AppInitiateRequestOf<T>
        ) -> DispatchResult {
            let session_id = Self::get_session_id(initiate_request.nonce, initiate_request.players.clone());
            ensure!(
                AppInfoMap::<T>::contains_key(&session_id) == false,
                "AppId alreads exists"
            );
            ensure!(
                initiate_request.players[0] < initiate_request.players[1], 
                "players is not asscending order"
            );

            let app_info = AppInfoOf::<T> {
                state: 0,
                nonce: initiate_request.nonce,
                players: initiate_request.players,
                seq_num: 0,
                timeout: initiate_request.timeout,
                deadline: Zero::zero(),
                status: AppStatus::Idle,
            };
            AppInfoMap::<T>::insert(session_id, app_info);
        
            Ok(())
        }

        /// Update state according to an off-chain state proof
        ///
        /// Parameters:
        /// - `state_proof`: Signed off-chain app state
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity `O(1)`
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage mutation `AppInfoMap`
        ///   - 1 storage read `AppInfoMap`
        /// - Based on benchmark;
        ///     44.68　µs
        /// # </weight>
        #[weight = 45_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn update_by_state(
            origin,
            state_proof: StateProofOf<T>
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let session_id = state_proof.app_state.session_id;
            let app_info = match AppInfoMap::<T>::get(session_id) {
                Some(app) => app,
                None => Err(Error::<T>::AppInfoNotExist)?,
            };

            // submit ad settle off-chain state
            let mut new_app_info: AppInfoOf<T> = Self::intend_settle(app_info, state_proof.clone())?;
            
            let state = state_proof.app_state.state;
            if state == 1 || state == 2 {
                new_app_info.state = state;
                new_app_info.status = AppStatus::Finalized;
            } else {
                new_app_info.state = state;
            }
            
            AppInfoMap::<T>::mutate(&session_id, |app_info| *app_info = Some(new_app_info.clone()));

            // Emit IntendSettle event
            Self::deposit_event(RawEvent::IntendSettle(session_id, new_app_info.seq_num));

            Ok(())
        }
        

        /// Update state according to an on-chain action
        ///
        /// Parameters:
        /// - `session_id`: Id of app
        /// - `action`: Action data
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage mutation `AppInfoMap`
        ///   - 1 storage read `AppInfoMap`
        /// - Based on benchmark;
        ///     23.92　µs
        /// # </weight>
        #[weight = 24_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn update_by_action(
            origin,
            session_id: T::Hash,
            action: u8
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let app_info = match AppInfoMap::<T>::get(session_id) {
                Some(app) => app,
                None => Err(Error::<T>::AppInfoNotExist)?,
            };

            // apply an action to the on-chain state
            let mut new_app_info: AppInfoOf<T> = Self::apply_action(app_info)?;
        
            if action == 1 || action == 2 {
                new_app_info.status = AppStatus::Finalized;
            } 
            AppInfoMap::<T>::mutate(&session_id, |app_info| *app_info = Some(new_app_info));

            Ok(())
        }

        /// Finalize in case of on-chain action timeout
        ///
        /// Parameters:
        /// - `session_id`: Id of app
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage mutation `AppInfoMap`
        ///   - 1 storage read `AppInfoMapp`
        /// - Based on benchmark;
        ///    21.59 　µs
        /// # </weight>
        #[weight = 22_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn finalize_on_action_timeout(
            origin,
            session_id: T::Hash
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let app_info = match AppInfoMap::<T>::get(session_id) {
                Some(app) => app,
                None => Err(Error::<T>::AppInfoNotExist)?,
            };
            
            let block_number = frame_system::Module::<T>::block_number();
            if app_info.status == AppStatus::Action {
                ensure!(
                    block_number >  app_info.deadline,
                    "deadline no passes"
                );
            } else if app_info.status == AppStatus::Settle {
                ensure!(
                    block_number > app_info.deadline + app_info.timeout,
                    "while setting"
                );
            } else {
                return Ok(());
            }

            let new_app_info = AppInfoOf::<T> {
                state: app_info.state,
                nonce: app_info.nonce,
                players: app_info.players,
                seq_num: app_info.seq_num,
                timeout: app_info.timeout,
                deadline: app_info.deadline,
                status: AppStatus::Finalized,
            };
            AppInfoMap::<T>::mutate(&session_id, |app_info| *app_info = Some(new_app_info));

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
        // AppInfo is not exist
        AppInfoNotExist,
        // A scale-codec encoded value can not decode correctly
        MustBeDecodable
    }
}

impl<T: Trait> Module<T> {   
    /// Query whether single session app is finalized
    ///
    /// Parameter:
    /// - `args_query_finalization`: encoded session_id
    ///
    /// Return the boolean value 
    pub fn is_finalized(
        args_query_finalization: Vec<u8>, 
    ) -> Result<bool, DispatchError> {
        let session_id: T::Hash = Decode::decode(&mut &args_query_finalization[..])
            .map_err(|_| Error::<T>::MustBeDecodable)?;
        let app_info = match AppInfoMap::<T>::get(session_id) {
            Some(app) => app,
            None => return Err(Error::<T>::AppInfoNotExist)?,
        };

        if app_info.status == AppStatus::Finalized {
            // Gomoku app is finalized
            return Ok(true);
        } else {
            // Gomoku app is not finalized
            return Ok(false);
        }   
    }

    /// Query the single session app outcome
    /// 
    /// Parameter:
    /// `args_query_outcome`: enoced SingleSessionArgsQueryOutcome
    ///
    /// Return the encoded boolean value
     pub fn get_outcome(
        args_query_outcome: Vec<u8>,
    ) -> Result<Vec<u8>, DispatchError> {
        let query_outcome: SingleSessionArgsQueryOutcomeOf<T> = SingleSessionArgsQueryOutcome::decode(&mut &args_query_outcome[..])
            .map_err(|_| Error::<T>::MustBeDecodable)?;
        let app_info = match AppInfoMap::<T>::get(query_outcome.session_id) {
            Some(app) => app,
            None => Err(Error::<T>::AppInfoNotExist)?,
        };

        if app_info.state == query_outcome.query_data {
            // If outcome is true, return encoded true value
            return Ok(true.encode());
        } else {
            // If outcome is false, return encoded false value
            return Ok(false.encode());
        }
    }

    /// Get Id of app
    ///
    /// Parameters:
    /// `nonce`: Nonce of app
    /// `players`: AccountId of players
    fn get_session_id(
        nonce: u128,
        players: Vec<T::AccountId>,
    ) -> T::Hash {
        let app_account = Self::app_account();
        let mut encoded = app_account.encode();
        encoded.extend(nonce.encode());
        encoded.extend(players[0].encode());
        encoded.extend(players[1].encode());
        let session_id = T::Hashing::hash(&encoded);
        return session_id;
    }

    /// Get app state
    ///
    /// Parameter:
    /// `session_id`: Id of app
    pub fn get_state(session_id: T::Hash) -> Option<u8> {
        let app_info = match AppInfoMap::<T>::get(session_id) {
            Some(app) => app,
            None => return None,
        };

        return Some(app_info.state);
    }

    /// Get app status
    ///
    /// Parameter:
    /// `session_id`: Id of app
    pub fn get_status(session_id: T::Hash) -> Option<AppStatus> {
        let app_info = match AppInfoMap::<T>::get(session_id) {
            Some(app) => app,
            None => return None,
        };

        return Some(app_info.status);
    }

    /// Get state settle finalized time
    ///
    /// Parameter:
    /// `session_id`: Id of app
    pub fn get_settle_finalized_time(session_id: T::Hash) -> Option<T::BlockNumber> {
        let app_info = match AppInfoMap::<T>::get(session_id) {
            Some(app) => app,
            None => return None,
        };

        if app_info.status == AppStatus::Settle {
            return Some(app_info.deadline);
        }

        return None;
    }

    /// Get action deadline
    ///
    /// Parameter:
    /// `session_id`: Id of app
    pub fn get_action_deadline(session_id: T::Hash) -> Option<T::BlockNumber> {
        let app_info = match AppInfoMap::<T>::get(session_id) {
            Some(app) => app,
            None => return None,
        };
        if app_info.status == AppStatus::Action {
            return Some(app_info.deadline);
        } else if app_info.status == AppStatus::Settle {
            return Some(app_info.deadline + app_info.timeout);
        } else {
            return None;
        }
    }

    /// Get app sequence number
    ///
    /// Parameter:
    /// `session_id`: Id of app
    pub fn get_seq_num(session_id: T::Hash) -> Option<u128> {
        let app_info = match AppInfoMap::<T>::get(session_id) {
            Some(app) => app,
            None => return None,
        };     
        return Some(app_info.seq_num);
    }

    /// Get single session app account id
    pub fn app_account() -> T::AccountId {
        SINGLE_SESSION_APP_ID.into_account()
    }

    /// Submit and settle offchain state
    ///
    /// Parameter:
    /// `app_info`: Info of app state
    /// `state_proof`: Signed off-chain app state
    fn intend_settle(
        mut app_info: AppInfoOf<T>,
        state_proof: StateProofOf<T>
    ) -> Result<AppInfoOf<T>, DispatchError> {
        let app_state = state_proof.app_state;
        let encoded = Self::encode_app_state(app_state.clone());
        Self::valid_signers(state_proof.sigs, &encoded, app_info.players.clone())?;
        ensure!(
            app_info.status != AppStatus::Finalized,
            "app state is finalized"
        );
        ensure!(
            app_state.nonce == app_info.nonce,
            "nonce not match"
        );
        ensure!(
            app_info.seq_num < app_state.seq_num,
            "invalid sequence number"
        );

        app_info.seq_num = app_state.seq_num;
        app_info.deadline = frame_system::Module::<T>::block_number() + app_info.timeout;
        app_info.status = AppStatus::Settle;

        Ok(app_info)
    }

    /// Apply an action to the on-chain state
    ///
    /// Parameter:
    /// `app_info`: Info of app state
    fn apply_action(
        mut app_info: AppInfoOf<T>
    ) -> Result<AppInfoOf<T>, DispatchError> {
        ensure!(
            app_info.status != AppStatus::Finalized,
            "app state is finalized"
        );

        let block_number =  frame_system::Module::<T>::block_number();
        if app_info.status == AppStatus::Settle && block_number > app_info.deadline {
            app_info.seq_num = app_info.seq_num + 1;
            app_info.deadline = block_number + app_info.timeout;
            app_info.status = AppStatus::Action;
        } else {
            ensure!(
                app_info.status ==  AppStatus::Action,
                "app not in action mode"
            );
            app_info.seq_num = app_info.seq_num + 1;
            app_info.deadline = block_number + app_info.timeout;
            app_info.status = AppStatus::Action;
        }

        Ok(app_info)
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
    ) -> DispatchResult {
        for i in 0..2 {
            ensure!(&signatures[i].verify(encoded, &signers[i]), "Check co-sigs failed")
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
        let mut encoded = app_state.nonce.encode();
        encoded.extend(app_state.seq_num.encode());
        encoded.extend(app_state.state.encode());
        encoded.extend(app_state.timeout.encode());
        encoded.extend(app_state.session_id.encode());

        return encoded;
    }

}


   
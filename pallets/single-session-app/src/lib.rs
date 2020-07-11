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
    app_id: Hash,
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
            let app_id = Self::get_app_id(initiate_request.nonce, initiate_request.players.clone());
            ensure!(
                AppInfoMap::<T>::contains_key(&app_id) == false,
                "AppId alreads exists"
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
            AppInfoMap::<T>::insert(app_id, app_info);
        
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
            // submit ad settle off-chain state
            let mut new_app_info: AppInfoOf<T> = Self::intend_settle(state_proof.clone())?;
            
            let state = state_proof.app_state.state;
            if state == 1 || state == 2 {
                new_app_info = AppInfoOf::<T> {
                    state: state,
                    nonce: new_app_info.nonce,
                    players: new_app_info.players,
                    seq_num: new_app_info.seq_num,
                    timeout: new_app_info.timeout,
                    deadline: new_app_info.deadline,
                    status: AppStatus::Finalized
                }
            } else {
                new_app_info = AppInfoOf::<T> {
                    state: state,
                    nonce: new_app_info.nonce,
                    players: new_app_info.players,
                    seq_num: new_app_info.seq_num,
                    timeout: new_app_info.timeout,
                    deadline: new_app_info.deadline,
                    status: new_app_info.status
                }
            }
            let app_id = state_proof.app_state.app_id;
            AppInfoMap::<T>::mutate(&app_id, |app_info| *app_info = Some(new_app_info.clone()));

            // Emit IntendSettle event
            Self::deposit_event(RawEvent::IntendSettle(app_id, new_app_info.seq_num));

            Ok(())
        }
        

        /// Update state according to an on-chain action
        ///
        /// Parameters:
        /// - `app_id`: Id of app
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
            app_id: T::Hash,
            action: u8
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let mut new_app_info: AppInfoOf<T> = Self::apply_action(app_id)?;
        
            if action == 1 || action == 2 {
                new_app_info = AppInfoOf::<T> {
                    state: new_app_info.state,
                    nonce: new_app_info.nonce,
                    players: new_app_info.players,
                    seq_num: new_app_info.seq_num,
                    timeout: new_app_info.timeout,
                    deadline: new_app_info.deadline,
                    status: AppStatus::Finalized,
                }
            } 
            AppInfoMap::<T>::mutate(&app_id, |app_info| *app_info = Some(new_app_info));

            Ok(())
        }

        /// Finalize in case of on-chain action timeout
        ///
        /// Parameters:
        /// - `app_id`: Id of app
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
            app_id: T::Hash
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let app_info = match AppInfoMap::<T>::get(app_id) {
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
            AppInfoMap::<T>::mutate(&app_id, |app_info| *app_info = Some(new_app_info));

            Ok(())
        }

        /// Check whether app is finalized
        ///
        /// Parameters:
        /// - `app_id`: Id of app
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage read `AppInfoMap`
        /// - Based on benchmark;
        ///     10.27　µs
        /// # </weight>
        #[weight = 11_000_000 + T::DbWeight::get().reads(1)]
        pub fn is_finalized(
            origin,
            app_id: T::Hash,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let app_info = match AppInfoMap::<T>::get(app_id) {
                Some(app) => app,
                None => return Err(Error::<T>::AppInfoNotExist)?,
            };

            // If app is not finlized, return DispatchError::Other("NotFianlized")
            ensure!(
                app_info.status == AppStatus::Finalized,
                "NotFinalized"
            );

            // If app is finalized, return Ok(())
            Ok(())
        }

        /// Get the app outcome
        /// 
        /// Parameters:
        /// - `app_id`: Id of app
        /// - `query`: query param
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage read `AppInfoMap`
        /// - Based on benchmark;
        ///     10.83　µs
        /// # </weight>
        #[weight = 11_000_000 + T::DbWeight::get().reads(1)]
        pub fn get_outcome(
            origin,
            app_id: T::Hash,
            query: u8
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let app_info = match AppInfoMap::<T>::get(app_id) {
                Some(app) => app,
                None => Err(Error::<T>::AppInfoNotExist)?,
            };

            // If outcome is false, return DispatchError::Other("FalseOutcome")
            ensure!(
                app_info.state == query,
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
        /// IntendSettle(app_id, seq_num)
        IntendSettle(Hash, u128),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        // AppInfo is not exist
        AppInfoNotExist,
    }
}

impl<T: Trait> Module<T> {   
    /// Get Id of app
    ///
    /// Parameters:
    /// `nonce`: Nonce of app
    /// `players`: AccountId of players
    fn get_app_id(
        nonce: u128,
        players: Vec<T::AccountId>,
    ) -> T::Hash {
        let app_account = Self::app_account();
        let mut encoded = app_account.encode();
        encoded.extend(nonce.encode());
        encoded.extend(players[0].encode());
        encoded.extend(players[1].encode());
        let app_id = T::Hashing::hash(&encoded);
        return app_id;
    }

    /// Get app state
    ///
    /// Parameter:
    /// `app_id`: Id of app
    pub fn get_state(app_id: T::Hash) -> Option<u8> {
        let app_info = match AppInfoMap::<T>::get(app_id) {
            Some(app) => app,
            None => return None,
        };

        return Some(app_info.state);
    }

    /// Get app status
    ///
    /// Parameter:
    /// `app_id`: Id of app
    pub fn get_status(app_id: T::Hash) -> Option<AppStatus> {
        let app_info = match AppInfoMap::<T>::get(app_id) {
            Some(app) => app,
            None => return None,
        };

        return Some(app_info.status);
    }

    /// Get state settle finalized time
    ///
    /// Parameter:
    /// `app_id`: Id of app
    pub fn get_settle_finalized_time(app_id: T::Hash) -> Option<T::BlockNumber> {
        let app_info = match AppInfoMap::<T>::get(app_id) {
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
    /// `app_id`: Id of app
    pub fn get_action_deadline(app_id: T::Hash) -> Option<T::BlockNumber> {
        let app_info = match AppInfoMap::<T>::get(app_id) {
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
    /// `app_id`: Id of app
    pub fn get_seq_num(app_id: T::Hash) -> Option<u128> {
        let app_info = match AppInfoMap::<T>::get(app_id) {
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
    /// `state_proof`: Signed off-chain app state
    fn intend_settle(
        state_proof: StateProofOf<T>
    ) -> Result<AppInfoOf<T>, DispatchError> {
        let app_state = state_proof.app_state;
        let app_info = match AppInfoMap::<T>::get(app_state.app_id) {
            Some(app) => app,
            None => Err(Error::<T>::AppInfoNotExist)?,
        };
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

        let block_number = frame_system::Module::<T>::block_number();
        let new_app_info = AppInfoOf::<T> {
            state: app_info.state,
            nonce: app_info.nonce,
            players: app_info.players,
            seq_num: app_state.seq_num,
            timeout: app_info.timeout,
            deadline: block_number + app_info.timeout,
            status: AppStatus::Settle
        };

        Ok(new_app_info)
    }

    /// Apply an action to the on-chain state
    ///
    /// Parameter:
    /// `app_id`: Id of app
    fn apply_action(
        app_id: T::Hash,
    ) -> Result<AppInfoOf<T>, DispatchError> {
        let app_info = match AppInfoMap::<T>::get(app_id) {
            Some(app) => app,
            None => Err(Error::<T>::AppInfoNotExist)?,
        };
        ensure!(
            app_info.status != AppStatus::Finalized,
            "app state is finalized"
        );

        let block_number =  frame_system::Module::<T>::block_number();
        let new_app_info: AppInfoOf<T>;
        if app_info.status == AppStatus::Settle && block_number > app_info.deadline {
            new_app_info = AppInfoOf::<T> {
                state:  app_info.state,
                nonce: app_info.nonce,
                players: app_info.players,
                seq_num:  app_info.seq_num + 1,
                timeout: app_info.timeout,
                deadline: block_number + app_info.timeout,
                status: AppStatus::Action
            };
        } else {
            ensure!(
                app_info.status ==  AppStatus::Action,
                "app not in action mode"
            );
            new_app_info = AppInfoOf::<T> {
                state: app_info.state,
                nonce: app_info.nonce,
                players: app_info.players,
                seq_num:  app_info.seq_num + 1,
                timeout: app_info.timeout,
                deadline: block_number + app_info.timeout,
                status: AppStatus::Action
            };
        }

        Ok(new_app_info)
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
        let signature1 = &signatures[0];
        let signature2 = &signatures[1];
        ensure!(
            (signature1.verify(encoded, &signers[0]) && signature2.verify(encoded, &signers[1]))
                || (signature1.verify(encoded, &signers[1])
                    && signature2.verify(encoded, &signers[0])),
            "Check co-sigs failed"
        );

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
        encoded.extend(app_state.app_id.encode());

        return encoded;
    }

}


   
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
    player_num: u8,
    players: Vec<AccountId>,
    timeout: BlockNumber,
    min_stone_offchain: u8,
    max_stone_onchain: u8,
}

pub type AppInitiateRequestOf<T> = AppInitiateRequest<
    <T as system::Trait>::AccountId,
    <T as system::Trait>::BlockNumber,
>;

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct AppState<BlockNumber, Hash> {
    seq_num: u128,
    board_state: Vec<u8>,
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
pub struct GomokuInfo<AccountId, BlockNumber> {
    players: Vec<AccountId>,
    player_num: u8,
    seq_num: u128,
    timeout: BlockNumber,
    deadline: BlockNumber,
    status: AppStatus,
    gomoku_state: GomokuState,
}

pub type GomokuInfoOf<T> = GomokuInfo<
    <T as system::Trait>::AccountId,
    <T as system::Trait>::BlockNumber,
>;

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub enum StateKey {
    TurnColor = 0,
    WinnerColor = 1,
    FullState = 2,
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
struct GomokuState {
    board_state: Option<Vec<u8>>, // 228 length: u8 winner color + u8 turn color + u8 black id + 15*15 board
    stone_num: Option<u16>, // number of stones
    stone_num_onchain: Option<u16>, // number of stones placed on-chain
    state_key: Option<StateKey>, // key of turn_color, winner_color, full_state
    min_stone_offchain: u8, // minimal number of stones before go onchain
    max_stone_onchain: u8, // maximal number of stones after go onchain
}

#[derive(Eq, PartialEq)]
pub enum Color {
    Black = 1,
    White = 2,
}

pub const MULTI_GOMOKU_ID: ModuleId = ModuleId(*b"m_gomoku");

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    type Public: IdentifyAccount<AccountId = Self::AccountId>;
    type Signature: Verify<Signer = <Self as Trait>::Public> + Member + Decode + Encode; 
}

decl_storage! {
    trait Store for Module<T: Trait> as MultiGomoku {
        pub MultiGomokuInfoMap get(fn gmoku_info):
            map hasher(blake2_128_concat) T::Hash => Option<GomokuInfoOf<T>>;
    }
}


decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;

        fn deposit_event() = default;

        /// Initate multi gomoku app
        ///
        /// Parameters:
        /// - `initiate_request`: App initiate request message
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(N)`
        ///      - `N` player number
        /// - DB:
        ///   - 1 storage insertion `GomokuInfoMap`
        ///   - 1 storage reads `GomokuxInfoMap`
        /// - Based on benchmark;
        ///     18.59　µs
        /// # </weight>
        #[weight = 19_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn app_initiate(
            origin,
            initiate_request: AppInitiateRequestOf<T>
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let app_id = Self::get_app_id(initiate_request.nonce, initiate_request.players.clone());
            ensure!(
                MultiGomokuInfoMap::<T>::contains_key(&app_id) == false,
                "AppId already exists"
            );

            // check whether account is asscending order
            Self::is_ordered_account(initiate_request.players.clone())?;

            let gomoku_state = GomokuState {
                board_state: None,
                stone_num: None,
                stone_num_onchain: None,
                state_key: None,
                min_stone_offchain: initiate_request.min_stone_offchain,
                max_stone_onchain: initiate_request.max_stone_onchain,
            };
            let gomoku_info = GomokuInfoOf::<T> {
                players: initiate_request.players,
                player_num: initiate_request.player_num,
                seq_num: 0,
                timeout: initiate_request.timeout,
                deadline: Zero::zero(),
                status: AppStatus::Idle,
                gomoku_state: gomoku_state,
            };
            MultiGomokuInfoMap::<T>::insert(app_id, gomoku_info);

            Ok(())
        }

        /// Update on-chain state according to offchain state proof
        ///
        /// Parameters:
        /// - `state_proof`: Signed off-chain session state
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///      - `N` player number
        ///   - 1 storage mutation `GomokuInfoMap`
        ///   - 1 storage read `GomokuInfoMap`
        /// - Based on benchmark;
        ///     49.04　µs
        /// # </weight>
        #[weight = 49_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn update_by_state(
            origin,
            state_proof: StateProofOf<T>
        ) -> DispatchResult {
            ensure_signed(origin)?;
            
            // submit and settle off-chain state
            let mut gomoku_info: GomokuInfoOf<T> = Self::intend_settle(state_proof.clone())?;

            let _state = state_proof.app_state.board_state;
            // u8 winner color + u8 turn color + u8 black ud + 15*15 board state
            ensure!(
                _state.len() == 228,
                "invalid state length"
            );

            let count = 0;
            if _state[0] != 0 {
                gomoku_info = Self::win_game(_state[0], gomoku_info.clone())?;
            } else {
                // advance to _state[3]
                let mut _state_iter = _state.iter();
                for _i in 0..4 {
                    _state_iter.next();
                }
                // load other states only if winner color is not BLACK or WHITE
                let count = _state_iter.filter(|&x| *x != 0).count() as u8;

                ensure!(
                    count >= gomoku_info.gomoku_state.min_stone_offchain,
                    "not enough offchain stones"
                );
            }

            let new_gomoku_state = GomokuState {
                board_state: Some(_state),
                stone_num: Some(count),
                stone_num_onchain: gomoku_info.gomoku_state.stone_num_onchain,
                state_key: gomoku_info.gomoku_state.state_key,
                min_stone_offchain: gomoku_info.gomoku_state.min_stone_offchain,
                max_stone_onchain: gomoku_info.gomoku_state.max_stone_onchain,
            };
            let new_gomoku_info = GomokuInfoOf::<T> {
                players: gomoku_info.players,
                player_num: gomoku_info.player_num,
                seq_num: gomoku_info.seq_num,
                timeout: gomoku_info.timeout,
                deadline: gomoku_info.deadline,
                status: gomoku_info.status,
                gomoku_state: new_gomoku_state,
            };
            let app_id = state_proof.app_state.app_id;
            MultiGomokuInfoMap::<T>::mutate(app_id, |info| *info = Some(new_gomoku_info.clone()));

            Self::deposit_event(RawEvent::IntendSettle(app_id, new_gomoku_info.seq_num));

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
        ///   - 2 storage mutation `GomokuInfoMap`
        ///   - 1 storage read `GomokuInfoMap`
        /// - Based on benchmark;
        ///     46.07　µs
        /// # </weight>
        #[weight = 46_000_000 + T::DbWeight::get().reads_writes(1, 2)]
        fn update_by_action(
            origin,
            app_id: T::Hash,
            action: Vec<u8>
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            // apply an action to the on-chain state
            let gomoku_info = Self::apply_action(app_id)?;
            let gomoku_state = gomoku_info.gomoku_state.clone();
            let mut board_state = match gomoku_state.board_state {
                Some(state) => state,
                None => Err(Error::<T>::EmptyBoardState)?,
            };
            let turn_color: usize = board_state[1] as usize;
            // black player index, smaller (=1) or larger(=2) addr
            let black_id = board_state[2];
            if black_id == 1 {
                ensure!(
                    caller == gomoku_info.players[turn_color - 1],
                    "Not your turn"
                );
            } else if black_id == 2 {
                ensure!(
                    caller == gomoku_info.players[2 - turn_color],
                    "Not your turn"
                )
            } else {
                Err(Error::<T>::InvalidBlackId)?
            }
            ensure!(
                action.len() == 2,
                "invalid action length"
            );
            let x = action[0];
            let y = action[1];
            ensure!(
                Self::check_boundary(x, y),
                "out of boundary"
            );
            let index: usize = Self::state_index(x, y);
            ensure!(
                board_state[index] == 0,
                "slot is occupied"
            );

            // place the stone
            board_state[index] = turn_color as u8;
            let new_stone_num = gomoku_state.stone_num.unwrap_or(0) + 1;
            let new_stone_num_onchain = gomoku_state.stone_num_onchain.unwrap_or(0) + 1;
            let new_gomoku_state_1 = GomokuState {
                board_state: Some(board_state.clone()),
                stone_num: Some(new_stone_num),
                stone_num_onchain: Some(new_stone_num_onchain),
                state_key: gomoku_state.state_key.clone(),
                min_stone_offchain: gomoku_state.min_stone_offchain,
                max_stone_onchain: gomoku_state.max_stone_onchain,
            };
            let mut new_gomoku_info_1 = GomokuInfoOf::<T> {
                players: gomoku_info.players.clone(),
                player_num: gomoku_info.player_num,
                seq_num: gomoku_info.seq_num,
                timeout: gomoku_info.timeout,
                deadline: gomoku_info.deadline,
                status: gomoku_info.status.clone(),
                gomoku_state: new_gomoku_state_1,
            };
            MultiGomokuInfoMap::<T>::mutate(app_id, |info| *info = Some(new_gomoku_info_1.clone()));

            // check if there is five-in-a-row including this new stone
            if Self::check_five(board_state.clone(), x, y, 1, 0) // horizontal bidirection
                || Self::check_five(board_state.clone(), x, y, 0, 1) // vertical bidirection
                || Self::check_five(board_state.clone(), x, y, 1, 1) // main-diagonal bidirection
                || Self::check_five(board_state.clone(), x, y, 1, -1) // anti-diagonal bidirection
            {
                new_gomoku_info_1 = Self::win_game(turn_color as u8, new_gomoku_info_1)?;
                MultiGomokuInfoMap::<T>::mutate(app_id, |info| *info = Some(new_gomoku_info_1));
                return Ok(());
            }

            if new_stone_num == 225 
                || new_stone_num_onchain as u8 > gomoku_state.max_stone_onchain {
                    // all slots occupied, game is over with no winner
                    board_state[1] = 0;
                    let new_gomoku_state_2 = GomokuState {
                        board_state: Some(board_state),
                        stone_num: Some(new_stone_num),
                        stone_num_onchain: Some(new_stone_num_onchain),
                        state_key: gomoku_state.state_key,
                        min_stone_offchain: gomoku_state.min_stone_offchain,
                        max_stone_onchain: gomoku_state.max_stone_onchain,
                    };
                    let new_gomoku_info_2 = GomokuInfoOf::<T> {
                        players: gomoku_info.players,
                        player_num: gomoku_info.player_num,
                        seq_num: gomoku_info.seq_num,
                        timeout: gomoku_info.timeout,
                        deadline: gomoku_info.deadline,
                        status: AppStatus::Finalized,
                        gomoku_state: new_gomoku_state_2,
                    };
                    MultiGomokuInfoMap::<T>::mutate(app_id, |info| *info = Some(new_gomoku_info_2.clone()));
            } else {
                // toggle turn and update game phase
                if turn_color == Color::Black as usize {
                    // set turn color white
                    board_state[1] = 2;
                } else {
                    // set turn color black
                    board_state[1] = 1;
                }
                let new_gomoku_state_2 = GomokuState {
                    board_state: Some(board_state),
                    stone_num: Some(new_stone_num),
                    stone_num_onchain: Some(new_stone_num_onchain),
                    state_key: gomoku_state.state_key,
                    min_stone_offchain: gomoku_state.min_stone_offchain,
                    max_stone_onchain: gomoku_state.max_stone_onchain,
                };
                let new_gomoku_info_2 = GomokuInfoOf::<T> {
                    players: gomoku_info.players,
                    player_num: gomoku_info.player_num,
                    seq_num: gomoku_info.seq_num,
                    timeout: gomoku_info.timeout,
                    deadline: gomoku_info.deadline,
                    status: gomoku_info.status,
                    gomoku_state: new_gomoku_state_2,
                };
                MultiGomokuInfoMap::<T>::mutate(app_id, |info| *info = Some(new_gomoku_info_2.clone()));
            }

            Ok(())
        }

        /// Finalize the app based on current state in case of on-chain action timeout
        ///
        /// Parameters:
        /// - `app_id`: Id of app
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage mutation `GomokuInfoMap`
        ///   - 1 storage read `GomokuInfoMapp`
        /// - Based on benchmark;
        ///     29.36 µs
        /// # </weight>
        #[weight = 30_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn finalize_on_action_timeout(
            origin,
            app_id: T::Hash
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let gomoku_info = match MultiGomokuInfoMap::<T>::get(app_id) {
                Some(info) => info,
                None => Err(Error::<T>::MultiGomokuInfoNotExist)?,
            };

            let block_number = frame_system::Module::<T>::block_number();
            if gomoku_info.status == AppStatus::Action {
                ensure!(
                    block_number >  gomoku_info.deadline,
                    "deadline does not passes"
                );
            } else if gomoku_info.status == AppStatus::Settle {
                ensure!(
                    block_number > gomoku_info.deadline + gomoku_info.timeout,
                    "while setting"
                );
            } else {
                return Ok(());
            }

            let board_state = match gomoku_info.clone().gomoku_state.board_state {
                Some(state) => state,
                None => Err(Error::<T>::EmptyBoardState)?,
            };

            if board_state[1] == Color::Black as u8 {
                let new_gomoku_info = Self::win_game(2, gomoku_info)?;
                MultiGomokuInfoMap::<T>::mutate(app_id, |info| *info = Some(new_gomoku_info));
            } else if board_state[1] == Color::White as u8 {
                let new_gomoku_info = Self::win_game(1, gomoku_info)?;
                MultiGomokuInfoMap::<T>::mutate(app_id, |info| *info = Some(new_gomoku_info));
            } else {
                return Ok(());
            }

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
        ///   - 1 storage read `GomokuInfoMap`
        /// - Based on benchmark;
        ///     12.06　µs
        /// # </weight>
        #[weight = 12_000_000 + T::DbWeight::get().reads(1)]
        pub fn is_finalized(
           origin,
           app_id: T::Hash
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let gomoku_info = match MultiGomokuInfoMap::<T>::get(app_id) {
                Some(info) => info,
                None => Err(Error::<T>::MultiGomokuInfoNotExist)?,
            };

            // If app is not finalized, return DispatchError::Other("NotFinalized")
            ensure!(
                gomoku_info.status == AppStatus::Finalized,
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
        ///   - 1 storage read `GomokuInfoMap`
        /// - Based on benchmark;
        ///     11.88　µs
        /// # </weight>
        #[weight = 12_000_000 + T::DbWeight::get().reads(1)]
        pub fn get_outcome(
            origin,
            app_id: T::Hash,
            query: u8,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let gomoku_info = match MultiGomokuInfoMap::<T>::get(app_id) {
                Some(info) => info,
                None => Err(Error::<T>::MultiGomokuInfoNotExist)?,
            };
            let board_state = match gomoku_info.gomoku_state.board_state {
                Some(state) => state,
                None => Err(Error::<T>::EmptyBoardState)?,
            };

            // If outcome is false, return DispatchError::Other("FalseOutcome")
            ensure!(
                board_state[0] == query,
                "FalseOutcome"
            );

            // If outcome is ture, return Ok(())
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
        // MultiGomokuInfo is not exist
        MultiGomokuInfoNotExist,
        // BoardState is empty
        EmptyBoardState,
        // BlackId is invalid
        InvalidBlackId,
    }
}

impl<T: Trait> Module<T> {
    /// Get Id of app
    ///
    /// Parameters:
    /// `nonce`: Nonce of app
    /// `players`: AccountId of players
    pub fn get_app_id(
       nonce:  u128,
       players: Vec<T::AccountId>,
    ) -> T::Hash {
        let multi_gomoku_app_account = Self::app_account();
        let mut encoded = multi_gomoku_app_account.encode();
        encoded.extend(nonce.encode());
        players.into_iter()
            .for_each(|players| { encoded.extend(players.encode()); });
        let app_id = T::Hashing::hash(&encoded);
        return app_id;
    }

    /// Get app state
    ///
    /// Parameters:
    /// `app_id`: Id of app
    /// `key`: Query key 0:WinnerColor, 1:TurnColor, 2:FullState
    pub fn get_state(app_id: T::Hash, key: u8) -> Option<Vec<u8>> {
        let gomoku_info = match MultiGomokuInfoMap::<T>::get(app_id) {
            Some(info) => info,
            None => return None
        };
        let board_state = gomoku_info.gomoku_state.board_state.unwrap();
        if key == StateKey::WinnerColor as u8 {
            let state = vec![board_state[0]];
            return Some(state);
        } else if key == StateKey::TurnColor as u8 {
            let state = vec![board_state[1]];
            return Some(state);
        } else if key == StateKey::FullState as u8 {
            return Some(board_state);
        } else {
            return None;
        }
    }

    /// Get app status
    ///
    /// Parameter:
    /// `app_id`: Id of app
    pub fn get_status(app_id: T::Hash) -> Option<AppStatus> {
        let gomoku_info = match MultiGomokuInfoMap::<T>::get(app_id) {
            Some(info) => info,
            None => return None
        };

        return Some(gomoku_info.status);
    }

    /// Get state settle finalized time
    ///
    /// Parameter:
    /// `app_id`: Id of app
    pub fn get_settle_finalized_time(app_id: T::Hash) -> Option<T::BlockNumber> {
        let gomoku_info = match MultiGomokuInfoMap::<T>::get(app_id) {
            Some(info) => info,
            None => return None
        };

        if gomoku_info.status == AppStatus::Settle {
            return Some(gomoku_info.deadline);
        } else {
            return None;
        }
    }

    /// Get action deadline
    ///
    /// Parameter:
    /// `app_id`: Id of app
    pub fn get_action_deadline(app_id: T::Hash) -> Option<T::BlockNumber> {
        let gomoku_info = match MultiGomokuInfoMap::<T>::get(app_id) {
            Some(info) => info,
            None => return None
        };

        if gomoku_info.status == AppStatus::Action {
            return Some(gomoku_info.deadline);
        } else if gomoku_info.status == AppStatus::Settle {
            return Some(gomoku_info.deadline + gomoku_info.timeout);
        } else {
            return None;
        }
    }

    /// Get app sequence number
    ///
    /// Parameter:
    /// `app_id`: Id of app
    pub fn get_seq_num(app_id: T::Hash) -> Option<u128> {
        let gomoku_info = match MultiGomokuInfoMap::<T>::get(app_id) {
            Some(info) => info,
            None => return None
        };

        return Some(gomoku_info.seq_num);
    }

    /// Get multi gomoku app account id
    pub fn app_account() -> T::AccountId {
        MULTI_GOMOKU_ID.into_account()
    }

    /// Submit and settle offchain state
    ///
    /// Parameter:
    /// `state_proof`: Signed off-chain app state
    fn intend_settle(
        state_proof: StateProofOf<T>
    ) -> Result<GomokuInfoOf<T>, DispatchError> {
        let app_state = state_proof.app_state;
        let gomoku_info = match MultiGomokuInfoMap::<T>::get(app_state.app_id) {
            Some(info) => info,
            None => Err(Error::<T>::MultiGomokuInfoNotExist)?,
        };
        let encoded = Self::encode_app_state(app_state.clone());
        Self::valid_signers(state_proof.sigs, &encoded, gomoku_info.players.clone())?;
        ensure!(
            gomoku_info.status != AppStatus::Finalized,
            "app state is finalized"
        );
    
        ensure!(
            gomoku_info.seq_num < app_state.seq_num,
            "invalid sequence number"
        );

        let block_number = frame_system::Module::<T>::block_number();
        let new_gomoku_info = GomokuInfoOf::<T> {
            players: gomoku_info.players,
            player_num: gomoku_info.player_num,
            seq_num: gomoku_info.seq_num,
            timeout: gomoku_info.timeout,
            deadline: block_number + gomoku_info.deadline,
            status: AppStatus::Settle,
            gomoku_state: gomoku_info.gomoku_state,
        };

        Ok(new_gomoku_info)
    }

    /// Apply an action to the on-chain state
    ///
    /// Parameter:
    /// `app_id`: Id of app
    fn apply_action(
        app_id: T::Hash
    ) -> Result<GomokuInfoOf<T>, DispatchError> {
        let gomoku_info = match MultiGomokuInfoMap::<T>::get(app_id) {
            Some(info) => info,
            None => Err(Error::<T>::MultiGomokuInfoNotExist)?,
        };
        ensure!(
            gomoku_info.status != AppStatus::Finalized,
            "app state is finalized"
        );

        let block_number =  frame_system::Module::<T>::block_number();
        let new_gomoku_info: GomokuInfoOf<T>;
        if gomoku_info.status == AppStatus::Settle && block_number > gomoku_info.deadline {
            new_gomoku_info = GomokuInfoOf::<T> {
                players: gomoku_info.players,
                player_num: gomoku_info.player_num,
                seq_num:  gomoku_info.seq_num + 1,
                timeout: gomoku_info.timeout,
                deadline: block_number + gomoku_info.timeout,
                status: AppStatus::Action,
                gomoku_state: gomoku_info.gomoku_state
            };
        } else {
            ensure!(
                gomoku_info.status ==  AppStatus::Action,
                "app not in action mode"
            );
            new_gomoku_info = GomokuInfoOf::<T> {
                players: gomoku_info.players,
                player_num: gomoku_info.player_num,
                seq_num:  gomoku_info.seq_num + 1,
                timeout: gomoku_info.timeout,
                deadline: block_number + gomoku_info.timeout,
                status: AppStatus::Action,
                gomoku_state: gomoku_info.gomoku_state
            };
        }

        Ok(new_gomoku_info) 
    }
    
    /// Check whether account is assceding order
    /// 
    /// Parameter:
    /// `players`: AccountId of players
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

    /// Verify off-chain state signatures
    ///
    /// Parameters:
    /// `signatures`: Signaturs from the players
    /// `encoded`: Encoded app state
    /// `signers`: AccountId of players
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

    /// Set game states when there is a winner
    ///
    /// Parameters:
    /// `winner`: Id of winner
    /// `gomoku_info`: Info of gomoku state
    fn win_game(
        winner: u8,
        gomoku_info: GomokuInfoOf<T>
    ) -> Result<GomokuInfoOf<T>, DispatchError> {
        ensure!(
            u8::min_value() <= winner && winner <= 2,
            "invalid winner state"
        );
        let gomoku_state = gomoku_info.gomoku_state;
        let mut new_board_state = gomoku_state.board_state.unwrap_or(vec![0; 228]);
        // set winner color
        new_board_state[0] = winner;

        let new_gomoku_info: GomokuInfoOf<T>;
        if winner != 0 {// Game over
            // set turn color 0
            new_board_state[1] = 0; 

            let new_gomoku_state = GomokuState {
                board_state: Some(new_board_state),
                stone_num: gomoku_state.stone_num,
                stone_num_onchain: gomoku_state.stone_num_onchain,
                state_key: gomoku_state.state_key,
                min_stone_offchain: gomoku_state.min_stone_offchain,
                max_stone_onchain: gomoku_state.max_stone_onchain
            };
            
            new_gomoku_info = GomokuInfoOf::<T> {
                players: gomoku_info.players,
                player_num: gomoku_info.player_num,
                seq_num: gomoku_info.seq_num,
                timeout: gomoku_info.timeout,
                deadline: gomoku_info.deadline,
                status: AppStatus::Finalized,
                gomoku_state: new_gomoku_state,
            };
        } else {
            let new_gomoku_state = GomokuState {
                board_state: Some(new_board_state),
                stone_num: gomoku_state.stone_num,
                stone_num_onchain: gomoku_state.stone_num_onchain,
                state_key: gomoku_state.state_key,
                min_stone_offchain: gomoku_state.min_stone_offchain,
                max_stone_onchain: gomoku_state.max_stone_onchain
            };
            new_gomoku_info = GomokuInfoOf::<T> {
                players: gomoku_info.players,
                player_num: gomoku_info.player_num,
                seq_num: gomoku_info.seq_num,
                timeout: gomoku_info.timeout,
                deadline: gomoku_info.deadline,
                status: gomoku_info.status,
                gomoku_state: new_gomoku_state,
            };
        }
        
        return Ok(new_gomoku_info);
    }

    /// Check if there is five in a row in agiven direction
    ///
    /// Parameters:
    /// `_x`: x coordinate on the board
    /// `_y`: y coordinate on the board
    /// `_xdir`: direction (-1 or 0 or 1) in x axis
    /// `_ydir`: direction (-1 or 0 or 1) in y axis
    fn check_five(
        _board_state: Vec<u8>,
        _x: u8,
        _y: u8,
        _xdir: i8,
        _ydir: i8,
    ) -> bool {
        let mut count: u8 = 0;
        count += Self::count_stone(_board_state.clone(), _x, _y, _xdir, _ydir).unwrap();
        count += Self::count_stone(_board_state, _x, _y, -1 * _xdir, -1 * _ydir).unwrap() - 1; // reverse direction
        if count >= 5 {
            return true
        } else {
            return false;
        }
    }

    /// Count the maximum consecutive stones in a given direction
    ///
    /// Parameters:
    /// `_x`: x coordinate on the board
    /// `_y`: y coordinate on the board
    /// `_xdir`: direction (-1 or 0 or 1) in x axis
    /// `_ydir`: direction (-1 or 0 or 1) in y axis
    fn count_stone(
        _board_state: Vec<u8>, 
        _x: u8, 
        _y: u8, 
        _xdir: i8, 
        _ydir: i8
    ) -> Option<u8> {
        let mut count: u8 = 1;
        while count <= 5 {
            let x = (_x as i8 + _xdir * count as i8) as u8;
            let y = (_y as i8 + _ydir * count as i8) as u8;
            if Self::check_boundary(x, y) 
                && (_board_state[Self::state_index(x, y)] == _board_state[Self::state_index(_x, _y)]) {
                    count += 1;
            } else {
                return Some(count);
            }
        }

        return None;
    }

    /// Check if coordinate (x, y) is valid
    ///
    /// Parameters:
    /// `_x`: x coordinate on the board
    /// `_y`: y coordinate on the board
    fn check_boundary(x: u8, y: u8) -> bool {
        // board dimention is 15*15
        let board_dimention = 15;
        if x < board_dimention && y < board_dimention {
            return true;
        } else {
            return false;
        }
    }

    /// Check if coordinate (x, y) is valid
    ///
    /// Parameters:
    /// `_x`: x coordinate on the board
    /// `_y`: y coordinate on the board
    fn state_index(x: u8, y: u8) -> usize {
        // board dimention is 15*15
        let board_dimention = 15;
        let index = (3 + board_dimention * x + y) as usize;
        return index;
    }

    /// Encode app state
    ///
    /// Parameter:
    /// `app_state`: app state
    fn encode_app_state(
        app_state: AppStateOf<T>
    ) -> Vec<u8> {
        let mut encoded = app_state.seq_num.encode();
        app_state.board_state.iter()
            .for_each(|state| { encoded.extend(state.encode()); });
        encoded.extend(app_state.timeout.encode());
        encoded.extend(app_state.app_id.encode());

        return encoded;
    }
}
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
    min_stone_offchain: u8,
    max_stone_onchain: u8,
}

pub type AppInitiateRequestOf<T> = AppInitiateRequest<
    <T as system::Trait>::AccountId,
    <T as system::Trait>::BlockNumber,
>;

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
pub struct AppState<BlockNumber, Hash> {
    nonce: u128,
    seq_num: u128,
    board_state: Vec<u8>,
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
pub struct GomokuInfo<AccountId, BlockNumber> {
    nonce: u128,
    players: Vec<AccountId>,
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
    Turn = 0,
    Winner = 1,
    FullState = 2,
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord, Encode, Decode, RuntimeDebug)]
struct GomokuState {
    board_state: Option<Vec<u8>>, // 227 length: u8 winner + u8 turn + 15*15 board
    stone_num: Option<u16>, // number of stones
    stone_num_onchain: Option<u16>, // number of stones places on-chain
    state_key: Option<StateKey>, // key of turn, winner fullstate
    min_stone_offchain: u8, // minimal number of stones before go onchain
    max_stone_onchain: u8, // maximal number of stones after go onchain
}

pub const SINGLE_GOMOKU_ID: ModuleId = ModuleId(*b"s_gomoku");

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;
    type Public: IdentifyAccount<AccountId = Self::AccountId>;
    type Signature: Verify<Signer = <Self as Trait>::Public> + Member + Decode + Encode; 
}

decl_storage! {
    trait Store for Module<T: Trait> as SingleGomoku {
        pub SingleGomokuInfoMap get(fn gomoku_info): 
            map hasher(blake2_128_concat) T::Hash => Option<GomokuInfoOf<T>>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        
        fn deposit_event() = default;

        /// Initiate single gomoku app
        ///
        /// Parameters:
        /// - `initiate_request`: App initiate request message
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage insertion `GomokuInfoMap`
        ///   - 1 storage reads `GomokuxInfoMap`
        /// - Based on benchmark;
        ///     17.89　µs
        /// # </weight>
        #[weight = 18_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn app_initiate(
            origin,
            initiate_request: AppInitiateRequestOf<T>
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let session_id = Self::get_session_id(initiate_request.nonce, initiate_request.players.clone());
            ensure!(
                SingleGomokuInfoMap::<T>::contains_key(&session_id) == false,
                "AppId already exists"
            );
            ensure!(
                initiate_request.players.len() == 2,
                "invalid player length"
            );
            ensure!(
                initiate_request.players[0] < initiate_request.players[1],
                "players is not asscending order"
            );

            let gomoku_state = GomokuState {
                board_state: None,
                stone_num: None,
                stone_num_onchain: None,
                state_key: None,
                min_stone_offchain: initiate_request.min_stone_offchain,
                max_stone_onchain: initiate_request.max_stone_onchain,
            };
            let gomoku_info = GomokuInfoOf::<T> {
                nonce: initiate_request.nonce,
                players: initiate_request.players,
                seq_num: 0,
                timeout: initiate_request.timeout,
                deadline: Zero::zero(),
                status: AppStatus::Idle,
                gomoku_state: gomoku_state,
            };
            SingleGomokuInfoMap::<T>::insert(session_id, gomoku_info);

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
        ///   - 1 storage mutation `GomokuInfoMap`
        ///   - 1 storage read `GomokuInfoMap`
        /// - Based on benchmark;
        ///     50.27　µs
        /// # </weight>
        #[weight = 51_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn update_by_state(
            origin,
            state_proof: StateProofOf<T>
        ) -> DispatchResult {
            ensure_signed(origin)?;

            let session_id = state_proof.app_state.session_id;
            let gomoku_info = match SingleGomokuInfoMap::<T>::get(session_id) {
                Some(info) => info,
                None => Err(Error::<T>::SingleGomokuInfoNotExist)?,
            };

            // submit and settle off-chain state
            let mut new_gomoku_info: GomokuInfoOf<T> = Self::intend_settle(gomoku_info, state_proof.clone())?;

            let _state = state_proof.app_state.board_state;
            ensure!(
                _state.len() == 227,
                "invalid board state length"
            );

            let count = 0;
            if _state[0] != 0 {
                new_gomoku_info = Self::win_game(_state[0], new_gomoku_info.clone())?;
            } else {
                // advance to _state[2];
                let mut _state_iter = _state.iter();
                for _i in 0..3 {
                    _state_iter.next();
                }
                // load other states only if winner is not specified
                let count = _state_iter.filter(|&x| *x != 0).count() as u8;
    
                ensure!(
                    count >= new_gomoku_info.gomoku_state.min_stone_offchain,
                    "not enough offchain stones"
                );
            }

            new_gomoku_info.gomoku_state.board_state = Some(_state);
            new_gomoku_info.gomoku_state.stone_num = Some(count);
            SingleGomokuInfoMap::<T>::mutate(session_id, |info| *info = Some(new_gomoku_info.clone()));
            
            Self::deposit_event(RawEvent::IntendSettle(session_id, new_gomoku_info.seq_num));

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
        ///   - 2 storage mutation `GomokuInfoMap`
        ///   - 1 storage read `GomokuInfoMap`
        /// - Based on benchmark;
        ///     47.23　µs
        /// # </weight>
        #[weight = 48_000_000 + T::DbWeight::get().reads_writes(1, 2)]
        fn update_by_action(
            origin,
            session_id: T::Hash,
            action: Vec<u8>
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let gomoku_info = match SingleGomokuInfoMap::<T>::get(session_id) {
                Some(info) => info,
                None => Err(Error::<T>::SingleGomokuInfoNotExist)?,
            };
            
            // apply an action to the on-chain state except for gomoku state
            let mut new_gomoku_info = Self::apply_action(gomoku_info)?;

            let gomoku_state = new_gomoku_info.gomoku_state.clone();
            let mut board_state = new_gomoku_info.gomoku_state.board_state.unwrap_or(vec![0; 227]);
            let turn = board_state[1];
            ensure!(
                caller == new_gomoku_info.players[turn as usize - 1],
                "not your turn"    
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
            board_state[index] = turn;
            let new_stone_num = gomoku_state.stone_num.unwrap_or(0) + 1;
            let new_stone_num_onchain = gomoku_state.stone_num_onchain.unwrap_or(0) + 1;
            new_gomoku_info.gomoku_state =  GomokuState {
                board_state: Some(board_state.clone()),
                stone_num: Some(new_stone_num),
                stone_num_onchain: Some(new_stone_num_onchain),
                state_key: gomoku_state.state_key.clone(),
                min_stone_offchain: gomoku_state.min_stone_offchain,
                max_stone_onchain: gomoku_state.max_stone_onchain,
            };

            // check if there is five-in-a-row including this new stone
            if Self::check_five(board_state.clone(), x, y, 1, 0) // horizontal bidirection
                || Self::check_five(board_state.clone(), x, y, 0, 1) // vertical bidirection
                || Self::check_five(board_state.clone(), x, y, 1, 1) // main-diagonal bidirection
                || Self::check_five(board_state.clone(), x, y, 1, -1) // anti-diagonal bidirection
            {
                new_gomoku_info = Self::win_game(turn, new_gomoku_info.clone())?;
                SingleGomokuInfoMap::<T>::mutate(session_id, |info| *info = Some(new_gomoku_info));
                return Ok(());
            }

            if new_stone_num == 225 
                || new_stone_num_onchain as u8 > gomoku_state.max_stone_onchain {
                    // all slots occupied, game is over with no winner
                    // set turn 0
                    board_state[1] = 0;
                    new_gomoku_info.status = AppStatus::Finalized;
                    new_gomoku_info.gomoku_state = GomokuState {
                        board_state: Some(board_state),
                        stone_num: Some(new_stone_num),
                        stone_num_onchain: Some(new_stone_num_onchain),
                        state_key: gomoku_state.state_key,
                        min_stone_offchain: gomoku_state.min_stone_offchain,
                        max_stone_onchain: gomoku_state.max_stone_onchain,
                    };
                    SingleGomokuInfoMap::<T>::mutate(session_id, |info| *info = Some(new_gomoku_info));
            } else {
                // toggle turn and update game phase
                if turn == 1 {
                    // set turn 2
                    board_state[1] = 2;
                } else {
                    // set turn 1
                    board_state[1] = 1;
                }
                new_gomoku_info.gomoku_state = GomokuState {
                    board_state: Some(board_state),
                    stone_num: Some(new_stone_num),
                    stone_num_onchain: Some(new_stone_num_onchain),
                    state_key: gomoku_state.state_key,
                    min_stone_offchain: gomoku_state.min_stone_offchain,
                    max_stone_onchain: gomoku_state.max_stone_onchain,
                };
                SingleGomokuInfoMap::<T>::mutate(session_id, |info| *info = Some(new_gomoku_info));
            }

            Ok(())
        }

        /// Finalized based on current state in case of on-chain action timeout
        ///
        /// Parameters:
        /// - `session_id`: Id of app
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage mutation `GomokuInfoMap`
        ///   - 1 storage read `GomokuInfoMapp`
        /// - Based on benchmark;
        ///     30.51　µs
        /// # </weight>
        #[weight = 31_000_000 + T::DbWeight::get().reads_writes(1, 1)]
        fn finalize_on_action_timeout(
            origin,
            session_id: T::Hash
        ) -> DispatchResult {
            let gomoku_info = match SingleGomokuInfoMap::<T>::get(session_id) {
                Some(info) => info,
                None => Err(Error::<T>::SingleGomokuInfoNotExist)?,
            };

            let block_number = frame_system::Module::<T>::block_number();
            if gomoku_info.status == AppStatus::Action {
                ensure!(
                    block_number > gomoku_info.deadline,
                    "deadline no passes"
                );
            } else if gomoku_info.status == AppStatus::Settle {
                ensure!(
                    block_number > gomoku_info.deadline + gomoku_info.timeout,
                    "while settling"
                );
            } else {
                return Ok(());
            }

            let board_state = match gomoku_info.clone().gomoku_state.board_state {
                Some(state) => state,
                None => Err(Error::<T>::EmptyBoardState)?,
            };
            if board_state[1] == 1 {
                let new_gomoku_info = Self::win_game(2, gomoku_info)?;
                SingleGomokuInfoMap::<T>::mutate(session_id, |info| *info = Some(new_gomoku_info.clone()));
            } else if board_state[1] == 2 {
                let new_gomoku_info = Self::win_game(1, gomoku_info)?;
                SingleGomokuInfoMap::<T>::mutate(session_id, |info| *info = Some(new_gomoku_info.clone()));
            } else {
                return Ok(());
            }

            Ok(())
        }

        /// Check whether app is finalized
        ///
        /// Parameters:
        /// - `session_id`: Id of app
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage read `GomokuInfoMap`
        /// - Based on benchmark;
        ///     13.7　µs
        /// # </weight>
        #[weight = 14_000_000 + T::DbWeight::get().reads(1)]
        pub fn is_finalized(
            origin,
            session_id: T::Hash
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let gomoku_info = match SingleGomokuInfoMap::<T>::get(session_id) {
                Some(info) => info,
                None => Err(Error::<T>::SingleGomokuInfoNotExist)?,
            };

            // If app is not finalized, return DispatchError::Other("NotFinalized")
            ensure!(
                gomoku_info.status ==  AppStatus::Finalized,
                "NotFinalized"
            );

            // If app is finalized, return Ok(())
            Ok(())
        }

        /// Get the app outcome
        /// 
        /// Parameters:
        /// - `session_id`: Id of app
        /// - `query`: query param
        ///
        /// # <weight>
        /// ## Weight
        /// - Complexity: `O(1)`
        ///   - 1 storage read `GomokuInfoMap`
        /// - Based on benchmark;
        ///     12.06　µs
        /// # </weight>
        #[weight = 12_000_000 + T::DbWeight::get().reads(1)]
        pub fn get_outcome(
            origin,
            session_id: T::Hash,
            query: u8,
        ) -> DispatchResult {
            ensure_signed(origin)?;
            let gomoku_info = match SingleGomokuInfoMap::<T>::get(session_id) {
                Some(info) => info,
                None => Err(Error::<T>::SingleGomokuInfoNotExist)?,
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
        // SingleGomokuInfo is not exist
        SingleGomokuInfoNotExist,
        // BoardState is empty
        EmptyBoardState,
    }
}

impl<T: Trait> Module<T> {
    /// Get Id of app
    ///
    /// Parameters:
    /// `nonce`: Nonce of app
    /// `players`: AccountId of players
    pub fn get_session_id(
        nonce: u128,
        players: Vec<T::AccountId>,
    ) -> T::Hash {
        let single_gomoku_app_account = Self::app_account();
        let mut encoded = single_gomoku_app_account.encode();
        encoded.extend(nonce.encode());
        encoded.extend(players[0].encode());
        encoded.extend(players[1].encode());
        let session_id = T::Hashing::hash(&encoded);
        return session_id;
    }

    /// Get app state
    ///
    /// Parameters:
    /// `session_id`: Id of app
    /// `key`: Query key 0:Turn, 1:Winner, 2:FullState
    pub fn get_state(session_id: T::Hash, key: u8) -> Option<Vec<u8>> {
        let gomoku_info = match SingleGomokuInfoMap::<T>::get(session_id) {
            Some(info) => info,
            None => return None
        };
        let board_state = gomoku_info.gomoku_state.board_state.unwrap();
        if key == StateKey::Winner as u8 {
            return Some(vec![board_state[0]]);
        } else if key == StateKey::Turn as u8 {
            return Some(vec![board_state[1]]);
        } else if key == StateKey::FullState as u8 {
            return Some(board_state);
        } else {
            return None;
        }
    }

    /// Get app status
    ///
    /// Parameter:
    /// `session_id`: Id of app
    pub fn get_status(session_id: T::Hash) -> Option<AppStatus> {
        let gomoku_info = match SingleGomokuInfoMap::<T>::get(session_id) {
            Some(app) => app,
            None => return None,
        };

        return Some(gomoku_info.status);
    }

    /// Get state settle finalized time
    ///
    /// Parameter:
    /// `session_id`: Id of app
    pub fn get_settle_finalized_time(session_id: T::Hash) -> Option<T::BlockNumber> {
        let gomoku_info = match SingleGomokuInfoMap::<T>::get(session_id) {
            Some(info) => info,
            None => return None
        };
        if gomoku_info.status == AppStatus::Settle {
            return Some(gomoku_info.deadline);
        }

        return None;
    }

    /// Get action deadline
    ///
    /// Parameter:
    /// `session_id`: Id of app
    pub fn get_action_deadline(session_id: T::Hash) -> Option<T::BlockNumber> {
        let gomoku_info = match SingleGomokuInfoMap::<T>::get(session_id) {
            Some(info) => info,
            None => return None
        };
        if gomoku_info.status == AppStatus::Action {
            return Some(gomoku_info.deadline);
        } else if gomoku_info.status ==  AppStatus::Settle {
            return Some(gomoku_info.deadline + gomoku_info.timeout);
        } else {
            return None;
        }
    }

    /// Get app sequence number
    ///
    /// Parameter:
    /// `session_id`: Id of app
    pub fn get_seq_num(session_id: T::Hash) -> Option<u128> {
        let gomoku_info = match SingleGomokuInfoMap::<T>::get(session_id) {
            Some(info) => info,
            None => return None
        };
        return Some(gomoku_info.seq_num);
    }

    /// Get single gomoku app account id
    pub fn app_account() -> T::AccountId {
        SINGLE_GOMOKU_ID.into_account()
    }

    /// Submit and settle off-chain state
    ///
    /// Parameter:
    /// `gomoku_info`: Info of gomoku state
    /// `state_proof`: Signed off-chain app state
    fn intend_settle(
        mut gomoku_info: GomokuInfoOf<T>,
        state_proof: StateProofOf<T>
    ) -> Result<GomokuInfoOf<T>, DispatchError> {
        let app_state = state_proof.app_state;
        let encoded = Self::encode_app_state(app_state.clone());
        Self::valid_signers(state_proof.sigs, &encoded, gomoku_info.players.clone())?;
        ensure!(
            gomoku_info.status != AppStatus::Finalized,
            "app state is finalized"
        );
        ensure!(
            app_state.nonce == gomoku_info.nonce,
            "nonce not match"
        );
        ensure!(
            gomoku_info.seq_num < app_state.seq_num,
            "invalid sequence number"
        );

        gomoku_info.seq_num = app_state.seq_num;
        gomoku_info.deadline = frame_system::Module::<T>::block_number() + gomoku_info.timeout;
        gomoku_info.status = AppStatus::Settle;

        Ok(gomoku_info)
    }

    /// Apply an action to the on-chain state
    ///
    /// Parameter:
    /// `gomoku_info`: Info of gomoku state
    fn apply_action(
        mut gomoku_info: GomokuInfoOf<T>
    ) -> Result<GomokuInfoOf<T>, DispatchError> {
        ensure!(
            gomoku_info.status != AppStatus::Finalized,
            "app state is finalized"
        );

        let block_number =  frame_system::Module::<T>::block_number();
        if gomoku_info.status == AppStatus::Settle && block_number > gomoku_info.deadline {
            gomoku_info.seq_num = gomoku_info.seq_num + 1;
            gomoku_info.deadline = block_number + gomoku_info.timeout;
            gomoku_info.status = AppStatus::Action;
        } else {
            ensure!(
                gomoku_info.status ==  AppStatus::Action,
                "app not in action mode"
            );
            gomoku_info.seq_num = gomoku_info.seq_num + 1;
            gomoku_info.deadline = block_number + gomoku_info.timeout;
            gomoku_info.status = AppStatus::Action;
        }

        Ok(gomoku_info)        
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
    ) -> DispatchResult {
        for i in 0..2 {
            ensure!(
                &signatures[i].verify(encoded, &signers[i]),
                "Check co-sigs failed"
            );
        };
        Ok(())
    }

    /// Set game states when there is a winner
    ///
    /// Parameters:
    /// `winner`: Id of winner
    /// `gomoku_info`: Info of gomoku state
    fn win_game(
        winner: u8, 
        mut gomoku_info: GomokuInfoOf<T>,
    ) -> Result<GomokuInfoOf<T>, DispatchError> {
        ensure!(
            u8::min_value() <= winner && winner <= 2,
            "invalid winner state"
        );

        let mut new_board_state = gomoku_info.gomoku_state.board_state.unwrap_or(vec![0; 227]);
        // set winner
        new_board_state[0] = winner;

        if winner != 0 {// Game over
            // set turn 0
            new_board_state[1] = 0; 
            gomoku_info.status = AppStatus::Finalized;
            gomoku_info.gomoku_state.board_state = Some(new_board_state);
        } else {
            gomoku_info.gomoku_state.board_state = Some(new_board_state);
        }
        
        return Ok(gomoku_info);
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
        let index: usize = (2 + board_dimention * x + y) as usize;
        return index;
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
        app_state.board_state.iter()
            .for_each(|state| { encoded.extend(state.encode()); });
        encoded.extend(app_state.timeout.encode());
        encoded.extend(app_state.session_id.encode());

        return encoded;
    }

} 
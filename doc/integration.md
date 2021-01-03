---
title: Integrate runtime module condition into celer-pay runtime module
---

We will now integrate [single-session-app](https://github.com/celer-network/cApps-substrate/tree/master/pallets/single-session-app) into [celer-pay runtime module](https://github.com/celer-network/cChannel-substrate/tree/master/pallets/celer-pay). [single-session-app](https://github.com/celer-network/cApps-substrate/tree/master/pallets/single-session-app) is boolean outcome runtime module condition.

# Open condition-caller rutime module file

1. ```git clone git@github.com:celer-network/cChannel-substrate.git```
2. Go to cChannel-substrate's root directory.
3. ```cd ./pallets/condition-caller```

[condition-caller(module name = runtime-module-condition-caller)](https://github.com/celer-network/cChannel-substrate/tree/master/pallets/condition-caller) is module for registering runtime module condition and call query API. This module is called by celer-pay runtime module.

We will be editing two files: `condition-caller/Cargo.toml`, `condition-caller/src/lib.rs`
```
|
+-- condition-caller [module name = runtime-module-condition-caller]
  |
  +-- src
  | |
  | +-- lib.rs
  |
  +-- Cargo.toml <-- Two change in this file
```

# Imporing a single-session-app runtime module

First, we will now importing single-session-app runtime module in condition-caller `Cargo.toml` file.
single-session-app runtime module is already published in [crates.io](https://crates.io/crates/single-session-app)

Open `condition-caller/Cargo.toml` and edit it.

**`condition-caller/Cargo.toml`**
```TOML
[dependencies]
#--snip--
single-session-app = { version = "0.8.5", default_features = false } <-- add this line

#--snip--
[features]
default=['std']
std = [
    "codec/std",
    #--snip--
    "single-session-app/std", <-- add this line
]
TOML```
```

# Edit source file of condition caller.
Second, we will now add [Trait](https://doc.rust-lang.org/book/ch10-02-traits.html) of single-session-app runtime module to [Trait](https://doc.rust-lang.org/book/ch10-02-traits.html) of condition-caller runtime module.

Open `condition-caller/src/lib.rs` and edit it.

**`condition-caller/src/lib.rs`**
```
pub trait Trait: system::Trait + mock_numeric_condition::Trait + mock_boolean_condition::Trait + single_session_app::Trait {}
// --------------------------------------------------------------------------------------------^^^^^^^^^^^^^^^^^^^^^^^^^^^^
// Add single session app Trait.
```

Third, we will now add call logic of [query function](https://github.com/celer-network/cApps-substrate/blob/master/pallets/single-session-app/src/lib.rs)[line 309~357] and register [registration_num](https://github.com/celer-network/cChannel-substrate/blob/master/pallets/celer-pay/src/pay_resolver.rs)[line 24] of single-session-app runtime module into condition-caller runtime module.

```
    pub fn call_runtime_module_condition(
        registration_num: u32,
        args_query_finalization: Vec<u8>,
        args_query_outcome: Vec<u8>,
    ) -> Result<(bool, Vec<u8>), DispatchError> {
        // In the if block, call query function of your runtime module condition 
        // and return tuple(is_finalized result, encoded boolean or numeic outcome)
        match registration_num {
            0 => { // Register registration_num of your runtime module condition 
                // is_finalized function return bool value
                let is_finalized: bool = match mock_numeric_condition::Module::<T>::is_finalized(args_query_finalization) {
                    Ok(_is_finalized) => _is_finalized,
                    Err(dispatch_error) => return Err(dispatch_error)?,
                };
                // get_outcome function return encoded u32 value
                let outcome: Vec<u8> = match mock_numeric_condition::Module::<T>::get_outcome(args_query_outcome) {
                    Ok(_outcome) => _outcome,
                    Err(dispatch_error) => return Err(dispatch_error)?,
                };
                return Ok((is_finalized, outcome));
            },
            1 => {
                // is_finalized function return bool value
                let is_finalized: bool = match mock_boolean_condition::Module::<T>::is_finalized(args_query_finalization) {
                    Ok(_is_finalized) => _is_finalized,
                    Err(dispatch_error) => return Err(dispatch_error)?,
                };
                // get_outcome function return encoded bool value
                let outcome: Vec<u8> = match mock_boolean_condition::Module::<T>::get_outcome(args_query_outcome) {
                    Ok(_outcome) => _outcome,
                    Err(dispatch_error) => return Err(dispatch_error)?,
                };
                return Ok((is_finalized, outcome));
            },
            2 => {
                // is_finalized function return bool value
                let is_finalized: bool = match single_session_app::Module::<T>::is_finalized(args_query_finalization) {
                    Ok(_is_finalized) => _is_finalized,
                    Err(dispatch_error) => return Err(dispatch_error)?,
                };
                // get_outcome function return encoded bool value
                let outcome: Vec<u8> = match single_session_app::Module::<T>::get_outcome(args_query_outcome) {
                    Ok(_outcome) => _outcome,
                    Err(dispatch_error) => return Err(dispatch_error)?,
                };
                return Ok((is_finalized, outcome));
            }
            _ => return Err(Error::<T>::RuntimeModuleConditionNotRegistered)?,
        }
    }
```

Hack of integration runtime module condition into celer-pay runtime module is All!

# Compile celer-pay runtime module 
Finally, we will now check whether compile is success.

**`condition-calller and celer-pay directory`**

```cargo build --release```

*Integrated souce code is [here](https://github.com/celer-network/cChannel-substrate/tree/integration/single-session-app).

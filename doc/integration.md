---
title: Integrate runtime module condition into celer-pay runtime module
---

We will now integrate [single-session-app](https://github.com/celer-network/cApps-substrate/tree/master/pallets/single-session-app) into [celer-pay runtime module](https://github.com/celer-network/cChannel-substrate/tree/master/pallets/celer-pay). [single-session-app](https://github.com/celer-network/cApps-substrate/tree/master/pallets/single-session-app) is boolean outcome runtime module condition.

# Open celer-pay rutime module file

1. ```git clone git@github.com:celer-network/cChannel-substrate.git```
2. Go to cChannel-substrate's root directory.
3. ```cd ./pallets/celer-pay```

We will be editing three files: `Cargo.toml`, `celer-pay/src/traits.rs`, `celer-pay/src/runtime_module_condition_caller.rs`.

```
|
+-- celer-pay
|
+-- rpc
|
+-- src
| |
| +-- traits.rs <-- One change in this file
| |
| +-- runtime_module_condition_caller.rs <-- Most changes in this file
| |
| +-- ...
|
+-- Cargo.toml <-- Two change in this file
```

# Imporing a single-session-app runtime module

First, we will now importing single-session-app runtime module in celer-pay `Cargo.toml` file.
single-session-app runtime module is already published in [crates.io](https://crates.io/crates/single-session-app)

Open `celer-pay/Cargo.toml` and edit it.

**`celer-pay/Cargo.toml`**
```TOML
[dependencies]
#--snip--
single-session-app = { version = "0.8.5", default_features = false } <-- add this line

#--snip--
[features]
default=['std']
std = [
    "codec/std",
	"serde",
	"sp-io/std",
    #--snip--
    "single-session-app/std", <-- add this line
]
TOML```
```

# Edit traits file 
Second, we will now add [Trait](https://doc.rust-lang.org/book/ch10-02-traits.html) of single-session-app runtime module to [Trait](https://doc.rust-lang.org/book/ch10-02-traits.html) of celer-pay runtime module.

Open `celer-pay/src/traits.rs` and edit it.

**`celer-pay/src/traits.rs`**
```
pub trait Trait: system::Trait + pallet_timestamp::Trait + celer_contracts::Trait 
   + mock_numeric_condition::Trait + mock_boolean_condition::Trait + single_session_app::Trait
// ----------------------------------------------------------------^^^^^^^^^^^^^^^^^^^^^^^^^^^
// Add single-session-app Trait
```

# Edit runtime module condition caller file
Third, we will now add call logic of [query function](https://github.com/celer-network/cApps-substrate/blob/master/pallets/single-session-app/src/lib.rs)[line 309~357] and register [registration_num](https://github.com/celer-network/cChannel-substrate/blob/master/pallets/celer-pay/src/pay_resolver.rs)[line 24] of single-session-app runtime module into `celer-pay/src/runtime_module_condition_caller.rs`.

Open `celer-pay/src/runtime_module_condition_caller.rs` and edit it.

**`celer-pay/src/runtime_module_condition_caller.rs`**
```
    pub fn call_runtime_module_condition(
        registration_num: u32,
        args_query_finalization: Vec<u8>,
        args_query_outcome: Vec<u8>,
    ) -> Result<(bool, Vec<u8>), DispatchError> {
        // In the if block, call query function of your runtime module condition 
        // and return tuple(is_finalized result, encoded boolean or numeic outcome)
        //
        // Register registration_num of your runtime module condition 
        // vvvvvvvvvvvvvvvvvvvvvv----------------
        if registration_num == 0 { // MockNumerocCondition

        #--snip--

        // Add call logic of query function 
        // and Register registration_num of single-session-app runtime module condition 
        } else if registration_num == 2 { // SingleSessionApp
            // is_finalized function return boolean value
            let is_finalized: bool = match single_session_app::Module::<T>::is_finalized(args_query_finalization) {
                Ok(_is_finalized) => _is_finalized,
                Err(dispatch_error) => return Err(dispatch_error)?,
            };
            // get_outcome function return encoded boolean value
            let outcome: Vec<u8> = match single_session_app::Module::<T>::get_outcome(args_query_outcome) {
                Ok(_outcome) => _outcome,
                Err(dispatch_error) => return Err(dispatch_error)?,
            };
            // return tuple(is_finalized result, encoded boolean outcome)
            return Ok((is_finalized, outcome));
        } else {
```

Hack of integration runtime module condition into celer-pay runtime module is All!

# Compile celer-pay runtime module 
Finally, we will now check whether compile is success.

**`celer-pay directory`**

```cargo build --release```

*Integrated souce code is [here](https://github.com/celer-network/cChannel-substrate/tree/integration/single-session-app).

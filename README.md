# Celer App Moodule

CelerApp are highly interactive, secure and low-cost state-channel applications running on 
Substrate together with [Celer Generic payment channel](https://github.com/celer-network/cChannel-substrate).
Generic payment channel is payment channel independent of specifc application.
This repo provides examples for developing the on-chain runtime module parts of dApps. On-chain operations are only needed when players cannot reach consensus off-chain and want to dispute.

[CelerPay](https://github.com/celer-network/cChannel-substrate) and CelerApp are loosely connected through the simple conditional dependency interface. 
CelerApp module by exposing two functions for the CelerPay to use as payment condition: `is_finalized`
returns whether the app state outcome is finalized; `get_outcome` returns the boolean or numeric outcome.
```
// Simple interface of CelerApp with boolean outcome
// If app is finalized, return true value
// else, return false value
// dev: `args_query_finalization` is encoded value and you can take any type as argument.
pub fn is_finalized(args_query_finalization: Vec<u8>) -> Result<bool, DispatchError> {}

// If app outcome is true, return encode true value
// else, return encoded false value
// dev: `args_query_outcome` is encoded value and you can take any type as argument.
pub fn get_outcome(args_query_outcome: Vec<u8>) -> Result<Vec<u8>, DispatchError> {}

// Simple interface of CelerApp with numeric outcome 
// If app is finalized, return true value
// else, return false value
// dev: args_query_finalization is encoded value and you can take any type as an argument.
pub fn is_finalized(args_query_finalization: Vec<u8>) -> Result<bool, DispatchError> {}

// return encoded u32 value
// dev: args_query_outcome is encoded value and you can take any type as an argument.
pub fn get_outcome(args_query_outcome: Vec<u8>) -> Result<Vec<u8>, DispatchError> {}
```

You can implement CelerApp with Substrate runtime module or smart contract.

|  | boolean & newmeric outcome runtime module | boolean & numeric outcome smart contract |
| ----------|---------- | ---------------|
| deploy option | forkless runtime upgrade | forkless runtime upgrade or virtual contract |
|Ease of runtime upgrade or deploy| hard |　easy |
|Ease of development| medium |  easy | 
|Level of customization | high |　low |

- [forkless runtime upgrade](https://substrate.dev/docs/en/tutorials/upgrade-a-chain/): forkless runtime upgrade once by the developer and can be repeatedly shared by all players. No additional code needs to be deployed or runtime upgrade when players want to dispute on-chain. 

- virtual contract: The contract can also stay off-chain as a virtual counterfactually instantiated by involved parties. A virtual contract only needs to be deployed only needs to be deployed on-chain if someone wants to dispute, in which case CelerPay can find where to call the `is_finalized` and `get_outcome`APIs through a unique identifier computed by the hash of the virtual contract code, initial states, and a nonce.

*Smart contract will support future.



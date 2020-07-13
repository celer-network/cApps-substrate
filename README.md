# Celer App Moodule

CelerApp are highly interactive, secure and low-cost state-channel applications running on 
Substrate together with Celer [Generic payment channel](https://github.com/celer-network/cChannel-substrate).
Generic payment channel is payment channel independent of specifc application.

[CelerPay](https://github.com/celer-network/cChannel-substrate) and CelerApp are loosely connected through the simple conditional dependency interface. 
CelerApp module by exposing two functions for the CelerPay to use as payment condition: `is_finalized`
returns whether the app state outcome is finalized; `get_outcome` returns the boolean or numeric outcome.
```
// Simple interface of CelerApp with boolean outcome
// If app is not finalized, return DispatchError::Other("NotFinalized")
// else, return Ok(())
pub fn is_finalized(origin, _app_id: T::Hash) -> DispatchResult {}

// If app outcome is false, return DispatchError::Other("FalseOutcome")
// else, return Ok(()) 
pub fn get_outcome(origin, _app_id: T::Hash) -> DispatchResult {}

// Simple interface of CelerApp with numeric outcome 
// If app is not finalized, return false
// else, return true
// dev: query is encoded value and you can take any type as an argument.
pub fn is_finalized(_app_id: T::Hash, query: Option<Vec<u8>>) -> Result<bool, DispatchError> {}

// return u32 amount
// dev: query is encoded value and you can take any type as an argument.
pub fn get_outcome(_app_id: T::Hash, query: Option<Vec<u8>>) -> Result<u32, DispatchError> {}
```

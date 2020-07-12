# Celer App Moodule

CelerApp are highly interactive, secure and low-cost state-channel applications running on 
Substrate together with Celer [Generic payment channel](https://github.com/celer-network/cChannel-substrate).
Generic payment channel is payment channel independent of specifc application.

[CelerPay](https://github.com/celer-network/cChannel-substrate) and CelerApp are loosely connected through the simple conditional dependency interface. 
CelerApp module by exposing two functions for the CelerPay to use as payment condition: `is_finalized`
returns whether the app state outcome is finalized; `get_outcome` returns the boolean or numeric outcome.
```
// Simple example of CelerApp with boolean outcome
pub fn is_finalized(origin, _app_id: T::Hash) -> DispatchResult {
    // If app is not finalized, return DispatchError::Other("NotFinalized")
    ensure!(
       app_info.status == AppStatus::Finalized,
       "NotFinalized"
    );
    
    // If app is finalized, return Ok(())
    Ok(())
}

pub fn get_outcome(origin, _app_id: T::Hash) -> DispatchResult {
    // If outcome is false, return DispatchError::Other("FalseOutcome")
    ensure!(
        app_info.state == query,
        "FalseOutcome"
    );
    
    // If outcome is true, return Ok(())
    Ok(())
}

// Simple example of CelerApp with numeric outcome 
pub fn is_finalized(_app_id: T::Hash, query: Option<Vec<u8>>) -> Result<bool, DispatchError> {
     let _query = query.unwrap();
     let number: u8 = Decode::decode(&mut &_query[..]).map_err(|_| Error::<T>::MustBeDecodable)?;
     if number == 0 { 
         // If app is not finalized, return false
         return Ok(false);
     } else {
        // If app is finalized, return true
         return Ok(true);
     }
}

pub fn get_outcome(_app_id: T::Hash, query: Option<Vec<u8>>) -> Result<u32, DispatchError> {
    let _query = query.unwrap();
    let amount: u32 = Decode::decode(&mut &_query[..]).map_err(|_| Error::<T>::MustBeDecodable)?;
    // return u32 value 
    return Ok(amount);
}
```

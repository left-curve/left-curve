mod batch_update_orders;
mod cancel_conditional_order;
mod cancel_order;
mod deposit;
mod submit_conditional_order;
mod submit_order;
mod withdraw;

pub use {
    batch_update_orders::*, cancel_conditional_order::*, cancel_order::*, deposit::*,
    submit_conditional_order::*, submit_order::*, withdraw::*,
};

use {
    crate::USER_STATES,
    dango_types::perps::UserState,
    grug::{Addr, StdResult, Storage},
};

/// 1. Load the user's state.
/// 2. Perform a mutable action on the user state. The action may have side
///    effect on the storage.
/// 3. If the user state becomes empty, delete it from storage; otherwise, save
///    the updated user state to storage.
fn update_user_state_with<F, T>(storage: &mut dyn Storage, user: Addr, action: F) -> StdResult<T>
where
    F: FnOnce(&mut dyn Storage, &mut UserState) -> StdResult<T>,
{
    let mut user_state = USER_STATES.load(storage, user)?;

    let result = action(storage, &mut user_state)?;

    if user_state.is_empty() {
        USER_STATES.remove(storage, user)?;
    } else {
        USER_STATES.save(storage, user, &user_state)?;
    }

    Ok(result)
}

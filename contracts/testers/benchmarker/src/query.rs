use grug::{Empty, Number, StdResult, Uint128};

// Function needs to be named `do_loop` instead of `loop`, because the latter is
// a reserved Rust keyword.
pub fn do_loop(iterations: u64) -> StdResult<Empty> {
    // Keep the same operation per iteration for consistency
    for _ in 0..iterations {
        let number = Uint128::new(100);
        number.checked_add(number)?;
        number.checked_sub(number)?;
        number.checked_mul(number)?;
        number.checked_div(number)?;
        number.checked_pow(2)?;
    }

    Ok(Empty {})
}

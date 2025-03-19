use {
    crate::{PUBLIC_KEY, SEQUENCE, StateResponse},
    grug_types::{StdResult, Storage},
};

pub fn query_state(storage: &dyn Storage) -> StdResult<StateResponse> {
    Ok(StateResponse {
        public_key: PUBLIC_KEY.load(storage)?,
        sequence: SEQUENCE.current(storage)?,
    })
}

use crate::Storage;

pub struct ExecuteCtx<'a> {
    pub store: &'a mut dyn Storage,
    // TODO: other fields...
}

pub struct QueryCtx<'a> {
    pub store: &'a dyn Storage,
}

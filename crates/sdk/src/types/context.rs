use crate::Storage;

pub struct ExecuteCtx<'a> {
    pub store: &'a mut dyn Storage,
    // TODO: other fields...
}

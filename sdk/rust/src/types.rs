use cw_std::Addr;

pub enum AdminOption {
    SetToAddr(Addr),
    SetToSelf,
    SetToNone,
}

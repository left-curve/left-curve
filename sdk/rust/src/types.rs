use grug_types::Addr;

pub enum AdminOption {
    SetToAddr(Addr),
    SetToSelf,
    SetToNone,
}

impl AdminOption {
    // TODO: make the self_addr lazy (only compute it if SelfToSelf)
    pub fn decide(self, self_addr: &Addr) -> Option<Addr> {
        match self {
            AdminOption::SetToAddr(addr) => Some(addr),
            AdminOption::SetToSelf => Some(self_addr.clone()),
            AdminOption::SetToNone => None,
        }
    }
}

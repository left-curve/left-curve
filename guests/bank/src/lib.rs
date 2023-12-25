use sdk::{Region, Storage};

#[no_mangle]
pub extern "C" fn send(from_ptr: usize, to_ptr: usize, amount: u64) {
    let mut balances = Balances::new(Storage::new());

    let from = unsafe { Region::consume(from_ptr as *mut Region) };
    balances.decrease(&from, amount);

    let to = unsafe { Region::consume(to_ptr as *mut Region) };
    balances.increase(&to, amount);
}

struct Balances {
    store: Storage,
}

impl Balances {
    pub fn new(store: Storage) -> Self {
        Self {
            store,
        }
    }

    pub fn increase(&mut self, address: &[u8], amount: u64) {
        let mut balance = self.get(address);

        balance = balance.checked_add(amount).unwrap_or_else(|| {
            panic!("Excessive balance: {balance} + {amount} > u64::MAX");
        });

        self.set(address, balance);
    }

    pub fn decrease(&mut self, address: &[u8], amount: u64) {
        let mut balance = self.get(address);

        balance = balance.checked_sub(amount).unwrap_or_else(|| {
            panic!("Insufficient balance: {balance} < {amount}");
        });

        if balance > 0 {
            self.set(address, balance);
        } else {
            self.store.remove(address);
        }
    }

    fn get(&self, address: &[u8]) -> u64 {
        self.store
            .read(address)
            .map(|bytes| {
                let bytes: [u8; 8] = bytes.try_into().unwrap_or_else(|_| {
                    panic!("Failed to parse balance into u64");
                });

                u64::from_be_bytes(bytes)
            })
            .unwrap_or(0)
    }

    fn set(&mut self, address: &[u8], balance: u64) {
        self.store.write(address, &balance.to_be_bytes());
    }
}

use {
    grug_types::{Order, Record, Storage, extend_one_byte},
    std::vec,
};

pub struct Iterator {
    batch: Option<vec::IntoIter<Record>>,
    min: Option<Vec<u8>>,
    max: Option<Vec<u8>>,
    order: Order,
}

impl Iterator {
    const BATCH_SIZE: usize = 30;

    pub fn new(min: Option<Vec<u8>>, max: Option<Vec<u8>>, order: Order) -> Self {
        Self {
            batch: None,
            min,
            max,
            order,
        }
    }

    pub fn next(&mut self, storage: &dyn Storage) -> Option<Record> {
        if let Some(record) = self.batch.as_mut().and_then(|batch| batch.next()) {
            return Some(record);
        }

        let batch = storage
            .scan(self.min.as_deref(), self.max.as_deref(), self.order)
            .take(Self::BATCH_SIZE)
            .collect::<Vec<_>>();

        if let Some((key, _)) = batch.iter().last() {
            match self.order {
                Order::Ascending => self.min = Some(extend_one_byte(key.clone())),
                Order::Descending => self.max = Some(key.clone()),
            }
        }

        self.batch = Some(batch.into_iter());
        self.batch.as_mut().unwrap().next()
    }
}

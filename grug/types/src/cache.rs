use {
    crate::StdError,
    std::{
        collections::{HashMap, hash_map::Entry},
        hash::Hash,
    },
};

pub struct Cache<'a, K, V, Err = StdError, Aux = ()>
where
    K: Eq + Hash + Clone,
{
    data: HashMap<K, V>,
    fetcher: Box<dyn Fn(&K, Option<Aux>) -> Result<V, Err> + 'a>,
}

impl<'a, K, V, Err, Aux> Cache<'a, K, V, Err, Aux>
where
    K: Eq + Hash + Clone,
{
    pub fn new<F>(fetcher: F) -> Self
    where
        F: Fn(&K, Option<Aux>) -> Result<V, Err> + 'a,
    {
        Self {
            data: HashMap::new(),
            fetcher: Box::new(fetcher),
        }
    }

    pub fn get_or_fetch(&mut self, k: &K, aux: Option<Aux>) -> Result<&V, Err> {
        match self.data.entry(k.clone()) {
            Entry::Occupied(entry) => Ok(entry.into_mut()),
            Entry::Vacant(entry) => {
                let v = (self.fetcher)(k, aux)?;
                Ok(entry.insert(v))
            },
        }
    }
}

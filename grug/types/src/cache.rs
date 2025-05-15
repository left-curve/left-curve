use {
    crate::StdError,
    std::{collections::HashMap, hash::Hash},
};

pub struct Cache<'a, K, V, Err = StdError, Aux = ()>
where
    K: Eq + Hash + Clone,
{
    data: HashMap<K, V>,
    fetcher: Box<dyn Fn(K, Aux) -> Result<V, Err> + 'a>,
}

impl<'a, K, V, Err, Aux> Cache<'a, K, V, Err, Aux>
where
    K: Eq + Hash + Clone,
{
    pub fn new<F>(fetcher: F) -> Self
    where
        F: Fn(K, Aux) -> Result<V, Err> + 'a,
    {
        Self {
            data: HashMap::new(),
            fetcher: Box::new(fetcher),
        }
    }

    pub fn get_or_fetch(&mut self, k: &K, aux: Aux) -> Result<&V, Err> {
        if !self.data.contains_key(k) {
            let v = (self.fetcher)(k.clone(), aux)?;
            self.data.insert(k.clone(), v);
        }

        Ok(self.data.get(k).unwrap())
    }
}

use std::{
    ops::Deref,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use grug_types::Order;

use {
    grug_types::{Buffer, MockStorage, Storage},
    ouroboros::self_referencing,
};

pub struct VmProvider<'a> {
    pub storage: &'a mut dyn Storage,
}

impl<'a> VmProvider<'a> {
    pub fn new(storage: &'a mut dyn Storage) -> Self {
        Self { storage }
    }

    pub fn split(&self) {}
}

pub struct S<'a> {
    storage: Arc<RwLock<&'a mut dyn Storage>>,
}

pub struct Asd {
    inner: Arc<RwLock<MockStorage>>,
}

impl Asd {
    fn scan<'a>(
        &'a self,
        min: Option<&[u8]>,
        max: Option<&[u8]>,
        order: grug_types::Order,
    ) -> Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'a> {
        let guard = self.inner.read().unwrap();
        Box::new(ScanIter::new(guard, |a| a.scan(min, max, order)))
    }
}

#[self_referencing]
pub struct ScanIter<'a> {
    guard: RwLockReadGuard<'a, MockStorage>,
    #[borrows(guard)]
    #[covariant] // <— DICHIARA CHE QUESTO CAMPO È COVARIANTE SU 'this
    inner: Box<dyn Iterator<Item = (Vec<u8>, Vec<u8>)> + 'this>,
}

// impl<'a> ScanIter<'a> {
//     pub fn new(
//         guard: RwLockReadGuard<'a, MockStorage>,
//         min: Option<&[u8]>,
//         max: Option<&[u8]>,
//         order: Order,
//     ) -> Self {
//         ScanIterBuilder {
//             guard,
//             inner_builder: |g: &MockStorage| {
//                 // Boxiamo l'iteratore che borrows da g
//                 Box::new(g.scan(min, max, order))
//             },
//         }
//         .build()
//     }

//     // comodo: esponi l’iteratore
//     pub fn iter<'t>(&'t mut self) -> impl Iterator<Item = (Vec<u8>, Vec<u8>)> + 't {
//         // access_inner_mut().by_ref() ti dà &mut (dyn Iterator + 'this)
//         self.with_inner_mut(|it| it.by_ref().map(|x| x))
//     }
// }

impl<'a> Iterator for ScanIter<'a> {
    type Item = (Vec<u8>, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        self.with_inner_mut(|it| it.next())
    }
}

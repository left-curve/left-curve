use {
    grug_storage::{Index, IndexList, IndexedMap, Item, Map, MultiIndex, Set},
    grug_types::{
        Addr, BlockInfo, Code, CodeStatus, Config, ContractInfo, Hash256, Json, Timestamp,
    },
};

/// A string that identifies the chain
pub const CHAIN_ID: Item<String> = Item::new("chain_id");

/// Chain-level configuration
pub const CONFIG: Item<Config> = Item::new("config");

/// Application-specific configurations.
pub const APP_CONFIGS: Map<&str, Json> = Map::new("app_config");

/// The most recently finalized block
pub const LAST_FINALIZED_BLOCK: Item<BlockInfo> = Item::new("last_finalized_block");

/// Scheduled cronjobs.
///
/// This needs to be a `Set` instead of `Map<Timestamp, Addr>` because there can
/// be multiple jobs with the same scheduled time.
pub const NEXT_CRONJOBS: Set<(Timestamp, Addr)> = Set::new("jobs");

/// Wasm contract byte codes: code_hash => byte_code
pub const CODES: IndexedMap<Hash256, Code, CodeIndexes> = IndexedMap::new("codes", CodeIndexes {
    status: MultiIndex::new(|_, c| c.status, "codes", "codes_status"),
});

/// Contract metadata: address => contract_info
pub const CONTRACTS: Map<Addr, ContractInfo> = Map::new("contract");

/// Each contract has its own storage space, which we term the "substore".
/// A key in a contract's substore is prefixed by the word "wasm" + contract address.
pub const CONTRACT_NAMESPACE: &[u8] = b"wasm";

pub struct CodeIndexes<'a> {
    status: MultiIndex<'a, Hash256, CodeStatus, Code>,
}

impl IndexList<Hash256, Code> for CodeIndexes<'_> {
    fn get_indexes(
        &self,
    ) -> Box<dyn Iterator<Item = &'_ dyn grug_storage::Index<Hash256, Code>> + '_> {
        let v: Vec<&dyn Index<Hash256, Code>> = vec![&self.status];
        Box::new(v.into_iter())
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::CODES,
        grug_types::{Binary, Code, CodeStatus, CodeStatusType, Hash256, MockStorage, StdResult},
    };

    #[test]
    fn codes() {
        let mut storage = MockStorage::new();
        CODES
            .save(&mut storage, Hash256::from_inner([0; 32]), &Code {
                code: Binary::from_inner(vec![0; 32]),
                status: CodeStatus::Orphan { since: 100 },
            })
            .unwrap();

        CODES
            .save(&mut storage, Hash256::from_inner([1; 32]), &Code {
                code: Binary::from_inner(vec![1; 32]),
                status: CodeStatus::Orphan { since: 20 },
            })
            .unwrap();

        CODES
            .save(&mut storage, Hash256::from_inner([2; 32]), &Code {
                code: Binary::from_inner(vec![2; 32]),
                status: CodeStatus::Amount { amount: 2 },
            })
            .unwrap();

        let bound = grug_types::Bound::Inclusive((100, Hash256::from_inner([0; 32])));

        let res: Vec<_> = CODES
            .idx
            .status
            .sub_prefix(CodeStatusType::Orphan)
            .range(&storage, Some(bound), None, grug_types::Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        assert_eq!(res, vec![(Hash256::from_inner([0; 32]), Code {
            code: Binary::from_inner(vec![0; 32]),
            status: CodeStatus::Orphan { since: 100 },
        })]);

        let bound = grug_types::Bound::Inclusive((15, Hash256::from_inner([0; 32])));

        let res: Vec<_> = CODES
            .idx
            .status
            .sub_prefix(CodeStatusType::Orphan)
            .range(&storage, Some(bound), None, grug_types::Order::Ascending)
            .collect::<StdResult<Vec<_>>>()
            .unwrap();

        assert_eq!(res, vec![
            (Hash256::from_inner([1; 32]), Code {
                code: Binary::from_inner(vec![1; 32]),
                status: CodeStatus::Orphan { since: 20 },
            }),
            (Hash256::from_inner([0; 32]), Code {
                code: Binary::from_inner(vec![0; 32]),
                status: CodeStatus::Orphan { since: 100 },
            }),
        ]);
    }
}

use {
    crate::AdminOption,
    anyhow::bail,
    chrono::{DateTime, SecondsFormat, Utc},
    grug_types::{
        from_json_slice, hash, to_json_value, Addr, Binary, Coins, Config, Duration, GenesisState,
        Hash, Json, Message, Permission, Permissions, StdError, TSBInit, TSBUnset, GENESIS_SENDER,
    },
    serde::Serialize,
    std::{collections::BTreeMap, fs, path::Path},
};

#[derive(Default)]
pub struct GenesisBuilder<BA, TA, UP, IP> {
    // Consensus parameters
    genesis_time: Option<DateTime<Utc>>,
    chain_id: Option<String>,
    // Chain configs
    owner: Option<Addr>,
    bank: BA,
    taxman: TA,
    cronjobs: BTreeMap<Addr, Duration>,
    upload_permission: UP,
    instantiate_permission: IP,
    // Genesis messages
    upload_msgs: Vec<Message>,
    other_msgs: Vec<Message>,
}

impl GenesisBuilder<TSBUnset<Addr>, TSBUnset<Addr>, TSBUnset<Permission>, TSBUnset<Permission>> {
    pub fn new() -> Self {
        GenesisBuilder::default()
    }
}
impl<BA, TA, UP, IP> GenesisBuilder<BA, TA, UP, IP> {
    pub fn with_genesis_time<T>(mut self, genesis_time: T) -> Self
    where
        T: Into<DateTime<Utc>>,
    {
        self.genesis_time = Some(genesis_time.into());
        self
    }

    pub fn with_chain_id<T>(mut self, chain_id: T) -> Self
    where
        T: ToString,
    {
        self.chain_id = Some(chain_id.to_string());
        self
    }

    pub fn set_owner(mut self, owner: Addr) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn with_upload_permission(
        self,
        permission: Permission,
    ) -> GenesisBuilder<BA, TA, TSBInit<Permission>, IP> {
        GenesisBuilder {
            genesis_time: self.genesis_time,
            chain_id: self.chain_id,
            owner: self.owner,
            bank: self.bank,
            taxman: self.taxman,
            cronjobs: self.cronjobs,
            upload_permission: TSBInit(permission),
            instantiate_permission: self.instantiate_permission,
            upload_msgs: self.upload_msgs,
            other_msgs: self.other_msgs,
        }
    }

    pub fn with_instantiate_permission(
        self,
        permission: Permission,
    ) -> GenesisBuilder<BA, TA, UP, TSBInit<Permission>> {
        GenesisBuilder {
            genesis_time: self.genesis_time,
            chain_id: self.chain_id,
            owner: self.owner,
            bank: self.bank,
            taxman: self.taxman,
            cronjobs: self.cronjobs,
            instantiate_permission: TSBInit(permission),
            upload_permission: self.upload_permission,
            upload_msgs: self.upload_msgs,
            other_msgs: self.other_msgs,
        }
    }

    pub fn set_bank(self, bank: Addr) -> GenesisBuilder<TSBInit<Addr>, TA, UP, IP> {
        GenesisBuilder {
            genesis_time: self.genesis_time,
            chain_id: self.chain_id,
            owner: self.owner,
            bank: TSBInit(bank),
            taxman: self.taxman,
            cronjobs: self.cronjobs,
            instantiate_permission: self.instantiate_permission,
            upload_permission: self.upload_permission,
            upload_msgs: self.upload_msgs,
            other_msgs: self.other_msgs,
        }
    }

    pub fn set_taxman(self, taxman: Addr) -> GenesisBuilder<BA, TSBInit<Addr>, UP, IP> {
        GenesisBuilder {
            genesis_time: self.genesis_time,
            chain_id: self.chain_id,
            owner: self.owner,
            bank: self.bank,
            taxman: TSBInit(taxman),
            cronjobs: self.cronjobs,
            instantiate_permission: self.instantiate_permission,
            upload_permission: self.upload_permission,
            upload_msgs: self.upload_msgs,
            other_msgs: self.other_msgs,
        }
    }

    pub fn add_cronjob(mut self, contract: Addr, interval: Duration) -> Self {
        self.cronjobs.insert(contract, interval);
        self
    }

    pub fn upload<P>(&mut self, path: P) -> anyhow::Result<Hash>
    where
        P: AsRef<Path>,
    {
        let code = fs::read(path)?;
        let code_hash = hash(&code);

        self.upload_msgs.push(Message::upload(code));

        Ok(code_hash)
    }

    pub fn instantiate<M, S, C>(
        &mut self,
        code_hash: Hash,
        msg: &M,
        salt: S,
        funds: C,
        admin_opt: AdminOption,
    ) -> anyhow::Result<Addr>
    where
        M: Serialize,
        S: Into<Binary>,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        let salt = salt.into();
        let address = Addr::compute(&GENESIS_SENDER, &code_hash, &salt);
        let admin = admin_opt.decide(&address);

        let msg = Message::instantiate(code_hash, msg, salt, funds, admin)?;
        self.other_msgs.push(msg);

        Ok(address)
    }

    pub fn upload_and_instantiate<P, M, S, C>(
        &mut self,
        path: P,
        msg: &M,
        salt: S,
        funds: C,
        admin_opt: AdminOption,
    ) -> anyhow::Result<Addr>
    where
        P: AsRef<Path>,
        M: Serialize,
        S: Into<Binary>,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        let code_hash = self.upload(path)?;
        self.instantiate(code_hash, msg, salt, funds, admin_opt)
    }

    pub fn execute<M, C>(&mut self, contract: Addr, msg: &M, funds: C) -> anyhow::Result<()>
    where
        M: Serialize,
        C: TryInto<Coins>,
        StdError: From<C::Error>,
    {
        let msg = Message::execute(contract, msg, funds)?;
        self.other_msgs.push(msg);

        Ok(())
    }
}

impl GenesisBuilder<TSBInit<Addr>, TSBInit<Addr>, TSBInit<Permission>, TSBInit<Permission>> {
    pub fn write_to_cometbft_genesis<P>(self, path: P) -> anyhow::Result<()>
    where
        P: AsRef<Path>,
    {
        let cometbft_genesis_raw = fs::read(path.as_ref())?;
        let mut cometbft_genesis: Json = from_json_slice(cometbft_genesis_raw)?;

        let Some(obj) = cometbft_genesis.as_object_mut() else {
            bail!("CometBFT genesis file is not a JSON object");
        };

        if let Some(genesis_time) = self.genesis_time {
            let genesis_time_str = genesis_time.to_rfc3339_opts(SecondsFormat::Nanos, true);
            obj.insert("genesis_time".to_string(), Json::String(genesis_time_str));
        }

        if let Some(chain_id) = self.chain_id {
            obj.insert("chain_id".to_string(), Json::String(chain_id));
        }

        let permissions = Permissions {
            upload: self.upload_permission.0,
            instantiate: self.instantiate_permission.0,
        };

        let config = Config {
            owner: self.owner,
            bank: self.bank.0,
            taxman: self.taxman.0,
            cronjobs: self.cronjobs,
            permissions,
        };

        let mut msgs = self.upload_msgs;
        msgs.extend(self.other_msgs);

        let genesis_state = GenesisState { config, msgs };
        let genesis_state_json = to_json_value(&genesis_state)?;

        obj.insert("app_state".to_string(), genesis_state_json);

        let cometbft_genesis_raw = serde_json::to_string_pretty(&cometbft_genesis)?;

        fs::write(path, cometbft_genesis_raw)?;

        Ok(())
    }
}

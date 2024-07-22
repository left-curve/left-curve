use {
    crate::AdminOption,
    anyhow::bail,
    chrono::{DateTime, SecondsFormat, Utc},
    grug_types::{
        from_json_slice, hash, to_json_value, Addr, Binary, Coins, Config, Duration, GenesisState,
        Hash, Json, Message, Permission, Permissions, StdError, GENESIS_SENDER,
    },
    serde::Serialize,
    std::{collections::BTreeMap, fs, path::Path},
};

#[derive(Default)]
pub struct GenesisBuilder {
    // Consensus parameters
    genesis_time: Option<DateTime<Utc>>,
    chain_id: Option<String>,
    // Chain configs
    owner: Option<Addr>,
    bank: Option<Addr>,
    cronjobs: BTreeMap<Addr, Duration>,
    upload_permission: Option<Permission>,
    instantiate_permission: Option<Permission>,
    // Genesis messages
    upload_msgs: Vec<Message>,
    other_msgs: Vec<Message>,
}

impl GenesisBuilder {
    pub fn new() -> Self {
        Self::default()
    }

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

    pub fn with_upload_permission(mut self, permission: Permission) -> Self {
        self.upload_permission = Some(permission);
        self
    }

    pub fn with_instantiate_permission(mut self, permission: Permission) -> Self {
        self.instantiate_permission = Some(permission);
        self
    }

    pub fn set_owner(mut self, owner: Addr) -> Self {
        self.owner = Some(owner);
        self
    }

    pub fn set_bank(mut self, bank: Addr) -> Self {
        self.bank = Some(bank);
        self
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

        let Some(owner) = self.owner else {
            bail!("owner address isn't set");
        };

        let Some(bank) = self.bank else {
            bail!("bank address isn't set");
        };

        let Some(upload_permission) = self.upload_permission else {
            bail!("upload permission isn't set");
        };

        let Some(instantiate_permission) = self.instantiate_permission else {
            bail!("instantiate permission isn't set");
        };

        let permissions = Permissions {
            upload: upload_permission,
            instantiate: instantiate_permission,
        };

        let config = Config {
            owner,
            bank,
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

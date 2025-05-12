use {
    dango_genesis::Contracts,
    dango_types::{gateway::Domain, warp::TokenMessage},
    grug::{Addr, Coins, Hash256, ResultExt, Signer, TestSuite, Uint128},
    grug_app::{AppError, Db, Indexer, ProposalPreparer, Vm},
    hyperlane_testing::{MockValidatorSets, constants::MOCK_HYPERLANE_LOCAL_DOMAIN},
    hyperlane_types::{Addr32, mailbox},
    std::ops::{Deref, DerefMut},
};

pub struct HyperlaneTestSuite<DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    suite: TestSuite<DB, VM, PP, ID>,
    validator_sets: MockValidatorSets,
    mailbox: Addr,
    warp: Addr,
}

impl<DB, VM, PP, ID> Deref for HyperlaneTestSuite<DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    type Target = TestSuite<DB, VM, PP, ID>;

    fn deref(&self) -> &Self::Target {
        &self.suite
    }
}

impl<DB, VM, PP, ID> DerefMut for HyperlaneTestSuite<DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.suite
    }
}

impl<DB, VM, PP, ID> HyperlaneTestSuite<DB, VM, PP, ID>
where
    DB: Db,
    VM: Vm + Clone + 'static,
    PP: ProposalPreparer,
    ID: Indexer,
    AppError: From<DB::Error> + From<VM::Error> + From<PP::Error> + From<ID::Error>,
{
    pub fn new(
        suite: TestSuite<DB, VM, PP, ID>,
        validator_sets: MockValidatorSets,
        contracts: &Contracts,
    ) -> Self {
        Self {
            suite,
            validator_sets,
            mailbox: contracts.hyperlane.mailbox,
            warp: contracts.warp,
        }
    }

    pub fn receive_warp_transfer(
        &mut self,
        relayer: &mut dyn Signer,
        origin_domain: Domain,
        origin_warp: Addr32,
        recipient: Addr,
        amount: Uint128,
    ) -> Hash256 {
        // Mock validator set signs the message.
        let (message_id, raw_message, raw_metadata) = self.validator_sets.get(origin_domain).sign(
            origin_warp,
            MOCK_HYPERLANE_LOCAL_DOMAIN,
            self.warp,
            TokenMessage {
                recipient: recipient.into(),
                amount,
                metadata: Default::default(),
            }
            .encode(),
        );

        // Deliver the message to Dango mailbox.
        self.suite
            .execute(
                relayer,
                self.mailbox,
                &mailbox::ExecuteMsg::Process {
                    raw_message,
                    raw_metadata,
                },
                Coins::new(),
            )
            .should_succeed();

        // Return the message ID.
        message_id
    }
}

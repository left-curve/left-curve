# Chain upgrades

There are three dimensions in which to evaluate whether a chain upgrade is a **breaking change**:

- **Consensus-breaking**: a change in the chain's business logic. Given the same finanlized state as of block `N - 1` and the same block `N`, executing the block `N` using the old and the new software would yield different results, resulting in a consensus failure.
- **State-breaking**: a change in the format in which the chain's state is stored in the DB.
- **API-breaking**: a change in the chain's transaction or query API.

For example, PR [#1217](https://github.com/left-curve/left-curve/pull/1217) is breaking in all three dimensions; PR [#1299](https://github.com/left-curve/left-curve/pull/1299) however, is state-breaking, but not consensus- or API-breaking.

Generally speaking, an upgrade that is breaking in any dimension requires a **coordinated upgrade**, meaning all _validating nodes_ should halt _at exactly the same block height_, upgrade the software, run the upgrade logic (if any), and resume block production.

## Coordinated upgrade

The typical procedure of a coordinated upgrade is as follows, in chronological order:

1. The chain owner sends a transaction containing the a message in the following schema:

   ```json
   {
     "upgrade": {
       "height": 12345,
       "cargo_version": "1.2.3",
       "git_tag": "v1.2.3",
       "url": "https://github.com/left-curve/left-curve/releases/v1.2.3"
     }
   }
   ```

   This signals to node operators at which block the chain will be upgraded, and the proper version of node software they should upgrade to. _The node operators should not upgrade the software at this point yet._

2. The chain finalizes the block right before the upgrade height (`12344` in this example). At the upgrade height (`12345`), during `FinalizeBlock`, Grug app notices the upgrade height is reached, but the chain isn't using the correct version (`1.2.3`), so it performs a graceful halt of the chain by retuning an error in ABCI `FinalizeBlockResponse`. The upgrade height (`12345`) is not finalized, with no state change committed.

3. The node operator replaces the node software on the server with the correct version (`1.2.3`), and restart the service.

4. CometBFT attempts `FinalizeBlock` of the upgrade height (`12345`) again. Grug app notices the upgrade height is reached, and the software is of the correct version. It runs the upgrade logic specified in `App::upgrade_handler` (if any), and then resumes processing blocks.

## Automation

Cosmos SDK chains uses a similar approach to coordinate upgrades, with the [`x/upgrade`](https://docs.cosmos.network/v0.53/build/modules/upgrade) module. There exists a tool, [cosmovisor](https://docs.cosmos.network/main/build/tooling/cosmovisor), that automates the step (3) discussed in the previous section, without the node operator having to manually do anything. Such a tool doesn't exist for Grug chains yet, but we're working on it.

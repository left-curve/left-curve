# Networks

Dango mainnet, testnets, and devnets.

## How to spin up a devnet

### Prerequisites

- Linux (we use Ubuntu 24.04)
- Docker
- Rust 1.80+
- Go

### Steps

1. Compile dango:

   ```bash
   git clone https://github.com/left-curve/left-curve.git
   cd left-curve
   cargo install --path dango/cli
   dango --version
   ```

2. Compile cometbft:

   ```bash
   git clone https://github.com/cometbft/cometbft.git
   cd cometbft
   make install
   cometbft version
   ```

3. Initialize the `~/.dango` directory:

   ```bash
   dango init
   ```

4. Initialize the `~/.cometbft` directory:

   ```bash
   cometbft init
   ```

5. Create genesis state. Provide chain ID and genesis time as positional arguments:

   ```bash
   cd left-curve
   cargo run -p dango-genesis --example build_genesis -- dev-5 2025-02-25T21:00:00Z
   ```

   Genesis should be written into `~/.cometbft/config/genesis.json`

6. Create systemd service for postgresql:

   ```ini
   [Unit]
   Description=PostgreSQL
   After=network.target

   [Service]
   Type=simple
   User=larry
   Group=docker
   WorkingDirectory=/home/larry/workspace/left-curve/indexer
   ExecStart=/usr/bin/docker compose up db
   ExecStop=/usr/bin/docker compose down db

   [Install]
   WantedBy=multi-user.target
   ```

   Save this as `/etc/systemd/system/postgresql.service`.

   **Notes:**

   - `WorkingDirectory` should be the directory where the `docker-compose.yml` is located.
   - The `User` should be added to the `docker` group:

     ```bash
     sudo usermod -aG docker larry
     ```

7. Create systemd service for dango:

   ```ini
   [Unit]
   Description=Dango
   After=network.target

   [Service]
   Type=simple
   User=larry
   ExecStart=/home/larry/.cargo/bin/dango start

   [Install]
   WantedBy=multi-user.target
   ```

   Save this as `/etc/systemd/system/dango.service`.

8. Create systemd service for cometbft:

   ```ini
   [Unit]
   Description=CometBFT
   After=network.target

   [Service]
   Type=simple
   User=larry
   ExecStart=/home/larry/.go/bin/cometbft start

   [Install]
   WantedBy=multi-user.target
   ```

   Save this as `/etc/systemd/system/cometbft.service`.

9. Refresh systemd:

   ```bash
   sudo systemctl daemon-reload
   ```

10. Start postgresql:

    ```bash
    sudo systemctl start postgresql
    ```

11. Create database for the indexer:

    ```bash
    cd left-curve/indexer
    createdb -h localhost -U postgres grug_dev
    ```

12. Start dango:

    ```bash
    sudo systemctl start dango
    ```

13. Start cometbft:

    ```bash
    sudo systemctl start cometbft
    ```

    **Note:** when starting, start in this order: postgresql, dango, cometbft. When terminating, do it in the reverse order.

### Killing existing devnet and start a new one

1. Stop dango and cometbft services (no need to stop postgresql):

   ```bash
   sudo systemctl stop cometbft
   sudo systemctl stop dango
   ```

2. Reset cometbft:

   ```bash
   cometbft unsafe-reset-all
   ```

3. Reset dango:

   ```bash
   dango db reset
   ```

4. Reset indexer DB:

   ```bash
   dropdb -h localhost -U postgres grug_dev
   createdb -h localhost -U postgres grug_dev
   ```

5. Delete indexer saved blocks:

   ```bash
   rm -rfv ~/.dango/indexer
   ```

6. Restart the services:

   ```bash
   sudo systemctl start dango
   sudo systemctl start cometbft
   ```

## Test accounts

Each devnet comes with 13 genesis users: `owner`, `user{1-9}` and `val{1-3}`. They use Secp256k1 public keys derived from seed phrases with derivation path `m/44'/60'/0'/0/0`.

**Do NOT use these keys in production!!!**

|              |                                                                                                                                                                 |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Username** | `owner`                                                                                                                                                         |
| **Private**  | `8a8b0ab692eb223f6a2927ad56e63c2ae22a8bc9a5bdfeb1d8127819ddcce177`                                                                                              |
| **Public**   | `0278f7b7d93da9b5a62e28434184d1c337c2c28d4ced291793215ab6ee89d7fff8`                                                                                            |
| **Mnemonic** | success away current amateur  choose crystal busy labor cost genius industry cement rhythm refuse whale admit meadow truck edge tiger melt flavor weapon august |

<br>

|              |                                                                                                                                                                          |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Username** | `user1`                                                                                                                                                                  |
| **Private**  | `a5122c0729c1fae8587e3cc07ae952cb77dfccc049efd5be1d2168cbe946ca18`                                                                                                       |
| **Public**   | `03bcf89d5d4f18048f0662d359d17a2dbbb08a80b1705bc10c0b953f21fb9e1911`                                                                                                     |
| **Mnemonic** | auction popular sample armed lecture leader novel control muffin grunt ceiling alcohol pulse lunch eager chimney quantum attend deny copper stumble write suggest aspect |

<br>

|              |                                                                                                                                                           |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Username** | `user2`                                                                                                                                                   |
| **Private**  | `cac7b4ced59cf0bfb14c373272dfb3d4447c7cd5aea732ea6ff69e19f85d34c4`                                                                                        |
| **Public**   | `02d309ba716f271b1083e24a0b9d438ef7ae0505f63451bc1183992511b3b1d52d`                                                                                      |
| **Mnemonic** | noodle walk produce road repair tornado leisure trip hold bomb curve live feature satoshi avocado ask pitch there decrease guitar swarm hybrid alarm make |

<br>

|              |                                                                                                                                                              |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Username** | `user3`                                                                                                                                                      |
| **Private**  | `cf6bb15648a3a24976e2eeffaae6201bc3e945335286d273bb491873ac7c3141`                                                                                           |
| **Public**   | `024bd61d80a2a163e6deafc3676c734d29f1379cb2c416a32b57ceed24b922eba0`                                                                                         |
| **Mnemonic** | alley honey observe various success garbage area include demise age cash foster model royal kingdom section place lend frozen loyal layer pony october blush |

<br>

|              |                                                                                                                                               |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------- |
| **Username** | `user4`                                                                                                                                       |
| **Private**  | `126b714bfe7ace5aac396aa63ff5c92c89a2d894debe699576006202c63a9cf6`                                                                            |
| **Public**   | `024a23e7a6f85e942a4dbedb871c366a1fdad6d0b84e670125991996134c270df2`                                                                          |
| **Mnemonic** | foot loyal damp alien better first glue supply claw author jar know holiday slam main siren paper transfer cram breeze glow forest word giant |

<br>

|              |                                                                                                                                                   |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Username** | `user5`                                                                                                                                           |
| **Private**  | `fe55076e4b2c9ffea813951406e8142fefc85183ebda6222500572b0a92032a7`                                                                                |
| **Public**   | `03da86b1cd6fd20350a0b525118eef939477c0fe3f5052197cd6314ed72f9970ad`                                                                              |
| **Mnemonic** | cliff ramp foot thrive scheme almost notice wreck base naive warfare horse plug limb keep steel tone over season basic answer post exchange wreck |

<br>

|              |                                                                                                                                                          |
| ------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Username** | `user6`                                                                                                                                                  |
| **Private**  | `4d3658519dd8a8227764f64c6724b840ffe29f1ca456f5dfdd67f834e10aae34`                                                                                       |
| **Public**   | `03428b179a075ff2142453c805a71a63b232400cc33c8e8437211e13e2bd1dec4c`                                                                                     |
| **Mnemonic** | spring repeat dog spider dismiss bring media orphan process cycle soft divorce pencil parade hill plate message bamboo kid fun dose celery table unknown |

<br>

|              |                                                                                                                                                         |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Username** | `user7`                                                                                                                                                 |
| **Private**  | `82de24ba8e1bc4511ae10ce3fbe84b4bb8d7d8abc9ba221d7d3cf7cd0a85131f`                                                                                      |
| **Public**   | `028d4d7265d5838190842ada2573ef9edfc978dec97ca59ce48cf1dd19352a4407`                                                                                    |
| **Mnemonic** | indoor welcome kite echo gloom glance gossip finger cake entire laundry citizen employ total aim inmate parade grace end foot truly park autumn pelican |

<br>

|              |                                                                                                                                                        |
| ------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Username** | `user8`                                                                                                                                                |
| **Private**  | `ca956fcf6b0f32975f067e2deaf3bc1c8632be02ed628985105fd1afc94531b9`                                                                                     |
| **Public**   | `02a888b140a836cd71a5ef9bc7677a387a2a4272343cf40722ab9e85d5f8aa21bd`                                                                                   |
| **Mnemonic** | moon inmate unique oil cupboard tube cigar subway index survey anger night know piece laptop labor capable term ivory bright nice during pattern floor |

<br>

|              |                                                                                                                                                                 |
| ------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Username** | `user9`                                                                                                                                                         |
| **Private**  | `c0d853951557d3bdec5add2ca8e03983fea2f50c6db0a45977990fb7b0c569b3`                                                                                              |
| **Public**   | `0230f93baa8e1dbe40a928144ec2144eed902c94b835420a6af4aafd2e88cb7b52`                                                                                            |
| **Mnemonic** | bird okay punch bridge peanut tonight solar stereo then oil clever flock thought example equip juice twenty unfold reform dragon various gossip design artefact |

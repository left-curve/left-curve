use clap::Parser;

#[derive(Parser)]
pub enum TxCmd {
    // TODO
}

impl TxCmd {
    pub fn run(self) -> anyhow::Result<()> {
        todo!()
    }
}

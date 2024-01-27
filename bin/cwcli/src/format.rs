use serde::ser;

pub fn print_json_pretty(data: impl ser::Serialize) -> anyhow::Result<()> {
    serde_json::to_string_pretty(&data)
        .map(|data_str| println!("{data_str}"))
        .map_err(Into::into)
}

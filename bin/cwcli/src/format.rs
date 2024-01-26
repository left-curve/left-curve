use serde::ser;

pub fn print_json_pretty(data: impl ser::Serialize) -> anyhow::Result<()> {
    serde_json::to_string_pretty(&data)
        .and_then(|data_str| Ok(println!("{data_str}")))
        .map_err(Into::into)
}

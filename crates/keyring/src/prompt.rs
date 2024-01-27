use {colored_json::ToColoredJson, serde::ser};

pub fn print_json_pretty(data: impl ser::Serialize) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(&data)?;
    let colored = json.to_colored_json_auto()?;

    println!("{colored}");

    Ok(())
}

pub fn confirm(prompt: impl ToString) -> dialoguer::Result<bool> {
    dialoguer::Confirm::new()
        .with_prompt(prompt.to_string())
        .interact()
}

pub fn read_text(prompt: impl ToString) -> dialoguer::Result<String> {
    dialoguer::Input::new()
        .with_prompt(prompt.to_string())
        .report(false)
        .interact_text()
}

pub fn read_password(prompt: impl ToString) -> dialoguer::Result<String> {
    dialoguer::Password::new()
        .with_prompt(prompt.to_string())
        .interact()
}

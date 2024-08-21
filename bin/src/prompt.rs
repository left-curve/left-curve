use {colored_json::ToColoredJson, grug::JsonExt};

pub fn print_json_pretty<T>(data: &T) -> anyhow::Result<()>
where
    T: JsonExt,
{
    let json = data.to_json_string_pretty()?;
    let colored = json.to_colored_json_auto()?;

    println!("{colored}");

    Ok(())
}

pub fn confirm<T>(prompt: T) -> dialoguer::Result<bool>
where
    T: ToString,
{
    dialoguer::Confirm::new()
        .with_prompt(prompt.to_string())
        .interact()
}

pub fn read_text<T>(prompt: T) -> dialoguer::Result<String>
where
    T: ToString,
{
    dialoguer::Input::new()
        .with_prompt(prompt.to_string())
        .report(false)
        .interact_text()
}

pub fn read_password<T>(prompt: T) -> dialoguer::Result<String>
where
    T: ToString,
{
    dialoguer::Password::new()
        .with_prompt(prompt.to_string())
        .interact()
}

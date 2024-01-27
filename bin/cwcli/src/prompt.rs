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
        .report(false)
        .interact()
}

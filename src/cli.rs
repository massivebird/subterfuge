use clap::{command, Arg, Command, ValueHint};

pub fn build_command() -> Command {
    command!().args([
        Arg::new("api_key")
            .short('k')
            .long("api-key")
            .alias("key")
            .required(false)
            .value_hint(ValueHint::FilePath)
            .value_name("PATH")
            .help("Path to a file containing a Steam API key."),
        Arg::new("config")
            .short('c')
            .long("config-file")
            .alias("config")
            .required(false)
            .value_hint(ValueHint::FilePath)
            .value_name("PATH")
            .help("Path to the YAML config file."),
        Arg::new("user_ids")
            .long("user-ids")
            .alias("users")
            .conflicts_with("config")
            .required(false)
            .value_name("user-IDs")
            .help("Comma-separated user IDs."),
    ])
}

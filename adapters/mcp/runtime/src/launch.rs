//! Native launch grammar and generated value validation. Application composition
//! owns process selectors, owner admission, and the lifetime of protocol stdout.

pub use launch_types::ConnectorsMcpLaunchLocalLaunchInput as Input;

/// Extract before Clap propagates the fixture's finite-output global arguments.
/// The returned command has no output renderer and grants no runtime authority.
pub fn command() -> clap::Command {
    launch_contract::command()
        .get_subcommands()
        .find(|command| command.get_name() == "server")
        .expect("the generated native binding declares server")
        .clone()
}

/// Validate the server leaf against both generated shape and generated values.
/// Clap parses option syntax; the ESS input contract owns requiredness and enums.
pub fn input(matches: &clap::ArgMatches) -> Result<Input, clap::Error> {
    let value = match matches.get_one::<String>("field:transport") {
        Some(transport) => serde_json::json!({"transport": transport}),
        None => serde_json::json!({}),
    };
    let plan = launch_contract::plan();
    let shape = &plan.callables["local-server"]
        .input
        .as_ref()
        .expect("the generated launch callable has input")
        .shape;
    if !shape.accepts(&value) {
        return Err(invalid_input());
    }
    serde_json::from_value(value).map_err(|_| invalid_input())
}

fn invalid_input() -> clap::Error {
    // Never echo configuration, state paths or arbitrary supplied values.
    clap::Error::raw(
        clap::error::ErrorKind::ValueValidation,
        "server requires --transport stdio",
    )
}

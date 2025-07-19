use std::env;
use std::path;
use std::path::PathBuf;

pub mod error;
pub mod object;
pub mod secret;
pub mod server;

#[tokio::main]
async fn main() -> Result<(), error::Error> {
    let config_folder = env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| "$HOME/.config".to_string());
    let mut config_path = path::PathBuf::new();
    config_path.push(&config_folder);
    config_path.push("secret-service-server");

    let state_folder = if let Some(home_dir) = std::env::home_dir() {
        home_dir
    } else {
        PathBuf::from("$HOME")
    }.join(".local/share/sss/");

    let mut builder = config::Config::builder()
        .set_default("log_level", "INFO")?
        .set_default("dbus_name", "org.freedesktop.secrets")?
        .set_default("state_folder", state_folder.to_str())?
        .add_source(config::Environment::with_prefix("sss"));

    builder = if config_path.exists() {
        builder.add_source(config::File::from(config_path))
    } else {
        builder
    };
    let settings = builder.build()?;

    structured_logger::Builder::with_level(
        &settings
            .get_string("log_level")
            .expect("log_level defaults to 'INFO'"),
    )
    .with_target_writer(
        "*",
        structured_logger::async_json::new_writer(tokio::io::stdout()),
    )
    .init();


    let dbus_name: String = settings
        .get("dbus_name")
        .expect("dus_name defaults to 'org.freedesktop.secrets'");

    let state_folder: String = settings.get("state_folder").expect("Somehow no state folder");
    let state_path = path::PathBuf::from(state_folder);
    log::debug!("State path: {:?}", state_path);
    let state_path = match tokio::fs::create_dir_all(&state_path).await {
        Ok(_) => Some(state_path),
        Err(_) => None
    };

    log::debug!("Running server with state_path: {:?}", state_path);
    let server = server::SecretServiceServer::new(&dbus_name, event_listener::Event::new(), state_path).await?;
    server.run().await?;

    Ok(())
}

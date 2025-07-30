use clap::Parser;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None, name = "Environment Loader")]
struct Application {
    ///
    /// Specify a list of variables that should be passed through to the environment.
    ///
    /// These variables will not be pre-processed by the environment loader.
    ///
    /// This is useful for variables you dont control, like things injected by AWS
    /// or other services.
    ///
    #[arg(short, long)]
    pub pass: Vec<String>,

    ///
    /// Dont exit when a loadable variable is not found.
    ///
    #[arg(short, long, default_value_t = false)]
    pub ignore_missing: bool,

    ///
    /// Prefix for all environment variables
    ///
    /// if set, all variables will be forwarded except those with the prefix
    ///
    /// prefixed variables will be intercepted and loaded
    ///
    #[arg(short, long)]
    pub env_prefix: Option<String>,

    ///
    /// The command to run with the environment variables loaded.
    ///
    #[clap(trailing_var_arg = true, required = true)]
    pub cmd: Vec<String>,
}

#[derive(Default)]
pub struct Amazon {
    config: Option<aws_config::SdkConfig>,
    secrets_client: Option<aws_sdk_secretsmanager::Client>,
}

impl Amazon {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn get_config(&mut self) -> &aws_config::SdkConfig {
        let config = &mut self.config;
        if config.is_some() {
            config.as_ref().unwrap()
        } else {
            let amazon = aws_config::defaults(aws_config::BehaviorVersion::v2025_01_17())
                .load()
                .await;

            *config = Some(amazon);

            config.as_ref().unwrap()
        }
    }

    pub async fn get_secret(&mut self, secret_name: &str) -> Option<String> {
        if let Some(client) = self.secrets_client.as_ref() {
            let response = client
                .get_secret_value()
                .secret_id(secret_name)
                .send()
                .await;

            response.ok()?.secret_string().map(String::from)
        } else {
            let config = self.get_config().await;

            let new_secrets_client = aws_sdk_secretsmanager::Client::new(config);

            let response = new_secrets_client
                .get_secret_value()
                .secret_id(secret_name)
                .send()
                .await;

            self.secrets_client = Some(new_secrets_client);

            response.ok()?.secret_string().map(String::from)
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let application = Application::parse();

    let mut variables = std::env::vars().collect::<HashMap<String, String>>();

    let mut passed_variables = HashMap::<String, String>::new();

    for variable in &application.pass {
        if let Some(value) = variables.remove(variable) {
            passed_variables.insert(variable.clone(), value);
        } else {
            tracing::warn!(
                "Variable {} not found in environment - cannot pass through",
                variable
            );
        }
    }

    if let Some(prefix) = &application.env_prefix {
        for variable in variables.keys().cloned().collect::<Vec<_>>() {
            if !variable.starts_with(prefix) {
                let value = variables.remove(&variable).unwrap();
                passed_variables.insert(variable.clone(), value);
            }
        }
    }

    let mut amazon = Amazon::new();

    for (key, value) in variables {
        if value.contains("::") {
            let (load_method, remainder) = value.split_once("::").unwrap();

            match load_method {
                "value" => {
                    // Pass the remainder as the value directly
                    if let Some(prefix) = &application.env_prefix {
                        if key.starts_with(prefix) {
                            passed_variables.insert(
                                key.strip_prefix(prefix).unwrap().to_string(),
                                remainder.to_string(),
                            );
                        } else {
                            passed_variables.insert(key, remainder.to_string());
                        }
                    } else {
                        passed_variables.insert(key, remainder.to_string());
                    }
                }
                "aws_sm" => {
                    // Load the value from AWS Secrets Manager

                    match amazon.get_secret(remainder).await {
                        Some(value) => {
                            if let Some(prefix) = &application.env_prefix {
                                if key.starts_with(prefix) {
                                    passed_variables.insert(
                                        key.strip_prefix(prefix).unwrap().to_string(),
                                        value,
                                    );
                                } else {
                                    passed_variables.insert(key, value);
                                }
                            } else {
                                passed_variables.insert(key, value);
                            }
                        }
                        None => {
                            tracing::warn!(
                                "Failed to load secret {} for variable {}",
                                remainder,
                                key
                            );
                            if !application.ignore_missing {
                                std::process::exit(1);
                            }
                        }
                    }
                }
                _ => {
                    tracing::warn!("Unknown load method {} for variable {}", load_method, key);
                    if !application.ignore_missing {
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    // Go ahead and call the target application,

    let binary = std::ffi::CString::from_str(&application.cmd[0]).unwrap();

    let args = application
        .cmd
        .iter()
        .map(|s| std::ffi::CString::from_str(s).unwrap())
        .collect::<Vec<_>>();

    let env = passed_variables
        .iter()
        .map(|(k, v)| std::ffi::CString::from_str(&format!("{k}={v}")).unwrap())
        .collect::<Vec<_>>();

    nix::unistd::execvpe(&binary, &args, &env).unwrap();
}

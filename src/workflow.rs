use crate::config::SignConfig;
use std::fmt::Write;

const WRAPPER_TEMPLATE: &str = include_str!("templates/github-actions-wrapper.yml");
const MACOS_TEMPLATE: &str = include_str!("templates/github-actions-macos.yml");
const WINDOWS_TEMPLATE: &str = include_str!("templates/github-actions-windows.yml");
const LINUX_TEMPLATE: &str = include_str!("templates/github-actions-linux.yml");

pub fn generate_workflow(config: &SignConfig) -> String {
    let mut jobs = String::new();
    jobs.push_str("jobs:\n");

    if let Some(macos) = &config.macos {
        let secrets = build_secrets_block(&collect_macos_env_vars(macos));
        let job = MACOS_TEMPLATE.replace("{MACOS_SECRETS}", &secrets);
        jobs.push_str(&job);
        jobs.push('\n');
    }

    if let Some(windows) = &config.windows {
        let secrets = build_secrets_block(&collect_windows_env_vars(windows));
        let job = WINDOWS_TEMPLATE.replace("{WINDOWS_SECRETS}", &secrets);
        jobs.push_str(&job);
        jobs.push('\n');
    }

    if let Some(linux) = &config.linux {
        let secrets = build_secrets_block(&collect_linux_env_vars(linux));
        let job = LINUX_TEMPLATE.replace("{LINUX_SECRETS}", &secrets);
        jobs.push_str(&job);
        jobs.push('\n');
    }

    WRAPPER_TEMPLATE.replace("{JOBS}", jobs.trim_end())
}

fn build_secrets_block(env_vars: &[&str]) -> String {
    let mut block = String::new();
    for var in env_vars {
        writeln!(block, "          {var}: ${{{{ secrets.{var} }}}}").unwrap();
    }
    block.trim_end().to_string()
}

fn collect_macos_env_vars(macos: &crate::config::MacosConfig) -> Vec<&str> {
    let env = &macos.env;
    let mut vars = Vec::new();
    if let Some(v) = &env.certificate {
        vars.push(v.as_str());
    }
    if let Some(v) = &env.certificate_password {
        vars.push(v.as_str());
    }
    if let Some(v) = &env.notarization_key {
        vars.push(v.as_str());
    }
    if let Some(v) = &env.notarization_key_id {
        vars.push(v.as_str());
    }
    if let Some(v) = &env.notarization_issuer {
        vars.push(v.as_str());
    }
    if let Some(v) = &env.apple_id {
        vars.push(v.as_str());
    }
    if let Some(v) = &env.team_id {
        vars.push(v.as_str());
    }
    if let Some(v) = &env.app_password {
        vars.push(v.as_str());
    }
    vars
}

fn collect_windows_env_vars(windows: &crate::config::WindowsConfig) -> Vec<&str> {
    let env = &windows.env;
    let mut vars = Vec::new();
    if let Some(v) = &env.tenant_id {
        vars.push(v.as_str());
    }
    if let Some(v) = &env.client_id {
        vars.push(v.as_str());
    }
    if let Some(v) = &env.client_secret {
        vars.push(v.as_str());
    }
    if let Some(v) = &env.endpoint {
        vars.push(v.as_str());
    }
    if let Some(v) = &env.account_name {
        vars.push(v.as_str());
    }
    if let Some(v) = &env.cert_profile {
        vars.push(v.as_str());
    }
    vars
}

fn collect_linux_env_vars(linux: &crate::config::LinuxConfig) -> Vec<&str> {
    let mut vars = Vec::new();
    if let Some(v) = &linux.env.key {
        vars.push(v.as_str());
    }
    vars
}

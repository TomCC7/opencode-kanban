use std::env;
use std::ffi::{OsStr, OsString};
use std::process::Command;
use std::sync::LazyLock;

static LAUNCH_ENV: LazyLock<Vec<(OsString, OsString)>> =
    LazyLock::new(|| env::vars_os().collect());

pub fn launch_env() -> Vec<(OsString, OsString)> {
    LAUNCH_ENV.clone()
}

pub fn command(program: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(program);
    command.envs(launch_env());
    command
}

pub fn tmux_env_args() -> Vec<String> {
    tmux_env_args_from(launch_env())
}

pub(crate) fn tmux_env_args_from<I, K, V>(env: I) -> Vec<String>
where
    I: IntoIterator<Item = (K, V)>,
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    let mut entries: Vec<(String, String)> = env
        .into_iter()
        .filter_map(|(key, value)| {
            let key = key.as_ref().to_str()?;
            let value = value.as_ref().to_str()?;
            should_forward_tmux_env(key).then(|| (key.to_string(), value.to_string()))
        })
        .collect();
    entries.sort_by(|left, right| left.0.cmp(&right.0));

    let mut args = Vec::with_capacity(entries.len() * 2);
    for (key, value) in entries {
        args.push("-e".to_string());
        args.push(format!("{key}={value}"));
    }
    args
}

fn should_forward_tmux_env(key: &str) -> bool {
    !key.is_empty()
        && !key.contains('=')
        && !matches!(key, "TERM" | "TERMCAP" | "TMUX" | "TMUX_PANE")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tmux_env_args_forward_user_env() {
        let args = tmux_env_args_from([
            ("GIT_LFS_SKIP_SMUDGE", "1"),
            ("GIT_SSH_COMMAND", "ssh -i /tmp/key"),
        ]);

        assert_eq!(
            args,
            vec![
                "-e",
                "GIT_LFS_SKIP_SMUDGE=1",
                "-e",
                "GIT_SSH_COMMAND=ssh -i /tmp/key"
            ]
        );
    }

    #[test]
    fn test_tmux_env_args_skip_tmux_internal_env() {
        let args = tmux_env_args_from([
            ("TERM", "xterm-256color"),
            ("TMUX", "/tmp/tmux"),
            ("TMUX_PANE", "%1"),
        ]);

        assert!(args.is_empty());
    }
}

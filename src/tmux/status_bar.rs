use anyhow::{Context, Result};

use super::{ensure_success_with_output, tmux_command};

const STATUS_BG: &str = "colour234";
const STATUS_FG: &str = "colour252";
const STATUS_MUTED_FG: &str = "colour245";

const ACCENT_COLORS: &[AccentColor] = &[
    AccentColor { bg: 33, fg: 231 },
    AccentColor { bg: 37, fg: 16 },
    AccentColor { bg: 64, fg: 231 },
    AccentColor { bg: 67, fg: 231 },
    AccentColor { bg: 99, fg: 231 },
    AccentColor { bg: 135, fg: 231 },
    AccentColor { bg: 166, fg: 16 },
    AccentColor { bg: 172, fg: 16 },
    AccentColor { bg: 203, fg: 231 },
    AccentColor { bg: 209, fg: 16 },
    AccentColor { bg: 214, fg: 16 },
    AccentColor { bg: 70, fg: 16 },
    AccentColor { bg: 75, fg: 16 },
    AccentColor { bg: 141, fg: 16 },
];

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct AccentColor {
    bg: u8,
    fg: u8,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TaskStatusBarSpec {
    pub status_left: String,
    pub status_right: String,
    pub status_style: String,
}

pub fn tmux_apply_task_status_bar(
    session_name: &str,
    category_title: &str,
    task_title: &str,
    branch_name: &str,
    color_seed: &str,
) -> Result<()> {
    for args in task_status_bar_args(
        session_name,
        category_title,
        task_title,
        branch_name,
        color_seed,
    ) {
        let output = tmux_command()
            .args(args)
            .output()
            .context("failed to configure tmux task status bar")?;
        ensure_success_with_output(&output, "set-option")?;
    }

    Ok(())
}

pub fn render_task_status_bar(
    category_title: &str,
    task_title: &str,
    branch_name: &str,
    color_seed: &str,
) -> TaskStatusBarSpec {
    let accent = accent_for_seed(color_seed);
    let category = status_badge_title(category_title);
    let title = tmux_status_text(task_title, 96);
    let branch = tmux_status_text(branch_name, 72);

    TaskStatusBarSpec {
        status_left: format!(
            "#[fg=colour{},bg=colour{}] {} #[fg={STATUS_FG},bg={STATUS_BG}] {} ",
            accent.fg, accent.bg, category, title
        ),
        status_right: if branch.is_empty() {
            String::new()
        } else {
            format!("#[fg={STATUS_MUTED_FG},bg={STATUS_BG}] {branch} ")
        },
        status_style: format!("fg={STATUS_FG},bg={STATUS_BG}"),
    }
}

fn task_status_bar_args(
    session_name: &str,
    category_title: &str,
    task_title: &str,
    branch_name: &str,
    color_seed: &str,
) -> Vec<Vec<String>> {
    let spec = render_task_status_bar(category_title, task_title, branch_name, color_seed);

    vec![
        set_option_args(session_name, "status", "on"),
        set_option_args(session_name, "status-position", "bottom"),
        set_option_args(session_name, "status-style", &spec.status_style),
        set_option_args(session_name, "status-left-length", "160"),
        set_option_args(session_name, "status-right-length", "96"),
        set_option_args(session_name, "status-left", &spec.status_left),
        set_option_args(session_name, "status-right", &spec.status_right),
    ]
}

fn set_option_args(session_name: &str, option: &str, value: &str) -> Vec<String> {
    vec![
        "set-option".to_string(),
        "-t".to_string(),
        session_name.to_string(),
        option.to_string(),
        value.to_string(),
    ]
}

fn accent_for_seed(seed: &str) -> AccentColor {
    let hash = fnv1a(seed.trim().as_bytes());
    ACCENT_COLORS[hash as usize % ACCENT_COLORS.len()]
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn status_badge_title(title: &str) -> String {
    let title = tmux_status_text(title, 24);
    if title.is_empty() {
        "TASK".to_string()
    } else {
        title.to_uppercase()
    }
}

fn tmux_status_text(value: &str, max_chars: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut chars = normalized.chars();
    let mut text = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        text.push('…');
    }
    text.replace('#', "##")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_status_bar_uses_category_title_task_title_and_branch() {
        let spec = render_task_status_bar(
            "In Progress",
            "Custom tmux bottom bar",
            "feat/tmux-status",
            "task-1",
        );

        assert!(spec.status_left.contains(" IN PROGRESS "));
        assert!(spec.status_left.contains("Custom tmux bottom bar"));
        assert!(spec.status_right.contains("feat/tmux-status"));
        assert_eq!(spec.status_style, "fg=colour252,bg=colour234");
    }

    #[test]
    fn render_status_bar_escapes_tmux_hashes() {
        let spec = render_task_status_bar("Review", "Fix #123 #[bad]", "feat/#hash", "task-2");

        assert!(spec.status_left.contains("Fix ##123 ##[bad]"));
        assert!(spec.status_right.contains("feat/##hash"));
    }

    #[test]
    fn render_status_bar_uses_stable_accent_for_same_seed() {
        let first = render_task_status_bar("Todo", "One", "branch", "same-task");
        let second = render_task_status_bar("Todo", "Two", "other", "same-task");

        let first_badge_style = first.status_left.split(']').next();
        let second_badge_style = second.status_left.split(']').next();
        assert_eq!(first_badge_style, second_badge_style);
    }

    #[test]
    fn task_status_bar_args_are_session_local_set_options() {
        let args = task_status_bar_args("ok-session", "Todo", "Task", "branch", "seed");

        assert!(args.iter().all(|arg| arg[0] == "set-option"));
        assert!(args.iter().all(|arg| arg[1] == "-t"));
        assert!(args.iter().all(|arg| arg[2] == "ok-session"));
        assert!(
            args.iter()
                .any(|arg| arg[3] == "status-position" && arg[4] == "bottom")
        );
    }
}

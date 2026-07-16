use crate::permission::PermissionDecision;
use crate::render::{render_permission_controls, PermissionChoice};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal;
use std::io::{self, IsTerminal, Write};

/// 单次 CLI 权限提示的可变交互状态。
#[derive(Debug)]
struct PermissionPromptState {
    selected: PermissionChoice,
    reply_draft: Option<String>,
}

/// 单次按键处理后的状态变化。
#[derive(Debug, Eq, PartialEq)]
enum PromptTransition {
    Continue,
    Submit(PermissionDecision),
}

/// 终端原始模式恢复守卫。
struct RawModeGuard;

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

impl PermissionPromptState {
    /// 创建默认选中“允许一次”的权限提示状态。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - 新的权限提示状态
    fn new() -> Self {
        Self {
            selected: PermissionChoice::Allow,
            reply_draft: None,
        }
    }

    /// 处理一个终端事件并更新权限选择或回复草稿。
    ///
    /// 参数:
    /// - `event`: Crossterm 终端事件
    ///
    /// 返回:
    /// - 继续交互或提交最终决定
    fn handle_event(&mut self, event: Event) -> PromptTransition {
        if let Event::Paste(text) = event {
            if let Some(draft) = self.reply_draft.as_mut() {
                draft.push_str(&text);
            }
            return PromptTransition::Continue;
        }
        let Event::Key(key) = event else {
            return PromptTransition::Continue;
        };
        if key.kind == KeyEventKind::Release {
            return PromptTransition::Continue;
        }
        if let Some(draft) = self.reply_draft.as_mut() {
            return match key.code {
                KeyCode::Enter => PromptTransition::Submit(PermissionDecision::Deny {
                    reply: (!draft.trim().is_empty()).then(|| draft.trim().to_string()),
                }),
                KeyCode::Esc => {
                    self.reply_draft = None;
                    PromptTransition::Continue
                }
                KeyCode::Backspace => {
                    draft.pop();
                    PromptTransition::Continue
                }
                KeyCode::Char(value) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                    draft.push(value);
                    PromptTransition::Continue
                }
                _ => PromptTransition::Continue,
            };
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.prev();
                PromptTransition::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = self.selected.next();
                PromptTransition::Continue
            }
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Char('1') => {
                PromptTransition::Submit(PermissionDecision::Allow)
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Char('2') | KeyCode::Esc => {
                PromptTransition::Submit(PermissionDecision::Deny { reply: None })
            }
            KeyCode::Char('3') => {
                self.selected = PermissionChoice::DenyWithReply;
                self.reply_draft = Some(String::new());
                PromptTransition::Continue
            }
            KeyCode::Enter => match self.selected {
                PermissionChoice::Allow => PromptTransition::Submit(PermissionDecision::Allow),
                PermissionChoice::Deny => {
                    PromptTransition::Submit(PermissionDecision::Deny { reply: None })
                }
                PermissionChoice::DenyWithReply => {
                    self.reply_draft = Some(String::new());
                    PromptTransition::Continue
                }
            },
            _ => PromptTransition::Continue,
        }
    }

    /// 渲染当前权限选择和可选回复草稿。
    ///
    /// 参数:
    /// - 无
    ///
    /// 返回:
    /// - ANSI 权限控件文本
    fn render(&self) -> String {
        render_permission_controls(self.selected, self.reply_draft.as_deref())
    }
}

/// 读取单次 CLI 权限决定。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 用户选择的允许或拒绝决定
pub(super) fn read_permission_decision() -> Result<PermissionDecision> {
    if io::stdin().is_terminal() && io::stderr().is_terminal() {
        read_terminal_decision()
    } else {
        read_line_decision()
    }
}

/// 在交互式终端中读取方向键、快捷键和回复文本。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 用户提交的权限决定
fn read_terminal_decision() -> Result<PermissionDecision> {
    let mut state = PermissionPromptState::new();
    let mut stderr = io::stderr();
    terminal::enable_raw_mode()?;
    let _guard = RawModeGuard;
    let mut rendered_rows = render_state(&mut stderr, &state)?;
    loop {
        let event = event::read()?;
        match state.handle_event(event) {
            PromptTransition::Continue => {
                clear_state(&mut stderr, rendered_rows)?;
                rendered_rows = render_state(&mut stderr, &state)?;
            }
            PromptTransition::Submit(decision) => {
                writeln!(stderr)?;
                stderr.flush()?;
                return Ok(decision);
            }
        }
    }
}

/// 绘制权限状态并返回占用的视觉行数。
///
/// 参数:
/// - `stderr`: 终端错误输出
/// - `state`: 当前权限提示状态
///
/// 返回:
/// - 已绘制视觉行数
fn render_state(stderr: &mut io::Stderr, state: &PermissionPromptState) -> Result<usize> {
    let output = state.render();
    writeln!(stderr, "{output}")?;
    stderr.flush()?;
    Ok(crate::render::rendered_visual_rows(&output))
}

/// 清除上一帧权限控件。
///
/// 参数:
/// - `stderr`: 终端错误输出
/// - `rendered_rows`: 上一帧占用的视觉行数
///
/// 返回:
/// - 清除是否成功
fn clear_state(stderr: &mut io::Stderr, rendered_rows: usize) -> Result<()> {
    write!(
        stderr,
        "{}",
        crate::render::clear_rendered_rows(rendered_rows)
    )?;
    stderr.flush()?;
    Ok(())
}

/// 在管道或重定向输入中读取编号和拒绝原因。
///
/// 参数:
/// - 无
///
/// 返回:
/// - 用户提交的权限决定
fn read_line_decision() -> Result<PermissionDecision> {
    eprintln!("1. 允许一次\n2. 拒绝\n3. 拒绝并告诉 Miyu 如何调整");
    eprint!("选择 [1-3]，也可直接输入拒绝原因: ");
    io::stderr().flush()?;
    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    let answer = answer.trim();
    if answer == "1" || answer.eq_ignore_ascii_case("y") || answer.eq_ignore_ascii_case("yes") {
        return Ok(PermissionDecision::Allow);
    }
    if answer == "3" {
        eprint!("告诉 Miyu 应如何调整: ");
        io::stderr().flush()?;
        let mut reply = String::new();
        io::stdin().read_line(&mut reply)?;
        return Ok(PermissionDecision::Deny {
            reply: (!reply.trim().is_empty()).then(|| reply.trim().to_string()),
        });
    }
    Ok(PermissionDecision::Deny {
        reply: (!answer.is_empty()
            && answer != "2"
            && !answer.eq_ignore_ascii_case("n")
            && !answer.eq_ignore_ascii_case("no"))
        .then(|| answer.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyEventState};

    /// 构造权限提示状态机测试使用的按键事件。
    ///
    /// 参数:
    /// - `code`: 按键编码
    ///
    /// 返回:
    /// - 按下状态的终端事件
    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    /// 验证方向键可以选择拒绝并提交。
    #[test]
    fn down_and_enter_submit_denial() {
        let mut state = PermissionPromptState::new();

        assert_eq!(
            state.handle_event(key(KeyCode::Down)),
            PromptTransition::Continue
        );
        assert!(state.render().contains("❯ 拒绝"));
        assert_eq!(
            state.handle_event(key(KeyCode::Enter)),
            PromptTransition::Submit(PermissionDecision::Deny { reply: None })
        );
    }

    /// 验证拒绝回复支持输入、退格和提交。
    #[test]
    fn denial_reply_supports_editing() {
        let mut state = PermissionPromptState::new();

        state.handle_event(key(KeyCode::Char('3')));
        state.handle_event(key(KeyCode::Char('a')));
        state.handle_event(key(KeyCode::Char('b')));
        state.handle_event(key(KeyCode::Backspace));

        assert_eq!(
            state.handle_event(key(KeyCode::Enter)),
            PromptTransition::Submit(PermissionDecision::Deny {
                reply: Some("a".to_string())
            })
        );
    }
}

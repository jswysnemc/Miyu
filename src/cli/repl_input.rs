use super::*;

pub(super) struct ReplInputSubmission {
    pub(super) mode: AgentMode,
    pub(super) raw_input: String,
    pub(super) chat_input: clipboard::ClipboardChatInput,
}

pub(super) fn enable_repl_terminal_input(stdout: &mut io::Stdout) -> Result<()> {
    terminal::enable_raw_mode()?;
    if let Err(err) = execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES),
        EnableBracketedPaste
    ) {
        let _ = terminal::disable_raw_mode();
        return Err(err.into());
    }
    Ok(())
}

/// 恢复 REPL 输入终端模式。
///
/// 参数:
/// - `stdout`: 终端输出
///
/// 返回:
/// - 恢复是否成功
pub(super) fn disable_repl_terminal_input(stdout: &mut io::Stdout) -> Result<()> {
    let restore_result = execute!(stdout, DisableBracketedPaste, PopKeyboardEnhancementFlags);
    let raw_result = terminal::disable_raw_mode();
    restore_result?;
    raw_result?;
    Ok(())
}

pub(super) fn read_repl_input(
    mut mode: AgentMode,
    prefill: Option<String>,
    history: &[String],
) -> Result<Option<ReplInputSubmission>> {
    let mut stdout = io::stdout();
    let mut input = strip_terminal_control_sequences(&prefill.unwrap_or_default());
    let mut cursor = input.chars().count();
    let mut history_index = history.len();
    let mut clipboard_state = ReplClipboardState::default();
    let mut last_escape = None::<Instant>;
    let mut last_ctrl_c = None::<Instant>;
    let (cursor_col, _) = cursor::position()?;
    if cursor_col != 0 {
        writeln!(stdout)?;
        stdout.flush()?;
    }
    enable_repl_terminal_input(&mut stdout)?;
    let (_, mut input_row) = cursor::position()?;
    let mut rendered_rows = 0u16;
    let mut is_pasted = false;
    render_repl_input(
        &mut stdout,
        &mut input_row,
        &mut rendered_rows,
        mode,
        &input,
        cursor,
        is_pasted,
    )?;
    loop {
        match event::read()? {
            Event::Paste(text) => {
                let text = strip_terminal_control_sequences(&text);
                insert_str_at_cursor(&mut input, &mut cursor, &text);
                is_pasted = true;
                render_repl_input(
                    &mut stdout,
                    &mut input_row,
                    &mut rendered_rows,
                    mode,
                    &input,
                    cursor,
                    is_pasted,
                )?;
            }
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                if code != KeyCode::Esc {
                    last_escape = None;
                }
                if !matches!(code, KeyCode::Char('c')) || !modifiers.contains(KeyModifiers::CONTROL)
                {
                    last_ctrl_c = None;
                }
                match code {
                    KeyCode::Tab => {
                        if input.starts_with('/') {
                            if let Some(completed) = complete_repl_command(&input) {
                                input = completed.to_string();
                                cursor = input.chars().count();
                            }
                        } else {
                            mode = if mode == AgentMode::Yolo {
                                AgentMode::Plan
                            } else {
                                AgentMode::Yolo
                            };
                        }
                        is_pasted = false;
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Esc => {
                        let now = Instant::now();
                        if last_escape.is_some_and(|previous| {
                            now.duration_since(previous) <= REPL_ESC_CLEAR_WINDOW
                        }) {
                            input.clear();
                            cursor = 0;
                            clipboard_state.clear();
                            is_pasted = false;
                            last_escape = None;
                            render_repl_input(
                                &mut stdout,
                                &mut input_row,
                                &mut rendered_rows,
                                mode,
                                &input,
                                cursor,
                                is_pasted,
                            )?;
                        } else {
                            last_escape = Some(now);
                        }
                    }
                    KeyCode::Left => {
                        cursor = cursor.saturating_sub(1);
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Right => {
                        cursor = (cursor + 1).min(input.chars().count());
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Home => {
                        cursor = 0;
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::End => {
                        cursor = input.chars().count();
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Up => {
                        let plain_prefix = format!("[{}] > ", mode.label());
                        if let Some(next_cursor) = move_cursor_up_by_visual_row(
                            &plain_prefix,
                            &input,
                            cursor,
                            terminal_cols(),
                        ) {
                            cursor = next_cursor;
                            render_repl_input(
                                &mut stdout,
                                &mut input_row,
                                &mut rendered_rows,
                                mode,
                                &input,
                                cursor,
                                is_pasted,
                            )?;
                        } else if !history.is_empty() {
                            history_index = history_index.saturating_sub(1);
                            input = history.get(history_index).cloned().unwrap_or_default();
                            cursor = input.chars().count();
                            clipboard_state.clear();
                            is_pasted = false;
                            render_repl_input(
                                &mut stdout,
                                &mut input_row,
                                &mut rendered_rows,
                                mode,
                                &input,
                                cursor,
                                is_pasted,
                            )?;
                        }
                    }
                    KeyCode::Down => {
                        let plain_prefix = format!("[{}] > ", mode.label());
                        if let Some(next_cursor) = move_cursor_down_by_visual_row(
                            &plain_prefix,
                            &input,
                            cursor,
                            terminal_cols(),
                        ) {
                            cursor = next_cursor;
                        } else if history_index + 1 < history.len() {
                            history_index += 1;
                            input = history.get(history_index).cloned().unwrap_or_default();
                            cursor = input.chars().count();
                            clipboard_state.clear();
                            is_pasted = false;
                        } else if history_index < history.len() {
                            history_index = history.len();
                            input.clear();
                            cursor = input.chars().count();
                            clipboard_state.clear();
                            is_pasted = false;
                        }
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Enter if modifiers.contains(KeyModifiers::SHIFT) => {
                        insert_newline_at_cursor(&mut input, &mut cursor);
                        is_pasted = false;
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Enter => {
                        input = strip_terminal_control_sequences(&input);
                        let chat_input = clipboard_state.to_chat_input(&input);
                        clear_repl_input(&mut stdout, input_row, rendered_rows)?;
                        disable_repl_terminal_input(&mut stdout)?;
                        return Ok(Some(ReplInputSubmission {
                            mode,
                            raw_input: input,
                            chat_input,
                        }));
                    }
                    KeyCode::Char('j') if modifiers.contains(KeyModifiers::CONTROL) => {
                        insert_newline_at_cursor(&mut input, &mut cursor);
                        is_pasted = false;
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Char('v') if modifiers.contains(KeyModifiers::CONTROL) => {
                        is_pasted = clipboard_state.paste_into_input(&mut input, &mut cursor)?;
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Char('g') if modifiers.contains(KeyModifiers::CONTROL) => {
                        move_after_repl_input(&mut stdout, input_row, rendered_rows)?;
                        disable_repl_terminal_input(&mut stdout)?;
                        match edit_input_buffer(&input) {
                            Ok(edited) => {
                                input = strip_terminal_control_sequences(&edited);
                                cursor = input.chars().count();
                                clipboard_state.clear();
                            }
                            Err(err) => {
                                eprintln!("{err}");
                            }
                        }
                        enable_repl_terminal_input(&mut stdout)?;
                        let (_, row) = cursor::position()?;
                        input_row = row;
                        rendered_rows = 0;
                        is_pasted = false;
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                        let now = Instant::now();
                        if last_ctrl_c.is_some_and(|previous| {
                            now.duration_since(previous) <= REPL_CTRL_C_EXIT_WINDOW
                        }) {
                            move_after_repl_input(&mut stdout, input_row, rendered_rows)?;
                            disable_repl_terminal_input(&mut stdout)?;
                            return Ok(None);
                        }
                        last_ctrl_c = Some(now);
                        input.clear();
                        cursor = 0;
                        clipboard_state.clear();
                        is_pasted = false;
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Char('d')
                        if modifiers.contains(KeyModifiers::CONTROL) && input.is_empty() =>
                    {
                        move_after_repl_input(&mut stdout, input_row, rendered_rows)?;
                        disable_repl_terminal_input(&mut stdout)?;
                        return Ok(None);
                    }
                    KeyCode::Char('l') if modifiers.contains(KeyModifiers::CONTROL) => {
                        queue!(stdout, Clear(ClearType::All), MoveTo(0, 0))?;
                        stdout.flush()?;
                        input_row = 0;
                        rendered_rows = 0;
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Char('w') if modifiers.contains(KeyModifiers::CONTROL) => {
                        remove_word_before_cursor(&mut input, &mut cursor);
                        is_pasted = false;
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Backspace => {
                        if clipboard_state.remove_block_before_cursor(&mut input, &mut cursor) {
                            // 已删除完整剪贴板占位块
                        } else if cursor > 0 {
                            remove_char_before_cursor(&mut input, &mut cursor);
                        }
                        is_pasted = false;
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Delete => {
                        if !clipboard_state.remove_block_at_cursor(&mut input, cursor) {
                            remove_char_at_cursor(&mut input, cursor);
                        }
                        is_pasted = false;
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    KeyCode::Char(ch) if !modifiers.contains(KeyModifiers::CONTROL) => {
                        if !is_disallowed_control_char(ch) {
                            insert_char_at_cursor(&mut input, &mut cursor, ch);
                        }
                        is_pasted = false;
                        render_repl_input(
                            &mut stdout,
                            &mut input_row,
                            &mut rendered_rows,
                            mode,
                            &input,
                            cursor,
                            is_pasted,
                        )?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

use super::ZenviView;
use crate::nvim::process::{NvimEvent, NvimSession};
use gpui::*;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;

impl ZenviView {
    pub fn reload_nvim(&mut self, cx: &mut Context<Self>) {
        log::info!("Reloading Neovim session...");
        let old_session = Arc::clone(&self.session);
        let last_cols = self.last_cols;
        let last_rows = self.last_rows;
        let fallback_cwd = self.cwd.clone();
        let window_handle = self.window_handle;

        self._event_task = None;

        cx.spawn(move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                // 1. Check auto-session status and save session if active
                let check_lua = r#"
                    local ok, auto_session = pcall(require, "auto-session")
                    if not ok or not auto_session then
                        return { has_auto_session = false, should_restore = false, cwd = vim.fn.getcwd() }
                    end

                    local is_session_active = false
                    local this_session = vim.v.this_session
                    if this_session and this_session ~= "" then
                        is_session_active = true
                    else
                        local lib_ok, lib = pcall(require, "auto-session.lib")
                        if lib_ok and lib and lib.get_session_file_name then
                            local sfile = lib.get_session_file_name()
                            if sfile and vim.fn.filereadable(sfile) == 1 then
                                is_session_active = true
                            end
                        end
                    end

                    -- Check if any named file buffers are currently open
                    local bufs = vim.fn.getbufinfo({ buflisted = 1 })
                    local has_valid_buffers = false
                    for _, b in ipairs(bufs) do
                        if b.name and b.name ~= "" then
                            has_valid_buffers = true
                            break
                        end
                    end

                    local should_restore = false
                    if is_session_active or has_valid_buffers then
                        pcall(auto_session.save_session)
                        should_restore = true
                    end

                    return {
                        has_auto_session = true,
                        should_restore = should_restore,
                        cwd = vim.fn.getcwd(),
                    }
                "#;

                let (should_restore, current_cwd) =
                    match old_session.exec_lua(check_lua, vec![]).await {
                        Ok(val) => {
                            let mut should_restore = false;
                            let mut cwd_opt = None;
                            if let Some(map) = val.as_map() {
                                for (k, v) in map {
                                    if k.as_str() == Some("should_restore") {
                                        should_restore = v.as_bool().unwrap_or(false);
                                    } else if k.as_str() == Some("cwd") {
                                        if let Some(s) = v.as_str() {
                                            if !s.is_empty() {
                                                cwd_opt = Some(PathBuf::from(s));
                                            }
                                        }
                                    }
                                }
                            }
                            (should_restore, cwd_opt.or(fallback_cwd))
                        }
                        Err(e) => {
                            log::warn!(
                                "Failed to query auto-session before reload: {:?}",
                                e
                            );
                            (false, fallback_cwd)
                        }
                    };

                // 2. Terminate old session
                old_session.kill();

                // 3. Spawn new session in background
                let (event_tx, event_rx) = mpsc::unbounded_channel::<NvimEvent>();
                match NvimSession::spawn(event_tx, current_cwd.clone()) {
                    Ok(new_session) => {
                        new_session.attach_ui(last_cols, last_rows);
                        new_session.send_command("set mouse=a");

                        if let Some(ref dir) = current_cwd {
                            new_session.send_command(&format!("cd {}", dir.display()));
                        }

                        // 4. If auto-session was active, restore session in new instance
                        if should_restore {
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                            let restore_lua = r#"
                                (function()
                                    local ok, auto_session = pcall(require, "auto-session")
                                    if ok and auto_session then
                                        pcall(auto_session.restore_session)
                                    end
                                end)()
                            "#;
                            new_session
                                .send_command(&format!("lua {}", restore_lua));
                        }

                        let _ = this.update(&mut cx, |this, cx| {
                            this.session = new_session;
                            this.cwd = current_cwd;
                            this.last_guifont = String::new();
                            this.last_linespace = 0;
                            this._event_task =
                                Some(Self::spawn_event_listener(event_rx, window_handle, cx));
                            cx.notify();
                        });
                    }
                    Err(e) => {
                        eprintln!("Failed to reload Neovim: {:?}", e);
                    }
                }
            }
        })
        .detach();
    }

    pub fn open_file(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open File".into()),
        });
        let session = Arc::clone(&self.session);
        cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                if let Ok(Ok(Some(paths))) = receiver.await {
                    for path in paths {
                        if let Some(parent) = path.parent() {
                            let _ = this.update(&mut cx, |this, _cx| {
                                this.cwd = Some(parent.to_path_buf());
                            });
                            session.send_command(&format!("cd {}", parent.display()));
                        }
                        session.send_command(&format!("edit {}", path.display()));
                    }
                }
            }
        })
        .detach();
    }

    pub fn open_folder(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Open Folder".into()),
        });
        let session = Arc::clone(&self.session);
        cx.spawn(|this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                if let Ok(Ok(Some(paths))) = receiver.await {
                    for path in paths {
                        let _ = this.update(&mut cx, |this, _cx| {
                            this.cwd = Some(path.clone());
                        });
                        session.send_command(&format!("cd {}", path.display()));
                        session.send_command(&format!("edit {}", path.display()));
                    }
                }
            }
        })
        .detach();
    }

    pub fn open_paths(&mut self, paths: &[PathBuf]) {
        for path in paths {
            if path.is_dir() {
                self.cwd = Some(path.clone());
                self.session
                    .send_command(&format!("cd {}", path.display()));
                self.session
                    .send_command(&format!("edit {}", path.display()));
            } else {
                if let Some(parent) = path.parent() {
                    if parent.exists() && parent.as_os_str() != "" {
                        self.cwd = Some(parent.to_path_buf());
                        self.session
                            .send_command(&format!("cd {}", parent.display()));
                    }
                }
                self.session
                    .send_command(&format!("edit {}", path.display()));
            }
        }
    }

    pub fn paste(&mut self, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard() {
            if let Some(text) = item.text() {
                self.session.paste(&text);
            }
        }
    }

    pub fn copy(&mut self, _cx: &mut Context<Self>) {
        self.session.send_command(
            r#"lua (function()
            local mode = vim.api.nvim_get_mode().mode
            if mode:find("[vV\x16]") then
                vim.cmd('normal! "+y')
            end
        end)()"#,
        );
    }

    pub fn cut(&mut self, _cx: &mut Context<Self>) {
        self.session.send_command(
            r#"lua (function()
            local mode = vim.api.nvim_get_mode().mode
            if mode:find("[vV\x16]") then
                vim.cmd('normal! "+d')
            end
        end)()"#,
        );
    }

    pub fn select_all(&mut self, _cx: &mut Context<Self>) {
        self.session.send_command(
            r#"lua (function()
            local mode = vim.api.nvim_get_mode().mode
            if mode:find("[iR]") then
                vim.cmd('stopinsert')
            end
            vim.cmd('normal! ggVG')
        end)()"#,
        );
    }

    pub fn undo(&mut self, _cx: &mut Context<Self>) {
        self.session.send_command(
            r#"lua (function()
            local mode = vim.api.nvim_get_mode().mode
            if mode:find("[iR]") then
                vim.cmd('stopinsert')
            end
            pcall(vim.cmd, 'undo')
        end)()"#,
        );
    }

    pub fn redo(&mut self, _cx: &mut Context<Self>) {
        self.session.send_command(
            r#"lua (function()
            local mode = vim.api.nvim_get_mode().mode
            if mode:find("[iR]") then
                vim.cmd('stopinsert')
            end
            pcall(vim.cmd, 'redo')
        end)()"#,
        );
    }

    pub fn install_cli(&mut self, _cx: &mut Context<Self>) {
        match crate::cli::install_shell_command() {
            Ok(symlink_path) => {
                log::info!(
                    "Shell command successfully installed to {}",
                    symlink_path.display()
                );
                self.session.send_command(&format!(
                    "lua pcall(vim.notify, 'Successfully installed \"zenvi\" command to: {}', vim.log.levels.INFO)",
                    symlink_path.display()
                ));
            }
            Err(e) => {
                log::error!("Failed to install shell command: {:?}", e);
                self.session.send_command(&format!(
                    "lua pcall(vim.notify, 'Failed to install zenvi command: {}', vim.log.levels.ERROR)",
                    e
                ));
            }
        }
    }

    pub fn close_buffer(&mut self, _cx: &mut Context<Self>) {
        let lua_cmd = r##"lua (function()
            local cur = vim.api.nvim_get_current_buf()
            local bufs = vim.tbl_filter(function(b)
                return vim.api.nvim_buf_is_valid(b) and vim.bo[b].buflisted
            end, vim.api.nvim_list_bufs())

            if #bufs <= 1 then
                vim.cmd("confirm quit")
                return
            end

            local alt = vim.fn.bufnr("#")
            local next_buf = nil
            if alt > 0 and alt ~= cur and vim.api.nvim_buf_is_valid(alt) and vim.bo[alt].buflisted then
                next_buf = alt
            else
                for _, b in ipairs(bufs) do
                    if b ~= cur then
                        next_buf = b
                        break
                    end
                end
            end

            if next_buf then
                for _, w in ipairs(vim.api.nvim_list_wins()) do
                    if vim.api.nvim_win_is_valid(w) and vim.api.nvim_win_get_buf(w) == cur then
                        vim.api.nvim_win_set_buf(w, next_buf)
                    end
                end
            end

            vim.cmd("confirm bdelete " .. cur)
        end)()"##;
        self.session.send_command(lua_cmd);
    }
}

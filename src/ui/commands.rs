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
                let check_lua = include_str!("../../lua/commands/check_session.lua");

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
                match NvimSession::spawn(event_tx, current_cwd.clone(), Vec::new()) {
                    Ok(new_session) => {
                        new_session.attach_ui(last_cols, last_rows);
                        new_session.send_command("set mouse=a");
                        new_session.send_command("set title");

                        if let Some(ref dir) = current_cwd {
                            new_session.send_command(&format!("cd {}", dir.display()));
                        }

                        // 4. If auto-session was active, restore session in new instance
                        if should_restore {
                            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                            const RESTORE_LUA: &str =
                                concat!("lua ", include_str!("../../lua/commands/restore_session.lua"));
                            new_session.send_command(RESTORE_LUA);
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
        const CLOSE_BUFFER_LUA: &str =
            concat!("lua ", include_str!("../../lua/commands/close_buffer.lua"));
        self.session.send_command(CLOSE_BUFFER_LUA);
    }

    pub fn show_about(&mut self, _cx: &mut Context<Self>) {
        const SHOW_ABOUT_LUA: &str =
            concat!("lua ", include_str!("../../lua/commands/show_about.lua"));
        self.session.send_command(SHOW_ABOUT_LUA);
    }
}

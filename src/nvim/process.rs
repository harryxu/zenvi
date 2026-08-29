use crate::nvim::events::handle_redraw_event;
use crate::nvim::protocol::RpcMessage;
use crate::nvim::state::NvimState;
use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use rmpv::Value;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvimEvent {
    Redraw,
    Exit,
}

pub struct NvimSession {
    msg_id: AtomicU32,
    tx: mpsc::UnboundedSender<Value>,
    pub state: Arc<RwLock<NvimState>>,
    is_terminating: Arc<AtomicBool>,
    abort_handles: Vec<tokio::task::AbortHandle>,
}

impl NvimSession {
    pub fn spawn(
        event_tx: mpsc::UnboundedSender<NvimEvent>,
        cwd: Option<PathBuf>,
    ) -> Result<Arc<Self>> {
        let mut cmd = Command::new("nvim");
        cmd.arg("--embed")
            .arg("--cmd")
            .arg("let g:zenvi = v:true | let g:gui_running = 1 | set ttimeout ttimeoutlen=10")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        if let Some(ref dir) = cwd {
            cmd.current_dir(dir);
        }

        let mut child = cmd.spawn()?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to take stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("Failed to take stdout"))?;

        let (tx, mut rx) = mpsc::unbounded_channel::<Value>();
        let state = Arc::new(RwLock::new(NvimState::default()));
        let is_terminating = Arc::new(AtomicBool::new(false));

        // Background task to write to stdin
        let write_task = tokio::spawn(async move {
            while let Some(val) = rx.recv().await {
                let mut buf = Vec::new();
                if let Ok(_) = rmpv::encode::write_value(&mut buf, &val) {
                    let _ = stdin.write_all(&buf).await;
                    let _ = stdin.flush().await;
                }
            }
        });

        let state_clone = Arc::clone(&state);
        let event_tx_clone = event_tx.clone();
        let is_terminating_clone = Arc::clone(&is_terminating);

        // Background task to read from stdout
        let read_task = tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdout);
            let mut buffer = Vec::new();
            let mut temp_buf = [0u8; 8192];

            loop {
                match reader.read(&mut temp_buf).await {
                    Ok(0) => {
                        // Neovim has terminated (EOF)
                        if !is_terminating_clone.load(Ordering::SeqCst) {
                            let _ = event_tx_clone.send(NvimEvent::Exit);
                        }
                        break;
                    }
                    Ok(n) => {
                        buffer.extend_from_slice(&temp_buf[..n]);

                        let mut cursor = std::io::Cursor::new(&buffer);
                        let mut last_pos = 0;

                        while let Ok(val) = rmpv::decode::read_value(&mut cursor) {
                            last_pos = cursor.position() as usize;
                            if let Some(msg) = RpcMessage::parse(val) {
                                match msg {
                                    RpcMessage::Notification { method, params } => {
                                        if method == "redraw" {
                                            {
                                                let mut s = state_clone.write();
                                                for event in params {
                                                    if let Some(event_arr) = event.as_array() {
                                                        handle_redraw_event(&mut s, event_arr);
                                                    }
                                                }
                                            }
                                            let _ = event_tx_clone.send(NvimEvent::Redraw);
                                        }
                                    }
                                    RpcMessage::Response { .. } => {}
                                    RpcMessage::Request { .. } => {}
                                }
                            }
                        }

                        if last_pos > 0 {
                            buffer.drain(..last_pos);
                        }
                    }
                    Err(_) => {
                        if !is_terminating_clone.load(Ordering::SeqCst) {
                            let _ = event_tx_clone.send(NvimEvent::Exit);
                        }
                        break;
                    }
                }
            }
        });

        // Background task to wait on child process and clean up
        let child_task = tokio::spawn(async move {
            let _ = child.wait().await;
        });

        let abort_handles = vec![
            write_task.abort_handle(),
            read_task.abort_handle(),
            child_task.abort_handle(),
        ];

        let session = Arc::new(Self {
            msg_id: AtomicU32::new(1),
            tx,
            state,
            is_terminating,
            abort_handles,
        });

        Ok(session)
    }

    pub fn kill(&self) {
        self.is_terminating.store(true, Ordering::SeqCst);
        let _ = self.send_command("qa!");
        for handle in &self.abort_handles {
            handle.abort();
        }
    }

    pub fn attach_ui(&self, width: usize, height: usize) {
        let id = self.msg_id.fetch_add(1, Ordering::SeqCst);
        let mut opts = Vec::new();
        opts.push((Value::from("ext_linegrid"), Value::from(true)));
        opts.push((Value::from("rgb"), Value::from(true)));

        let msg = RpcMessage::Request {
            msgid: id,
            method: "nvim_ui_attach".to_string(),
            params: vec![
                Value::from(width as u64),
                Value::from(height as u64),
                Value::Map(opts),
            ],
        };
        let _ = self.tx.send(msg.to_value());
    }

    pub fn send_input(&self, input: &str) {
        let id = self.msg_id.fetch_add(1, Ordering::SeqCst);
        let msg = RpcMessage::Request {
            msgid: id,
            method: "nvim_input".to_string(),
            params: vec![Value::from(input)],
        };
        let _ = self.tx.send(msg.to_value());
    }

    pub fn send_command(&self, cmd: &str) {
        let id = self.msg_id.fetch_add(1, Ordering::SeqCst);
        let msg = RpcMessage::Request {
            msgid: id,
            method: "nvim_command".to_string(),
            params: vec![Value::from(cmd)],
        };
        let _ = self.tx.send(msg.to_value());
    }

    pub fn send_mouse(
        &self,
        button: &str,
        action: &str,
        modifier: &str,
        grid: u64,
        row: usize,
        col: usize,
    ) {
        let id = self.msg_id.fetch_add(1, Ordering::SeqCst);
        let msg = RpcMessage::Request {
            msgid: id,
            method: "nvim_input_mouse".to_string(),
            params: vec![
                Value::from(button),
                Value::from(action),
                Value::from(modifier),
                Value::from(grid),
                Value::from(row as u64),
                Value::from(col as u64),
            ],
        };
        let _ = self.tx.send(msg.to_value());
    }

    pub fn try_resize(&self, width: usize, height: usize) {
        let id = self.msg_id.fetch_add(1, Ordering::SeqCst);
        let msg = RpcMessage::Request {
            msgid: id,
            method: "nvim_ui_try_resize".to_string(),
            params: vec![Value::from(width as u64), Value::from(height as u64)],
        };
        let _ = self.tx.send(msg.to_value());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_nvim_insert_escape() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let session = NvimSession::spawn(tx, None).expect("Failed to spawn nvim");
        session.attach_ui(80, 24);

        // Wait for initial redraws
        while let Ok(event) = tokio::time::timeout(Duration::from_millis(300), rx.recv()).await {
            if event == Some(NvimEvent::Redraw) {
                break;
            }
        }

        session.send_command("inoremap <esc> <cmd>noh<cr><esc>");
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Enter insert mode
        session.send_input("i");
        while let Ok(event) = tokio::time::timeout(Duration::from_millis(300), rx.recv()).await {
            if event == Some(NvimEvent::Redraw) {
                let s = session.state.read();
                if s.current_mode == "insert" {
                    break;
                }
            }
        }

        {
            let s = session.state.read();
            println!("After 'i', mode: {}", s.current_mode);
            assert_eq!(s.current_mode, "insert");
        }

        // Send <Esc>
        session.send_input("<Esc>");
        while let Ok(event) = tokio::time::timeout(Duration::from_millis(300), rx.recv()).await {
            if event == Some(NvimEvent::Redraw) {
                let s = session.state.read();
                if s.current_mode == "normal" {
                    break;
                }
            }
        }

        {
            let s = session.state.read();
            println!("After '<Esc>', mode: {}", s.current_mode);
            assert_eq!(s.current_mode, "normal");
        }
    }

    #[tokio::test]
    async fn test_nvim_kill_and_reload() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let session = NvimSession::spawn(tx, None).expect("Failed to spawn initial nvim");
        session.attach_ui(80, 24);

        // Wait for initial redraws
        while let Ok(event) = tokio::time::timeout(Duration::from_millis(300), rx.recv()).await {
            if event == Some(NvimEvent::Redraw) {
                break;
            }
        }

        // Kill session - ensure no Exit event is triggered
        session.kill();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Drain any remaining events in channel; none should be Exit
        while let Ok(Some(event)) = tokio::time::timeout(Duration::from_millis(50), rx.recv()).await {
            assert_ne!(event, NvimEvent::Exit, "Kill should not send Exit event");
        }

        // Spawn new session
        let (tx2, mut rx2) = mpsc::unbounded_channel();
        let new_session = NvimSession::spawn(tx2, None).expect("Failed to spawn reloaded nvim");
        new_session.attach_ui(80, 24);

        let mut received_redraw = false;
        while let Ok(event) = tokio::time::timeout(Duration::from_millis(300), rx2.recv()).await {
            if event == Some(NvimEvent::Redraw) {
                received_redraw = true;
                break;
            }
        }
        assert!(received_redraw, "New session should send Redraw event");
        new_session.kill();
    }
}

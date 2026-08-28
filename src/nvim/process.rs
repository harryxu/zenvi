use crate::nvim::events::handle_redraw_event;
use crate::nvim::protocol::RpcMessage;
use crate::nvim::state::NvimState;
use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use rmpv::Value;
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
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
}

impl NvimSession {
    pub fn spawn(event_tx: mpsc::UnboundedSender<NvimEvent>) -> Result<Arc<Self>> {
        let mut child = Command::new("nvim")
            .arg("--embed")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

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

        // Background task to write to stdin
        tokio::spawn(async move {
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

        // Background task to read from stdout
        tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(stdout);
            let mut buffer = Vec::new();
            let mut temp_buf = [0u8; 8192];

            loop {
                match reader.read(&mut temp_buf).await {
                    Ok(0) => {
                        // Neovim has terminated (EOF)
                        let _ = event_tx_clone.send(NvimEvent::Exit);
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
                        let _ = event_tx_clone.send(NvimEvent::Exit);
                        break;
                    }
                }
            }
        });

        let session = Arc::new(Self {
            msg_id: AtomicU32::new(1),
            tx,
            state,
        });

        Ok(session)
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

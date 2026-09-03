use crate::nvim::events::handle_redraw_event;
use crate::nvim::protocol::RpcMessage;
use crate::nvim::state::NvimState;
use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use rmpv::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NvimEvent {
    Redraw,
    Exit,
}

type PendingResponseTx = oneshot::Sender<Result<Value, Value>>;
type PendingRequests = Arc<parking_lot::Mutex<HashMap<u32, PendingResponseTx>>>;

pub struct NvimSession {
    msg_id: AtomicU32,
    tx: mpsc::UnboundedSender<Value>,
    pub state: Arc<RwLock<NvimState>>,
    is_terminating: Arc<AtomicBool>,
    abort_handles: Vec<tokio::task::AbortHandle>,
    pending_requests: PendingRequests,
}

fn find_nvim_binary() -> PathBuf {
    if let Ok(path) = std::env::var("NVIM_PATH") {
        let p = PathBuf::from(path);
        if p.exists() {
            return p;
        }
    }

    let candidates = [
        "nvim",
        "/opt/homebrew/bin/nvim",
        "/usr/local/bin/nvim",
        "/usr/bin/nvim",
        "/bin/nvim",
    ];

    for candidate in candidates {
        let p = PathBuf::from(candidate);
        if candidate == "nvim" {
            if let Ok(output) = std::process::Command::new("which").arg("nvim").output() {
                if output.status.success() {
                    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !s.is_empty() {
                        return PathBuf::from(s);
                    }
                }
            }
        } else if p.exists() {
            return p;
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        let user_candidates = [
            format!("{}/.local/bin/nvim", home),
            format!("{}/.cargo/bin/nvim", home),
            format!("{}/bin/nvim", home),
        ];
        for candidate in user_candidates {
            let p = PathBuf::from(candidate);
            if p.exists() {
                return p;
            }
        }
    }

    PathBuf::from("nvim")
}

impl NvimSession {
    pub fn spawn(
        event_tx: mpsc::UnboundedSender<NvimEvent>,
        cwd: Option<PathBuf>,
        targets: Vec<PathBuf>,
    ) -> Result<Arc<Self>> {
        Self::spawn_with_options(event_tx, cwd, targets, false)
    }

    pub fn spawn_with_options(
        event_tx: mpsc::UnboundedSender<NvimEvent>,
        cwd: Option<PathBuf>,
        targets: Vec<PathBuf>,
        clean: bool,
    ) -> Result<Arc<Self>> {
        let nvim_bin = find_nvim_binary();
        let mut cmd = Command::new(&nvim_bin);
        cmd.arg("--embed");
        if clean {
            cmd.arg("--clean");
        }
        cmd.arg("--cmd")
            .arg("let g:zenvi = v:true | let g:gui_running = 1 | set title | set ttimeout ttimeoutlen=10");

        // Automatically restore filetype detection and syntax/treesitter highlighting
        // when a session is restored (via auto-session, persistence.nvim, or native :source Session.vim).
        // Neovim suppresses standard FileType autocommands while SessionLoad=1 during session sourcing,
        // leaving restored buffers with an empty filetype ("") and without syntax highlighting.
        // Also sets up idle pre-warming mechanism.
        const INIT_LUA: &str = concat!("lua ", include_str!("../../lua/init.lua"));
        cmd.arg("--cmd").arg(INIT_LUA);

        for target in &targets {
            cmd.arg(target);
        }

        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        // Augment PATH so GUI apps launched by Finder inherit Homebrew, Cargo, and local binaries
        let current_path = std::env::var("PATH").unwrap_or_default();
        let mut augmented_paths = vec![
            "/opt/homebrew/bin".to_string(),
            "/opt/homebrew/sbin".to_string(),
            "/usr/local/bin".to_string(),
            "/usr/bin".to_string(),
            "/bin".to_string(),
            "/usr/sbin".to_string(),
            "/sbin".to_string(),
        ];
        if let Ok(home) = std::env::var("HOME") {
            augmented_paths.insert(0, format!("{}/.local/bin", home));
            augmented_paths.insert(1, format!("{}/.cargo/bin", home));
        }
        augmented_paths.push(current_path);
        let new_path = augmented_paths.join(":");
        cmd.env("PATH", new_path);

        if let Some(ref dir) = cwd {
            if dir.exists() {
                cmd.current_dir(dir);
            } else if let Some(safe_dir) = crate::window::get_safe_default_dir() {
                cmd.current_dir(safe_dir);
            }
        } else if let Some(safe_dir) = crate::window::get_safe_default_dir() {
            cmd.current_dir(safe_dir);
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
        let pending_requests: PendingRequests = Arc::new(parking_lot::Mutex::new(HashMap::new()));
        let pending_requests_clone = Arc::clone(&pending_requests);

        // Background task to write to stdin
        let write_task = tokio::spawn(async move {
            let mut buf = Vec::with_capacity(1024);
            while let Some(val) = rx.recv().await {
                buf.clear();
                let _ = rmpv::encode::write_value(&mut buf, &val);

                // Drain any additional pending values to batch multiple IPC messages in one write
                while let Ok(next_val) = rx.try_recv() {
                    let _ = rmpv::encode::write_value(&mut buf, &next_val);
                }

                if !buf.is_empty() {
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
            let mut reader = stdout;
            let mut buffer = Vec::with_capacity(65536);
            let mut temp_buf = [0u8; 65536];

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
                                        match method.as_str() {
                                            "redraw" => {
                                                let mut s = state_clone.write();
                                                for event in params {
                                                    if let Some(event_arr) = event.as_array() {
                                                        if handle_redraw_event(&mut s, event_arr) {
                                                            let _ = event_tx_clone.send(NvimEvent::Redraw);
                                                        }
                                                    }
                                                }
                                            }
                                            "zenvi_prewarm_start" => {
                                                state_clone.write().is_prewarming = true;
                                            }
                                            "zenvi_prewarm_end" => {
                                                state_clone.write().is_prewarming = false;
                                                let _ = event_tx_clone.send(NvimEvent::Redraw);
                                            }
                                            _ => {}
                                        }
                                    }
                                    RpcMessage::Response {
                                        msgid,
                                        error,
                                        result,
                                    } => {
                                        let mut map = pending_requests_clone.lock();
                                        if let Some(tx) = map.remove(&msgid) {
                                            if error.is_nil() {
                                                let _ = tx.send(Ok(result));
                                            } else {
                                                let _ = tx.send(Err(error));
                                            }
                                        }
                                    }
                                    RpcMessage::Request { .. } => {}
                                }
                            }
                        }

                        if last_pos > 0 {
                            if last_pos >= buffer.len() {
                                buffer.clear();
                            } else {
                                buffer.drain(..last_pos);
                            }
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
            pending_requests,
        });

        Ok(session)
    }

    pub fn kill(&self) {
        self.is_terminating.store(true, Ordering::SeqCst);
        let _ = self.send_command("qa!");
        for handle in &self.abort_handles {
            handle.abort();
        }
        self.pending_requests.lock().clear();
    }

    pub async fn request(&self, method: &str, params: Vec<Value>) -> Result<Value> {
        let (tx, rx) = oneshot::channel();
        let id = self.msg_id.fetch_add(1, Ordering::SeqCst);
        self.pending_requests.lock().insert(id, tx);

        let msg = RpcMessage::Request {
            msgid: id,
            method: method.to_string(),
            params,
        };
        self.tx
            .send(msg.into_value())
            .map_err(|_| anyhow!("Failed to send RPC request"))?;

        match tokio::time::timeout(std::time::Duration::from_millis(600), rx).await {
            Ok(Ok(Ok(val))) => Ok(val),
            Ok(Ok(Err(err))) => Err(anyhow!("RPC error: {:?}", err)),
            Ok(Err(_)) => Err(anyhow!("RPC channel closed")),
            Err(_) => {
                self.pending_requests.lock().remove(&id);
                Err(anyhow!("RPC request timed out"))
            }
        }
    }

    pub async fn exec_lua(&self, code: &str, args: Vec<Value>) -> Result<Value> {
        self.request(
            "nvim_exec_lua",
            vec![Value::from(code), Value::Array(args)],
        )
        .await
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
        let _ = self.tx.send(msg.into_value());
    }

    pub fn send_input(&self, input: &str) {
        let msg = RpcMessage::Notification {
            method: "nvim_input".to_string(),
            params: vec![Value::from(input)],
        };
        let _ = self.tx.send(msg.into_value());
    }

    pub fn send_command(&self, cmd: &str) {
        let msg = RpcMessage::Notification {
            method: "nvim_command".to_string(),
            params: vec![Value::from(cmd)],
        };
        let _ = self.tx.send(msg.into_value());
    }

    pub fn paste(&self, data: &str) {
        let msg = RpcMessage::Notification {
            method: "nvim_paste".to_string(),
            params: vec![
                Value::from(data),
                Value::from(false),
                Value::from(-1i64),
            ],
        };
        let _ = self.tx.send(msg.into_value());
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
        let msg = RpcMessage::Notification {
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
        let _ = self.tx.send(msg.into_value());
    }

    #[allow(dead_code)]
    pub async fn request_mouse(
        &self,
        button: &str,
        action: &str,
        modifier: &str,
        grid: u64,
        row: usize,
        col: usize,
    ) -> Result<Value> {
        self.request(
            "nvim_input_mouse",
            vec![
                Value::from(button),
                Value::from(action),
                Value::from(modifier),
                Value::from(grid),
                Value::from(row as u64),
                Value::from(col as u64),
            ],
        )
        .await
    }

    pub fn try_resize(&self, width: usize, height: usize) {
        let msg = RpcMessage::Notification {
            method: "nvim_ui_try_resize".to_string(),
            params: vec![Value::from(width as u64), Value::from(height as u64)],
        };
        let _ = self.tx.send(msg.into_value());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn spawn_test_session(tx: mpsc::UnboundedSender<NvimEvent>) -> Result<Arc<NvimSession>> {
        NvimSession::spawn_with_options(tx, None, Vec::new(), true)
    }

    #[test]
    fn test_nvim_insert_escape() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (tx, _rx) = mpsc::unbounded_channel();
            let session = spawn_test_session(tx).expect("Failed to spawn nvim");
            session.attach_ui(80, 24);

            // Wait for initial normal mode
            let start = std::time::Instant::now();
            while start.elapsed() < Duration::from_millis(1500) {
                if session.state.read().current_mode == "normal" {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }

            session.send_command("inoremap <esc> <cmd>noh<cr><esc>");
            tokio::time::sleep(Duration::from_millis(50)).await;

            // Enter insert mode and wait for mode update
            session.send_input("i");
            let start = std::time::Instant::now();
            while start.elapsed() < Duration::from_millis(1500) {
                if session.state.read().current_mode == "insert" {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }

            {
                let s = session.state.read();
                println!("After 'i', mode: {}", s.current_mode);
                assert_eq!(s.current_mode, "insert");
            }

            // Send <Esc> and wait for mode update
            session.send_input("<Esc>");
            let start = std::time::Instant::now();
            while start.elapsed() < Duration::from_millis(1500) {
                if session.state.read().current_mode == "normal" {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }

            {
                let s = session.state.read();
                println!("After '<Esc>', mode: {}", s.current_mode);
                assert_eq!(s.current_mode, "normal");
            }

            session.kill();
        });
    }

    #[test]
    fn test_nvim_kill_and_reload() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (tx, mut rx) = mpsc::unbounded_channel();
            let session = spawn_test_session(tx).expect("Failed to spawn initial nvim");
            session.attach_ui(80, 24);

            // Wait for initial redraws
            while let Ok(event) = tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
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
            let new_session = spawn_test_session(tx2).expect("Failed to spawn reloaded nvim");
            new_session.attach_ui(80, 24);

            let mut received_redraw = false;
            while let Ok(event) = tokio::time::timeout(Duration::from_millis(500), rx2.recv()).await {
                if event == Some(NvimEvent::Redraw) {
                    received_redraw = true;
                    break;
                }
            }
            assert!(received_redraw, "New session should send Redraw event");
            new_session.kill();
        });
    }

    #[test]
    fn test_nvim_exec_lua() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (tx, _rx) = mpsc::unbounded_channel();
            let session = spawn_test_session(tx).expect("Failed to spawn nvim");
            session.attach_ui(80, 24);

            let res = session
                .exec_lua("return 1 + 1", vec![])
                .await
                .expect("Failed to exec lua");
            assert_eq!(res.as_i64(), Some(2));

            let res_table = session
                .exec_lua("return { name = 'zenvi', count = 42 }", vec![])
                .await
                .expect("Failed to exec lua table");
            if let Some(map) = res_table.as_map() {
                let mut found_name = false;
                let mut found_count = false;
                for (k, v) in map {
                    if k.as_str() == Some("name") && v.as_str() == Some("zenvi") {
                        found_name = true;
                    }
                    if k.as_str() == Some("count") && v.as_i64() == Some(42) {
                        found_count = true;
                    }
                }
                assert!(found_name);
                assert!(found_count);
            } else {
                panic!("Expected map from lua table");
            }

            session.kill();
        });
    }

    #[test]
    fn test_auto_session_query_logic() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (tx, _rx) = mpsc::unbounded_channel();
            let session = spawn_test_session(tx).expect("Failed to spawn nvim");
            session.attach_ui(80, 24);

            let check_lua = include_str!("../../lua/commands/check_session.lua");

            let res = session
                .exec_lua(check_lua, vec![])
                .await
                .expect("Failed to run session check lua");
            let map = res.as_map().expect("Expected map from lua table");
            let mut has_cwd = false;
            for (k, v) in map {
                if k.as_str() == Some("cwd") {
                    assert!(v.as_str().is_some());
                    has_cwd = true;
                }
            }
            assert!(has_cwd);

            session.kill();
        });
    }

    #[test]
    fn test_nvim_paste() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (tx, _rx) = mpsc::unbounded_channel();
            let session = spawn_test_session(tx).expect("Failed to spawn nvim");
            session.attach_ui(80, 24);

            // Wait for initial normal mode
            let start = std::time::Instant::now();
            while start.elapsed() < Duration::from_millis(1500) {
                if session.state.read().current_mode == "normal" {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }

            // Enter insert mode and wait for mode update
            session.send_input("i");
            let start = std::time::Instant::now();
            while start.elapsed() < Duration::from_millis(1500) {
                if session.state.read().current_mode == "insert" {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }

            session.paste("Hello from Zenvi Clipboard!");

            // Poll until line content updates
            let start = std::time::Instant::now();
            let mut line = String::new();
            while start.elapsed() < std::time::Duration::from_millis(1500) {
                if let Ok(res) = session
                    .exec_lua("return vim.api.nvim_get_current_line()", vec![])
                    .await
                {
                    if let Some(s) = res.as_str() {
                        if s == "Hello from Zenvi Clipboard!" {
                            line = s.to_string();
                            break;
                        }
                    }
                }
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            }
            assert_eq!(line, "Hello from Zenvi Clipboard!");

            session.kill();
        });
    }

    #[test]
    fn test_spawn_with_targets() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (tx, _rx) = mpsc::unbounded_channel();
            let test_dir = std::env::temp_dir().join(format!("zenvi_test_spawn_{}", std::process::id()));
            let _ = std::fs::create_dir_all(&test_dir);
            let test_lua = test_dir.join("init.lua");
            let _ = std::fs::write(&test_lua, "-- test spawn with targets\n");

            let session = NvimSession::spawn_with_options(
                tx,
                Some(test_dir.clone()),
                vec![test_lua.clone()],
                true,
            )
            .expect("Failed to spawn nvim");
            session.attach_ui(80, 24);

            tokio::time::sleep(std::time::Duration::from_millis(300)).await;

            let res = session
                .exec_lua(include_str!("../../lua/test/check_buffer_ft.lua"), vec![])
                .await
                .expect("Failed to query buffer info");

            let map = res.as_map().expect("Expected map");
            for (k, v) in map {
                if k.as_str() == Some("ft") {
                    assert_eq!(v.as_str(), Some("lua"));
                }
            }

            session.kill();
            let _ = std::fs::remove_file(&test_lua);
            let _ = std::fs::remove_dir(&test_dir);
        });
    }

    #[test]
    fn test_session_restore_highlight() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (tx, _rx) = mpsc::unbounded_channel();
            let test_dir = std::env::temp_dir().join(format!("zenvi_session_test_{}", std::process::id()));
            let _ = std::fs::create_dir_all(&test_dir);
            let test_rs = test_dir.join("main.rs");
            let _ = std::fs::write(&test_rs, "fn main() {\n    println!(\"Hello\");\n}\n");
            let session_file = test_dir.join("Session.vim");

            let session = NvimSession::spawn_with_options(
                tx,
                Some(test_dir.clone()),
                Vec::new(),
                true,
            )
            .expect("Failed to spawn nvim");
            session.attach_ui(80, 24);

            // Open test file and create session sequentially
            session
                .request("nvim_command", vec![Value::from(format!("edit {}", test_rs.display()))])
                .await
                .unwrap();
            session
                .request("nvim_command", vec![Value::from(format!("mksession! {}", session_file.display()))])
                .await
                .unwrap();
            session
                .request("nvim_command", vec![Value::from("%bwipeout!")])
                .await
                .unwrap();

            // Source the session file, simulating session restore
            session
                .request("nvim_command", vec![Value::from(format!("source {}", session_file.display()))])
                .await
                .unwrap();

            tokio::time::sleep(Duration::from_millis(300)).await;

            let res = session
                .exec_lua(include_str!("../../lua/test/check_session_restore.lua"), vec![])
                .await
                .expect("Failed to run exec_lua");
            println!("BUFFER INFO: {:?}", res);

            // Assert that ft and syn are correctly restored to "rust"
            let map = res.as_map().unwrap();
            for (k, v) in map {
                if k.as_str() == Some("ft") {
                    assert_eq!(v.as_str(), Some("rust"));
                }
                if k.as_str() == Some("syn") {
                    assert_eq!(v.as_str(), Some("rust"));
                }
            }

            session.kill();
            let _ = std::fs::remove_file(&test_rs);
            let _ = std::fs::remove_file(&session_file);
            let _ = std::fs::remove_dir(&test_dir);
        });
    }

    #[test]
    fn test_prewarm_lua_execution() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let (event_tx, _event_rx) = tokio::sync::mpsc::unbounded_channel();
            let session = NvimSession::spawn(event_tx, None, vec![std::path::PathBuf::from("Cargo.toml")]).unwrap();

            // Wait briefly for nvim initialization
            tokio::time::sleep(Duration::from_millis(200)).await;

            let chans = session.exec_lua("return vim.api.nvim_list_chans()", vec![]).await.unwrap();
            println!("CHANS IN EMBEDDED NVIM: {:?}", chans);

            let res = session
                .exec_lua(include_str!("../../lua/test/test_prewarm.lua"), vec![])
                .await
                .expect("Failed to execute prewarm lua");

            let map = res.as_map().expect("Result must be a map");
            let mut ok = false;
            for (k, v) in map {
                if k.as_str() == Some("ok") {
                    ok = v.as_bool().unwrap_or(false);
                }
            }
            assert!(ok, "Prewarm should succeed for Cargo.toml");

            // Test that setting vim.g.zenvi_prewarm_max_lines = 0 disables prewarm
            session
                .exec_lua("vim.g.zenvi_prewarm_max_lines = 0", vec![])
                .await
                .unwrap();

            let res2 = session
                .exec_lua(include_str!("../../lua/test/test_prewarm_disabled.lua"), vec![])
                .await
                .unwrap();

            let map2 = res2.as_map().unwrap();
            let mut reason = "";
            for (k, v) in map2 {
                if k.as_str() == Some("reason") {
                    reason = v.as_str().unwrap_or("");
                }
            }
            assert_eq!(reason, "disabled", "Setting max_lines to 0 must disable prewarming");

            session.kill();
        });
    }
}

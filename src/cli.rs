use anyhow::{anyhow, Result};
use std::path::PathBuf;

pub fn install_shell_command() -> Result<PathBuf> {
    let current_exe = std::env::current_exe()
        .map_err(|e| anyhow!("Failed to get current executable path: {}", e))?;
    let canonical_exe = current_exe.canonicalize().unwrap_or(current_exe);

    let home = std::env::var("HOME")
        .map_err(|_| anyhow!("Environment variable $HOME is not set"))?;
    let bin_dir = PathBuf::from(home).join(".local").join("bin");

    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| anyhow!("Failed to create directory {}: {}", bin_dir.display(), e))?;

    let symlink_path = bin_dir.join("zenvi");

    // Remove existing file or symlink if it exists
    if symlink_path.exists() || symlink_path.is_symlink() {
        let _ = std::fs::remove_file(&symlink_path);
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&canonical_exe, &symlink_path)
            .map_err(|e| anyhow!("Failed to create symlink at {}: {}", symlink_path.display(), e))?;
    }

    log::info!(
        "Successfully installed shell command: {} -> {}",
        symlink_path.display(),
        canonical_exe.display()
    );

    Ok(symlink_path)
}

#[cfg(test)]
mod tests {
    use super::install_shell_command;

    #[test]
    fn test_install_shell_command_creates_valid_symlink() {
        let res = install_shell_command();
        assert!(res.is_ok(), "install_shell_command should succeed: {:?}", res.err());
        let symlink_path = res.unwrap();
        assert!(symlink_path.exists() || symlink_path.is_symlink());

        let target = std::fs::read_link(&symlink_path).expect("Should be a symlink");
        let current_exe = std::env::current_exe().unwrap();
        let canonical_exe = current_exe.canonicalize().unwrap_or(current_exe);
        assert_eq!(target, canonical_exe);
    }
}

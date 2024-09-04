#[cfg(windows)]
pub mod windows {
    use std::{path::PathBuf, process::Command};

    pub fn create_shortcut(
        target: PathBuf,
        args: &str,
        shortcut_path: PathBuf,
        desc: &str,
    ) -> anyhow::Result<()> {
        println!(
            "create shortcut: \"{} {}\" -> {}",
            target.display(),
            args,
            shortcut_path.display()
        );
        let ps_command = format!(
            r#"$WshShell = New-Object -ComObject WScript.Shell; $shortcut = $WshShell.CreateShortcut('{}'); $shortcut.TargetPath = '{}'; $shortcut.WorkingDirectory = '{}'; $shortcut.Description = '{}'; $shortcut.Save();"#,
            shortcut_path
                .to_str()
                .ok_or(anyhow::anyhow!("invalid shortcut path"))?,
            target
                .to_str()
                .ok_or(anyhow::anyhow!("invalid target path"))?,
            target
                .parent()
                .ok_or(anyhow::anyhow!("no parent"))?
                .to_str()
                .ok_or(anyhow::anyhow!("invalid parent path"))?,
            desc
        );

        println!("Command: {}", ps_command);

        let output = Command::new("powershell")
            .arg("-Command")
            .arg(ps_command)
            .output()?;

        if output.status.success() {
            println!("{}", String::from_utf8_lossy(&output.stdout));
            Ok(())
        } else {
            anyhow::bail!("{}", String::from_utf8_lossy(&output.stderr));
        }
    }
}

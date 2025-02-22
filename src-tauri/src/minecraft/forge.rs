use std::{path::Path, io};
use std::io::Read;
use tokio::fs;
use tokio::process::Command;

pub async fn download_forge(version: &str, install_dir: &str) -> io::Result<()> {
    let forge_installer_url = format!(
        "https://files.minecraftforge.net/maven/net/minecraftforge/forge/{}/forge-{}-installer.jar",
        version, version
    );
    let installer_path = format!("{}/forge_installer.jar", install_dir);

    let response = reqwest::get(&forge_installer_url).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let bytes = response.bytes().await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    fs::write(&installer_path, &bytes).await?;

    println!("Forge {} downloaded successfully.", version);
    Ok(())
}

pub async fn install_forge(install_dir: &str) -> io::Result<()> {
    let installer_path = format!("{}/forge_installer.jar", install_dir);
    if !Path::new(&installer_path).exists() {
        println!("Forge installer not found.");
        return Err(io::Error::new(io::ErrorKind::NotFound, "Forge installer not found"));
    }

    let output = Command::new("java")
        .arg("-jar")
        .arg(&installer_path)
        .arg("--installClient")
        .current_dir(install_dir)
        .output()
        .await?;

    println!("Forge installation output: {}", String::from_utf8_lossy(&output.stdout));
    Ok(())
}
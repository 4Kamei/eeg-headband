use anyhow::Result;
use std::{
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
    str::FromStr,
};

#[derive(Debug, Clone, Copy)]
pub enum Core {
    App,
    Net,
}

impl FromStr for Core {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "app" | "a" | "application" => Ok(Core::App),
            "net" | "n" | "network" => Ok(Core::Net),
            other => {
                let cores = ["app", "net"];
                Err(anyhow::anyhow!(
                    "{other:?} doesn't correspond to a core. Valid values are: {cores:?}"
                ))
            }
        }
    }
}

impl Core {
    fn get_build_path(&self) -> &Path {
        match self {
            Core::App => Path::new(concat!(env!("GIT_REPO_ROOT"), "/app-core/")),
            Core::Net => Path::new(concat!(env!("GIT_REPO_ROOT"), "/net-core/")),
        }
    }
}

/// Build the b
pub fn run() -> Result<()> {
    build_binary(Core::App)?;
    build_binary(Core::Net)?;

    // Link the binaries
    link_elfs()?;

    Ok(())
}

/// Link both of the elfs together to make a final elf
pub fn link_elfs() -> Result<()> {
    tracing::info!("Linking binaries together");
    Ok(())
}

pub fn flash_device() -> Result<()> {
    todo!()
}

/// Debug using the currently built binary, on the specific core
pub fn debug(core: Core) -> Result<()> {
    todo!("do whatever");
}

/// Build the binary for the specified core
pub fn build_binary(core: Core) -> Result<()> {
    let build_path = core.get_build_path();
    if !build_path.is_dir() || !build_path.exists() {
        anyhow::bail!("{build_path:?} doesn't point to an existing directory");
    }

    tracing::info!("Compiling {core:?}");

    let mut child = Command::new("cargo")
        .args(["build", "--release", "--message-format=json"])
        .current_dir(build_path)
        .stdout(Stdio::piped())
        .spawn()
        .or(Err(anyhow::anyhow!("failed to run cargo")))?;

    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    let mut executable = None;

    for line in reader.lines() {
        let line = line.unwrap();
        let json = serde_json::from_str::<serde_json::Value>(&line).or(Err(anyhow::anyhow!(
            "Couldn't deserialize cargo message: {line:?}"
        )))?;
        if json["reason"] == "compiler-artifact" {
            if let Some(elf_location) = json["executable"].as_str() {
                executable = Some(elf_location.to_string());
            }
        }
    }
    child.wait()?;
    if executable.is_none() {
        let mut cmd = Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(build_path)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .or(Err(anyhow::anyhow!("failed to run cargo")))?;
        cmd.wait().unwrap();
        anyhow::bail!("Couldn't continue due to compilation error");
    }

    Ok(())
}

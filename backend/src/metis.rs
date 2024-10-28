
use anyhow::{ bail, Context, Result };
use openssh::{ Session, KnownHosts };

pub type PBSId = String;
pub async fn metis_qsub (
    username: &str,
    hostname: &str,

    pbs_path: &str,
    args: Vec<&str>
) -> Result<PBSId> {
    // Attempt to connect to METIS
    let session = Session::connect_mux(&format!("{username}@{hostname}"), KnownHosts::Strict)
        .await
        .map_err(|e| anyhow::anyhow!("Error starting Metis connection! See below:\n{:#?}", e))?;

    // Add our args
    let mut command = session
        .command("qsub");
    for arg in &args {
        command.arg(arg);
    }

    // Run the job
    let output = command
        .arg(pbs_path)
        .output().await
        .context("Failed to run openpose command!")?;

    // Extract the output from stdout
    let stdout = String::from_utf8(output.stdout)
        .context("Server `stdout` was not valid UTF-8")?;
    let stderr = String::from_utf8(output.stderr)
        .context("Server `stderr` was not valid UTF-8")?;

    if stderr.len() > 0 {
        bail!("Server had `stderr`: {stderr}");
    }

    // Close the SSH session
    session.close().await
        .context("Failed to close SSH session - probably fine.")?;

    // Return as successful
    Ok(stdout.trim().into())
}

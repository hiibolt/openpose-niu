
use anyhow::{ bail, Context, Result, anyhow };
use openssh::{ Session, KnownHosts };

pub async fn metis_output_exists (
    username:  &str,
    hostname:  &str,

    pbs_job_id:   &str,
    pbs_job_name: &str
) -> Result<bool> {
    // Extract the job number since there's additional information in the job ID
    let pbs_job_number = pbs_job_id
        .split('.')
        .next()
        .ok_or(anyhow!("Missing job number! Ensure the Job ID is in the form <n>.cm!"))?;

    // Attempt to connect to METIS
    let session = Session::connect_mux(&format!("{username}@{hostname}"), KnownHosts::Strict)
        .await
        .map_err(|e| anyhow::anyhow!("Error starting Metis connection! See below:\n{:#?}", e))?;

    // Add our path and run the command
    let output = session
        .command("ls")
        .arg(format!("{pbs_job_name}.o{pbs_job_number}"))
        .output().await
        .context("Failed to run openpose command!")?;

    // Extract the output from stdout
    let _stdout = String::from_utf8(output.stdout)
        .context("Server `stdout` was not valid UTF-8")?;
    let stderr = String::from_utf8(output.stderr)
        .context("Server `stderr` was not valid UTF-8")?;

    // Close the SSH session
    session.close().await
        .context("Failed to close SSH session - probably fine.")?;

    // Return as successful
    Ok(stderr.is_empty())
}
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

    // Close the SSH session
    session.close().await
        .context("Failed to close SSH session - probably fine.")?;

    // Treat any error output as fatal
    if !stderr.is_empty() {
        bail!("Server had `stderr`: {stderr}");
    }

    // Return as successful
    Ok(stdout.trim().into())
}

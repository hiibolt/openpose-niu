mod fs;
mod metis;

use crate::metis::{ PBSId, metis_qsub, metis_output_exists };
use crate::fs::{ SSHPath, copy_file, move_to_send }; 

use anyhow::{ Context, Result };
use tokio::time::{ sleep, Duration };

#[tokio::main]
async fn main() -> Result<()> {
    let username: &str = "z1994244";
    let hostname: &str = "metis.niu.edu";

    let metis_pbs_path:     &str = "/lstr/sahara/zwlab/jw/openpose-api/run.pbs";
    let metis_inputs_path:  &str = "/lstr/sahara/zwlab/jw/openpose-api/inputs";
    let metis_outputs_path: &str = "/lstr/sahara/zwlab/jw/openpose-api/outputs";
    
    let local_to_send_dir:    &str = "./assets/to_send";
    let local_to_serve_dir:   &str = "./assets/to_serve";
    let local_pbs_path:       &str = "./assets/run.pbs";
    let local_test_file_path: &str = "meow.txt";

    // [ Sync Local Changes ]
    // Start by copying our PBS file to Metis to ensure it's up to date
    copy_file (
        username,
        hostname,

        SSHPath::Local(local_pbs_path),
        SSHPath::Remote(metis_pbs_path)
    ).await
        .context("Failed to copy file!")?;

    // [ File Upload ]
    // First, cryptographically hash and prepare the file
    let (sha256, extension) = move_to_send( local_test_file_path )
        .await
        .context("Couldn't move to send!")?;

    // Second, move the file to Metis
    println!("New filename: '{sha256}.{extension}'");
    copy_file(
        username,
        hostname,

        SSHPath::Local( &format!("{}/{}.{}", local_to_send_dir, sha256, extension ) ),
        SSHPath::Remote( &format!("{}/{}.{}", metis_inputs_path, sha256, extension ) )
    ).await
        .context("Failed to copy new file to Metis!")?;

    // [ Script Launch ]
    // Submit our PBS script
    let pbs_id: PBSId = metis_qsub (
        username,
        hostname,

        metis_pbs_path,
        vec!("-v", &format!("SHA256='{sha256}',EXTENSION='{extension}'"))
    ).await
        .context("Couldn't run `qsub` on Metis!")?;

    println!("Job ID: '{}'", pbs_id);

    // [ Output Await ]
    while !metis_output_exists(
                username,
                hostname,
                &sha256,
                &extension
            ).await
                .context("Failed to check existence of file on Metis!")?
    {
        println!("File does not exist! Trying again in 5 seconds.");
        sleep(Duration::from_secs(5)).await;
    }

    println!("File exists! :3");

    // [ Output Retrieval ]
    // First, create the directory if it doesn't exist
    let output_to_serve_path = format!("{}/{}", local_to_serve_dir, sha256);
    if !tokio::fs::try_exists( &output_to_serve_path )
            .await.unwrap_or(false)
    {
        tokio::fs::create_dir( &output_to_serve_path )
            .await
            .context("Failed to create output directory locally! Does it exist?")?;
    }

    copy_file(
        username,
        hostname,

        SSHPath::Remote( &format!("{}/{}/output.{}", metis_outputs_path, sha256, extension) ),
        SSHPath::Local( &format!("{}/output.{}", output_to_serve_path, extension) )
    ).await
        .context("Could not copy file back to host!")?;

    Ok(())
}

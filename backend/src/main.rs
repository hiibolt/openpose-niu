mod fs;
mod metis;

// SSH Constants
const USERNAME: &str = "z1994244";
const HOSTNAME: &str = "metis.niu.edu";

// Local Path Constants
const PUBLIC_PATH:  &str = "./public";    
const PBS_PATH:     &str = "./assets/run.pbs";
const PBS_JOB_NAME: &str = "testing";

// Metis Path Constants
const METIS_PBS_PATH:     &str = "/lstr/sahara/zwlab/jw/openpose-api/run.pbs";
const METIS_OUTPUTS_PATH: &str = "/lstr/sahara/zwlab/jw/openpose-api/outputs";


use crate::metis::{ PBSId, metis_qsub, metis_output_exists };
use crate::fs::{ SSHPath, copy_file, move_file, sha256_digest_bytes }; 

use std::path::Path;

use anyhow::{ Context, Result, anyhow };
use tokio::time::{ sleep, Duration };
use tracing::{ info, warn };

#[tokio::main]
async fn main() -> Result<()> {
    // We need prettier printing without in-house macros,
    //  so I have opted to use Tokio's tracing
    let subscriber = tracing_subscriber::fmt()
            .compact()
            .without_time()
            .with_file(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .finish();
    tracing::subscriber::set_global_default(subscriber)?;


    // [ Sync Local Changes ]
    // Start by copying our PBS file to Metis to ensure it's up to date
    copy_file (
        USERNAME,
        HOSTNAME,

        SSHPath::Local  ( PBS_PATH       ),
        SSHPath::Remote ( METIS_PBS_PATH ),
        false
    ).await
        .context("Failed to copy file!")?;

    // [ File Migration ]
    // First, create the file path representation and get the
    //  file extension
    let input_file_path_str:     &str  = "./meow.txt";
    let input_file_path:         &Path = Path::new ( input_file_path_str );
    let input_file_extension:    &str  = input_file_path.extension()
        .ok_or( anyhow!("File was missing an extension!") )?
        .to_str()
        .ok_or( anyhow!("File was invalid Unicode!") )?;
    let input_file_new_filename: String = format!("input.{}", input_file_extension); 

    // Second, read its bytes and create a SHA256 digest
    let input_file_bytes: Vec<u8> = tokio::fs::read ( input_file_path )
        .await
        .context("Couldn't read input file!")?;
    let input_file_sha256: String = sha256_digest_bytes ( &input_file_bytes );

    // Third, create the directory
    let serve_dir_path_str = format!("{}/{}", PUBLIC_PATH, input_file_sha256);
    if !tokio::fs::try_exists( &serve_dir_path_str )
        .await.unwrap_or(false)
    {
        tokio::fs::create_dir( &serve_dir_path_str )
            .await
            .context("Failed to create output directory locally! Does it exist?")?;
    }

    // Fourth, move the file into the directory
    move_file (
        input_file_path_str,
        &format!("{}/{}", serve_dir_path_str, input_file_new_filename)
    ).await
        .context("Failed to move the file!")?;

    // Second, move the folder to Metis
    copy_file(
        USERNAME,
        HOSTNAME,

        SSHPath::Local  ( &serve_dir_path_str ),
        SSHPath::Remote ( METIS_OUTPUTS_PATH ),
        true
    ).await
        .context("Failed to copy new file to Metis!")?;

    // [ Script Launch ]
    // Submit our PBS script
    let pbs_job_id: PBSId = metis_qsub (
        USERNAME,
        HOSTNAME,

        METIS_PBS_PATH,
        vec!("-v", &format!("SHA256='{input_file_sha256}',EXTENSION='{input_file_extension}'"))
    ).await
        .context("Couldn't run `qsub` on Metis!")?;

    info!("Job ID: '{}'", pbs_job_id);

    // [ Output Await ]
    while !metis_output_exists(
                USERNAME,
                HOSTNAME,

                &pbs_job_id,
                PBS_JOB_NAME
            ).await
                .context("Failed to check existence of file on Metis!")?
    {
        warn!("File does not exist! Trying again in 5 seconds.");
        sleep(Duration::from_secs(5)).await;
    }

    info!("File exists! :3");

    // [ Output Retrieval ]
    // Delete the current folder to ensure we know what Metis got
    tokio::fs::remove_dir_all( &serve_dir_path_str )
        .await
        .context("Couldn't remove current serve directory! Does it exist?")?;

    // Move the log into the output directory
    let metis_hash_output_dir = format!("{}/{}", METIS_OUTPUTS_PATH, input_file_sha256);
    let pbs_job_number = pbs_job_id
        .split('.')
        .next()
        .ok_or(anyhow!("PBS job ID was missing job number! It must be in the form `<n>.cm`."))?;
    copy_file(
        USERNAME,
        HOSTNAME,

        SSHPath::Remote( &format!("{PBS_JOB_NAME}.o{pbs_job_number}") ),
        SSHPath::Remote( &metis_hash_output_dir ),
        false
    ).await
        .context("Couldn't move the logfile around Metis!")?;

    // Bring the entire directory back from Metis
    copy_file(
        USERNAME,
        HOSTNAME,

        SSHPath::Remote( &metis_hash_output_dir ),
        SSHPath::Local( &serve_dir_path_str ),
        true
    ).await
        .context("Could not copy file back to host!")?;

    Ok(())
}

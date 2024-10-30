use tokio::process::Command;
use anyhow::{ bail, Context, Result };
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use sha2::{ Sha256, Digest };
use tracing::warn;

/// Digests a byte vector into a string representation
///  of a SHA256 hash.
///
/// # Args
/// * `bytes`: The byte vector to digest.
///
/// # Returns
/// * A stringified SHA256 digest of the byte vector.
pub fn sha256_digest_bytes (
    bytes: &Vec<u8>
) -> String {
    let mut hasher = Sha256::new();

    hasher.update( bytes );

    // The result comes out as hexadecimal, so we must
    //  convert it
    format!( "{:x}", hasher.finalize( ))
}

/// Moves a file from `from_path` to `to_path`.
///
/// # Args
/// * `from_path`: The path to the original file
/// * `to_path`: The path to the new file
///
/// # Notes
/// * Returns okay if the file already exists, doesn't
///    check the file's contents. Accordingly, pair with
///    a hashing function of some kind.
pub async fn move_file (
    from_path_str: &str,
    to_path_str:   &str
) -> Result<()> {
    // It's rather difficult to use a `&Path` upfront
    let from_path: &Path = Path::new( from_path_str );
    let to_path:   &Path = Path::new( to_path_str   );

    // Read the file contents
    let byte_vec: Vec<u8> = tokio::fs::read ( from_path )
        .await
        .context("Couldn't read the source file!")?;

    // Delete the file
    tokio::fs::remove_file( from_path )
        .await
        .context("Failed to remove source file!")?;

    // Check if that filename already exists
    if tokio::fs::try_exists ( to_path )
        .await.unwrap_or(false)
    {
        warn!("File already exists, skipping!");

        return Ok(());
    }

    // Create the new file path
    let mut to_file = File::create( to_path )
        .await
        .context("Couldn't create the new file!")?;

    // Write the contents back to it and flush
    to_file.write(&byte_vec)
        .await
        .context("Couldn't write contents to the new file!")?;
    to_file.flush()
        .await
        .context("Couln't flush contents to the new file!")?;

    Ok(())
}
pub enum SSHPath<'a> {
    Local  (&'a str),
    Remote (&'a str)
}
pub async fn copy_file<'a> (
    username:         &str,
    hostname:         &str,

    source:           SSHPath<'a>,
    destination:      SSHPath<'a>,
    directory:        bool
) -> Result<String> {
    let mut command = Command::new("scp");

    if directory {
        command.arg("-r");
    }

    match source {
        SSHPath::Remote(remote_file_path) => {
            match destination {
                SSHPath::Local(local_file_path) => {
                    command
                        .arg(format!("{username}@{hostname}:{}", remote_file_path ))
                        .arg(local_file_path);
                },
                SSHPath::Remote(new_remote_file_path) => {
                    command
                        .arg(format!("{username}@{hostname}:{}", remote_file_path ))
                        .arg(format!("{username}@{hostname}:{}", new_remote_file_path ));
                }
            }
        },
        SSHPath::Local(local_file_path) => {
            if let SSHPath::Remote(remote_file_path) = destination {
                command
                    .arg(local_file_path)
                    .arg(format!("{username}@{hostname}:{}", remote_file_path));
            } else {
                bail!("Must have differing SSHPath types!");
            }
        }
    }

    let output = command.output()
        .await
        .context("Failed to execute `scp` command!")?;

    let stdout: String = String::from_utf8 ( output.stdout )
        .context("Standard output contained invalid UTF-8!")?;
    let stderr: String = String::from_utf8 ( output.stderr )
        .context("Standard error contained invalid UTF-8!")?;

    if !stderr.is_empty() {
        bail!("Got error output: {stderr}");
    }

    Ok(stdout)
}

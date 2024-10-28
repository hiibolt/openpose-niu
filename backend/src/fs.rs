use tokio::process::Command;
use anyhow::{ bail, Context, Result, anyhow };
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use sha2::{ Sha256, Digest };

/// Moves a file to the `to_send` folder by calculating and 
///  changing the name to be the SHA256 hash of the contents.
///
/// # Args
/// The (full) filename you would like to move
///
/// # Returns
/// The new filename and the extension.
pub async fn move_to_send (
    file: &str
) -> Result<(String, String)> {
    let file_path_str = format!("./assets/to_process/{}", file);
    let file_path = Path::new(&file_path_str);

    // Read the file contents
    let bytes: Vec<u8> = tokio::fs::read ( file_path )
        .await
        .context("Couldn't read the source file!")?;

    // Delete the file
    tokio::fs::remove_file( Path::new( file_path ) )
        .await
        .context("Failed to remove source file!")?;

    // Create our hash
    let mut hasher = Sha256::new();
    hasher.update(bytes.clone());
    let sha256 = format!("{:x}", hasher.finalize());

    // Get the file's extension
    let extension: &str = file_path.extension()
        .ok_or(anyhow!("File did not have extension!"))?
        .to_str()
        .ok_or(anyhow!("File's extension was not valid Unicode!"))?;

    // Determine the new filename
    let new_file_name = format!("{}.{}", sha256, extension);
    let new_file_path_str: String = format!("./assets/to_send/{}", new_file_name);
    let new_file_path: &Path = Path::new(&new_file_path_str);

    // Check if that filename already exists
    if tokio::fs::try_exists ( new_file_path )
        .await.unwrap_or(false)
    {
        println!("File already exists cryptographically with the same extension, skipping!");

        return Ok((sha256, extension.into()));
    }

    // Create the new file path
    let mut new_file = File::create( new_file_path )
        .await
        .context("Couldn't create the new file!")?;

    // Write the contents back to it and flush
    new_file.write(&bytes)
        .await
        .context("Couldn't write contents to the new file!")?;
    new_file.flush()
        .await
        .context("Couln't flush contents to the new file!")?;

    Ok((sha256, extension.into()))
}
pub enum SSHPath<'a> {
    Local(&'a str),
    Remote(&'a str)
}
pub async fn copy_file<'a> (
    username:         &str,
    hostname:         &str,

    source:           SSHPath<'a>,
    destination:      SSHPath<'a>
) -> Result<String> {
    let output;
    match source {
        SSHPath::Remote(remote_file_path) => {
            if let SSHPath::Local(local_file_path) = destination {
                output = Command::new("scp")
                    .arg(&format!("{username}@{hostname}:{}", remote_file_path ))
                    .arg(local_file_path)
                    .output()
                    .await
                    .context("Failed to execute `scp` command!")?;
            } else {
                bail!("Must have differing SSHPath types!");
            }
        },
        SSHPath::Local(local_file_path) => {
            if let SSHPath::Remote(remote_file_path) = destination {
                output = Command::new("scp")
                    .arg(local_file_path)
                    .arg(&format!("{username}@{hostname}:{}", remote_file_path))
                    .output()
                    .await
                    .context("Failed to execute `scp` command!")?;
            } else {
                bail!("Must have differing SSHPath types!");
            }
        }
    }

    let stdout: String = String::from_utf8 ( output.stdout )
        .context("Standard output contained invalid UTF-8!")?;
    let stderr: String = String::from_utf8 ( output.stderr )
        .context("Standard error contained invalid UTF-8!")?;

    if stderr.len() > 0 {
        bail!("Got error output: {stderr}");
    }

    Ok(stdout)
}

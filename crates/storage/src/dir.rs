use std::{env, fs, io, path::PathBuf};

use directories::ProjectDirs;
use tempfile::TempDir;
use tracing::debug;

pub fn setup_data_dir(
    app_name: &str,
    data_dir: Option<PathBuf>,
    ephemeral: bool,
) -> io::Result<PathBuf> {
    if ephemeral {
        return create_temp_dir(app_name, data_dir).map(TempDir::keep);
    }
    let data_dir = match data_dir {
        Some(data_dir) => data_dir,
        None => ProjectDirs::from("", "", app_name)
            .map(|proj_dirs| proj_dirs.data_local_dir().to_path_buf())
            .ok_or_else(|| io::Error::other("No valid default directory."))?,
    };
    fs::create_dir_all(&data_dir)?;
    Ok(data_dir)
}

pub fn create_temp_dir(app_name: &str, root: Option<PathBuf>) -> io::Result<TempDir> {
    let temp_dir = root.unwrap_or_else(env::temp_dir).join(app_name);
    debug!("Creating temp dir: {temp_dir:?}");
    fs::create_dir_all(&temp_dir)?;
    TempDir::new_in(&temp_dir)
}

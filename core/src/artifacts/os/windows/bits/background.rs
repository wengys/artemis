use super::{
    carve::{carve_bits, WinBits},
    error::BitsError,
    files::get_files,
    jobs::{get_jobs, get_legacy_jobs},
};
use crate::{
    artifacts::os::windows::{accounts::parser::get_users, ese::parser::grab_ese_tables},
    filesystem::{files::is_file, ntfs::raw_files::raw_read_file},
};
use common::windows::{BitsInfo, WindowsBits};
use log::error;

/**
 * Parse modern version (Win10+) of BITS which is an ESE database by dumping the `Jobs` and `Files` tables and parsing their contents  
 */
pub(crate) fn parse_ese_bits(bits_path: &str, carve: bool) -> Result<WindowsBits, BitsError> {
    let tables = vec![String::from("Jobs"), String::from("Files")];
    // Dump the Jobs and Files tables from the BITS database
    let ese_results = grab_ese_tables(bits_path, &tables);
    let bits_tables = match ese_results {
        Ok(results) => results,
        Err(err) => {
            error!("[bits] Failed to parse ESE file: {err:?}");
            return Err(BitsError::ParseEse);
        }
    };

    let jobs = if let Some(values) = bits_tables.get("Jobs") {
        values
    } else {
        return Err(BitsError::MissingJobs);
    };

    let jobs_info = get_jobs(jobs)?;

    let files = if let Some(values) = bits_tables.get("Files") {
        values
    } else {
        return Err(BitsError::MissingFiles);
    };

    let files_info = get_files(files)?;
    let mut bits_info: Vec<BitsInfo> = Vec::new();
    let users = get_users().unwrap_or_default();

    for job in &jobs_info {
        for file in &files_info {
            if job.file_id == file.file_id {
                let bit_info = BitsInfo {
                    job_id: job.job_id.clone(),
                    file_id: job.file_id.clone(),
                    owner_sid: job.owner_sid.clone(),
                    username: users
                        .get(&job.owner_sid.clone())
                        .unwrap_or(&String::new())
                        .to_string(),
                    created: job.created,
                    modified: job.modified,
                    completed: job.completed,
                    expiration: job.expiration,
                    files_total: file.files_transferred,
                    bytes_downloaded: file.download_bytes_size,
                    bytes_tranferred: file.trasfer_bytes_size,
                    job_name: job.job_name.clone(),
                    job_description: job.job_description.clone(),
                    job_command: job.job_command.clone(),
                    job_arguements: job.job_arguements.clone(),
                    error_count: job.error_count,
                    job_type: job.job_type.clone(),
                    job_state: job.job_state.clone(),
                    priority: job.priority.clone(),
                    flags: job.flags.clone(),
                    http_method: job.http_method.clone(),
                    full_path: file.full_path.clone(),
                    filename: file.filename.clone(),
                    target_path: job.target_path.clone(),
                    tmp_file: file.tmp_fullpath.clone(),
                    volume: file.volume.clone(),
                    url: file.url.clone(),
                    timeout: job.timeout,
                    retry_delay: job.retry_delay,
                    transient_error_count: job.transient_error_count,
                    acls: job.acls.clone(),
                    additional_sids: job.additional_sids.clone(),
                    carved: false,
                };
                bits_info.push(bit_info);
            }
        }
    }

    let mut windows_bits = WindowsBits {
        bits: bits_info,
        carved_jobs: Vec::new(),
        carved_files: Vec::new(),
    };
    // If we are carving and since this is ESE bits we currently do not combine job and file info
    if carve {
        let is_legacy = false;
        let read_result = raw_read_file(bits_path);
        if read_result.is_ok() {
            let (_carved_bits, mut carved_jobs, mut carved_files) =
                parse_carve(&read_result.unwrap_or_default(), is_legacy);
            windows_bits.carved_jobs.append(&mut carved_jobs);
            windows_bits.carved_files.append(&mut carved_files);
        } else {
            error!(
                "[bits] Could not read {bits_path} for carving: {:?}",
                read_result.unwrap_err()
            );
        }
    }
    Ok(windows_bits)
}

/**
 * Parse older version (pre-Win10) of BITS which is a custom binary format
 */
pub(crate) fn parse_legacy_bits(systemdrive: &char, carve: bool) -> Result<WindowsBits, BitsError> {
    let mut bits_path =
        format!("{systemdrive}:\\ProgramData\\Microsoft\\Network\\Downloader\\qmgr0.dat");

    let mut windows_bits = WindowsBits {
        bits: Vec::new(),
        carved_jobs: Vec::new(),
        carved_files: Vec::new(),
    };
    if is_file(&bits_path) {
        let mut results = legacy_bits(&bits_path, carve)?;
        windows_bits.bits.append(&mut results.bits);
        windows_bits.carved_files.append(&mut results.carved_files);
        windows_bits.carved_jobs.append(&mut results.carved_jobs);
    }
    // Legacy BITS has two (2) files
    bits_path = format!("{systemdrive}:\\ProgramData\\Microsoft\\Network\\Downloader\\qmgr1.dat");
    if is_file(&bits_path) {
        let mut results = legacy_bits(&bits_path, carve)?;
        windows_bits.bits.append(&mut results.bits);
        windows_bits.carved_files.append(&mut results.carved_files);
        windows_bits.carved_jobs.append(&mut results.carved_jobs);
    }

    Ok(windows_bits)
}

/// Parse the older BITS file
pub(crate) fn legacy_bits(path: &str, carve: bool) -> Result<WindowsBits, BitsError> {
    let mut windows_bits = WindowsBits {
        bits: Vec::new(),
        carved_jobs: Vec::new(),
        carved_files: Vec::new(),
    };
    let read_results = raw_read_file(path);
    let bits_data = match read_results {
        Ok(results) => results,
        Err(err) => {
            error!("[bits] Could not read file {path}: {err:?}");
            return Err(BitsError::ReadFile);
        }
    };
    let mut bits = get_legacy_jobs(&bits_data)?;
    windows_bits.bits.append(&mut bits);

    if carve {
        let is_legacy = false;
        let (mut carved_bits, mut carved_jobs, mut carved_files) =
            parse_carve(&bits_data, is_legacy);
        windows_bits.carved_jobs.append(&mut carved_jobs);
        windows_bits.carved_files.append(&mut carved_files);
        windows_bits.bits.append(&mut carved_bits);
    }
    Ok(windows_bits)
}

/**
 * When BITS entries are deleted the data is not actually removed from the file  
 * This makes it possible to carve out older entries, however some parts of the old entries may be overwritten by new data
 * Carving is **best effort**  
 * For BITS in ESE format (Win10+) BITS jobs and files are separate tables but since we are scanning the whole ESE db  
 * we do not merge the jobs and file info since we cannot determine what links the tables
 */
fn parse_carve(data: &[u8], is_legacy: bool) -> WinBits {
    let results = carve_bits(data, is_legacy);
    match results {
        Ok((_, bits)) => bits,
        Err(_err) => {
            error!("[bits] Could not carve BITS data");
            (Vec::new(), Vec::new(), Vec::new())
        }
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::parse_ese_bits;
    use crate::{
        artifacts::os::windows::bits::background::{legacy_bits, parse_carve, parse_legacy_bits},
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_ese_bits() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");
        let results = parse_ese_bits(test_location.to_str().unwrap(), false).unwrap();
        assert_eq!(results.bits.len(), 1);
    }

    #[test]
    fn test_parse_legacy_bits() {
        let results = parse_legacy_bits(&'C', false).unwrap();
        assert_eq!(results.bits.is_empty(), true);
    }

    #[test]
    fn test_legacy_bits() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\bits\\win81\\qmgr0.dat");
        let results = legacy_bits(&test_location.to_str().unwrap(), false).unwrap();
        assert_eq!(results.bits.len(), 1);
    }

    #[test]
    fn test_parse_carve() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ese/win10/qmgr.db");
        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, jobs, files) = parse_carve(&data, false);
        assert_eq!(jobs.len(), 86);
        assert_eq!(files.len(), 41);
    }
}
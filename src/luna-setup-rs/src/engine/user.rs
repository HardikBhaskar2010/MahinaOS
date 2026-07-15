use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::ffi::CString;

#[derive(Debug)]
pub enum UserError {
    FileWriteFailed(String),
    CryptFailed,
}

fn hash_password(password: &str) -> Result<String, UserError> {
    let pass_c = CString::new(password).map_err(|_| UserError::CryptFailed)?;
    // Use $6$ for SHA-512 crypt. Generate a fixed salt for simplicity, or in a real scenario, randomly generate one.
    let salt_c = CString::new("$6$lunasalt$").map_err(|_| UserError::CryptFailed)?;
    
    unsafe {
        let hash_ptr = libc::crypt(pass_c.as_ptr(), salt_c.as_ptr());
        if hash_ptr.is_null() {
            return Err(UserError::CryptFailed);
        }
        let hash_c_str = std::ffi::CStr::from_ptr(hash_ptr);
        Ok(hash_c_str.to_string_lossy().into_owned())
    }
}

pub fn create_user(target_dir: &str, username: &str, full_name: &str, password: &str) -> Result<(), UserError> {
    let hashed_pw = hash_password(password)?;
    
    let passwd_path = Path::new(target_dir).join("etc/passwd");
    let shadow_path = Path::new(target_dir).join("etc/shadow");
    let group_path = Path::new(target_dir).join("etc/group");
    
    // In a real system, we would parse and find the next available UID. We'll use 1000 for the first user.
    let uid = 1000;
    
    let passwd_entry = format!("{}:x:{}:{}:{},,,:/home/{}:/bin/luna-shell\n", username, uid, uid, full_name, username);
    let shadow_entry = format!("{}:{}:19500:0:99999:7:::\n", username, hashed_pw);
    let group_entry = format!("{}:x:{}:\n", username, uid);
    
    // Append to passwd
    let mut p_file = OpenOptions::new().create(true).append(true).open(&passwd_path).map_err(|e| UserError::FileWriteFailed(e.to_string()))?;
    p_file.write_all(passwd_entry.as_bytes()).map_err(|e| UserError::FileWriteFailed(e.to_string()))?;

    // Append to shadow
    let mut s_file = OpenOptions::new().create(true).append(true).open(&shadow_path).map_err(|e| UserError::FileWriteFailed(e.to_string()))?;
    s_file.write_all(shadow_entry.as_bytes()).map_err(|e| UserError::FileWriteFailed(e.to_string()))?;
    
    // Append to group
    let mut g_file = OpenOptions::new().create(true).append(true).open(&group_path).map_err(|e| UserError::FileWriteFailed(e.to_string()))?;
    g_file.write_all(group_entry.as_bytes()).map_err(|e| UserError::FileWriteFailed(e.to_string()))?;
    
    // Create home directory
    let home_dir = Path::new(target_dir).join(format!("home/{}", username));
    std::fs::create_dir_all(&home_dir).map_err(|e| UserError::FileWriteFailed(e.to_string()))?;
    // Note: To properly set ownership of the home directory, we would use chown in a real implementation.
    
    Ok(())
}

pub fn set_hostname(target_dir: &str, hostname: &str) -> Result<(), UserError> {
    let hostname_path = Path::new(target_dir).join("etc/hostname"); // often /etc/hostname or /etc/luna/hostname
    let luna_hostname_path = Path::new(target_dir).join("etc/luna/hostname");
    
    std::fs::write(&hostname_path, format!("{}\n", hostname)).map_err(|e| UserError::FileWriteFailed(e.to_string()))?;
    std::fs::write(&luna_hostname_path, format!("{}\n", hostname)).unwrap_or(()); // Fallback for Luna's custom location
    
    Ok(())
}

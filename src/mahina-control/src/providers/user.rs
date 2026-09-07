use crate::error::ControlError;
use crate::protocol::UserEntry;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub const DEFAULT_PASSWD_PATH: &str = "/etc/passwd";
pub const DEFAULT_GROUP_PATH: &str = "/etc/group";

#[derive(Debug, Clone)]
pub struct UserProvider {
    passwd_path: PathBuf,
    group_path: PathBuf,
}

impl UserProvider {
    pub fn new() -> Self {
        Self {
            passwd_path: PathBuf::from(DEFAULT_PASSWD_PATH),
            group_path: PathBuf::from(DEFAULT_GROUP_PATH),
        }
    }

    pub fn with_paths(passwd_path: impl AsRef<Path>, group_path: impl AsRef<Path>) -> Self {
        Self {
            passwd_path: passwd_path.as_ref().to_path_buf(),
            group_path: group_path.as_ref().to_path_buf(),
        }
    }

    fn load_groups(&self) -> (HashMap<u32, String>, HashMap<String, Vec<String>>) {
        let mut gid_to_name: HashMap<u32, String> = HashMap::new();
        let mut user_to_groups: HashMap<String, Vec<String>> = HashMap::new();

        let file = match File::open(&self.group_path) {
            Ok(f) => f,
            Err(_) => return (gid_to_name, user_to_groups),
        };

        let reader = BufReader::new(file);
        for line_res in reader.lines() {
            let line = match line_res {
                Ok(l) => l,
                Err(_) => continue,
            };
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // format: group_name:password:gid:user_list
            let parts: Vec<&str> = trimmed.split(':').collect();
            if parts.len() < 3 {
                continue;
            }

            let group_name = parts[0].to_string();
            if let Ok(gid) = parts[2].parse::<u32>() {
                gid_to_name.insert(gid, group_name.clone());
            }

            if parts.len() >= 4 && !parts[3].is_empty() {
                for user in parts[3].split(',') {
                    let u = user.trim().to_string();
                    if !u.is_empty() {
                        user_to_groups.entry(u).or_default().push(group_name.clone());
                    }
                }
            }
        }

        (gid_to_name, user_to_groups)
    }

    pub fn list(&self) -> Result<Vec<UserEntry>, ControlError> {
        if !self.passwd_path.exists() {
            return Err(ControlError::NotFound(format!(
                "Passwd file not found at {:?}",
                self.passwd_path
            )));
        }

        let (gid_to_name, mut user_to_groups) = self.load_groups();

        let file = File::open(&self.passwd_path).map_err(|e| {
            ControlError::Io(format!("Failed to read passwd file at {:?}: {}", self.passwd_path, e))
        })?;

        let reader = BufReader::new(file);
        let mut users = Vec::new();

        for line_res in reader.lines() {
            let line = line_res?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            // format: username:password:uid:gid:gecos:home:shell
            let parts: Vec<&str> = trimmed.split(':').collect();
            if parts.len() < 7 {
                continue;
            }

            let username = parts[0].to_string();
            let uid = match parts[2].parse::<u32>() {
                Ok(id) => id,
                Err(_) => continue,
            };
            let gid = match parts[3].parse::<u32>() {
                Ok(id) => id,
                Err(_) => continue,
            };
            let comment = parts[4].to_string();
            let home_dir = parts[5].to_string();
            let shell = parts[6].to_string();

            let mut groups = Vec::new();
            if let Some(primary_group) = gid_to_name.get(&gid) {
                groups.push(primary_group.clone());
            }
            if let Some(secondary) = user_to_groups.remove(&username) {
                for g in secondary {
                    if !groups.contains(&g) {
                        groups.push(g);
                    }
                }
            }
            groups.sort();

            users.push(UserEntry {
                username,
                uid,
                gid,
                comment,
                home_dir,
                shell,
                groups,
            });
        }

        Ok(users)
    }
}

impl Default for UserProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_user_provider_parser() {
        let passwd_content = "\
root:x:0:0:Superuser:/root:/bin/sh
# System account
luna:x:1000:1000:Luna User:/home/luna:/bin/sh
luna-ai:x:950:950:LunaAI Agent:/var/lib/luna-ai:/usr/bin/nologin
";

        let group_content = "\
root:x:0:
wheel:x:10:luna,admin
luna:x:1000:
luna-ai:x:950:
";

        let dir = std::env::temp_dir();
        let passwd_path = dir.join("test_passwd");
        let group_path = dir.join("test_group");

        {
            let mut f1 = File::create(&passwd_path).unwrap();
            f1.write_all(passwd_content.as_bytes()).unwrap();
            let mut f2 = File::create(&group_path).unwrap();
            f2.write_all(group_content.as_bytes()).unwrap();
        }

        let provider = UserProvider::with_paths(&passwd_path, &group_path);
        let users = provider.list().expect("list failed");

        assert_eq!(users.len(), 3);

        let root = &users[0];
        assert_eq!(root.username, "root");
        assert_eq!(root.uid, 0);
        assert_eq!(root.gid, 0);
        assert_eq!(root.groups, vec!["root".to_string()]);

        let luna = &users[1];
        assert_eq!(luna.username, "luna");
        assert_eq!(luna.uid, 1000);
        assert_eq!(luna.home_dir, "/home/luna");
        assert_eq!(luna.groups, vec!["luna".to_string(), "wheel".to_string()]);

        let ai = &users[2];
        assert_eq!(ai.username, "luna-ai");
        assert_eq!(ai.uid, 950);
        assert_eq!(ai.shell, "/usr/bin/nologin");

        let _ = std::fs::remove_file(&passwd_path);
        let _ = std::fs::remove_file(&group_path);
    }
}

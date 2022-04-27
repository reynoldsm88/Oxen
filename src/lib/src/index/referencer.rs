use crate::error::OxenError;
use crate::model::{Branch, LocalRepository};
use crate::util;

use rocksdb::{IteratorMode, LogLevel, Options, DB};
use std::path::{Path, PathBuf};
use std::str;

pub const HEAD_FILE: &str = "HEAD";
pub const REFS_DIR: &str = "refs";
pub const DEFAULT_BRANCH: &str = "main";

pub struct Referencer {
    refs_db: DB,
    head_file: PathBuf,
}

impl Referencer {
    pub fn refs_dir(path: &Path) -> PathBuf {
        util::fs::oxen_hidden_dir(path).join(Path::new(REFS_DIR))
    }

    pub fn head_file(path: &Path) -> PathBuf {
        util::fs::oxen_hidden_dir(path).join(Path::new(HEAD_FILE))
    }

    pub fn new(repository: &LocalRepository) -> Result<Referencer, OxenError> {
        let refs_dir = Referencer::refs_dir(&repository.path);
        let head_filename = Referencer::head_file(&repository.path);

        if !head_filename.exists() {
            util::fs::write_to_path(&head_filename, DEFAULT_BRANCH);
        }

        let mut opts = Options::default();
        opts.set_log_level(LogLevel::Error);
        opts.create_if_missing(true);
        Ok(Referencer {
            refs_db: DB::open(&opts, &refs_dir)?,
            head_file: head_filename,
        })
    }

    pub fn new_read_only(repository: &LocalRepository) -> Result<Referencer, OxenError> {
        let refs_dir = Referencer::refs_dir(&repository.path);
        let head_filename = Referencer::head_file(&repository.path);

        if !head_filename.exists() {
            util::fs::write_to_path(&head_filename, DEFAULT_BRANCH);
        }

        let error_if_log_file_exist = false;
        let mut opts = Options::default();
        opts.set_log_level(LogLevel::Error);
        opts.create_if_missing(true);
        Ok(Referencer {
            refs_db: DB::open_for_read_only(&opts, &refs_dir, error_if_log_file_exist)?,
            head_file: head_filename,
        })
    }

    pub fn set_head(&self, name: &str) -> Result<(), OxenError> {
        if self.has_branch(name) {
            util::fs::write_to_path(&self.head_file, name);
            Ok(())
        } else {
            let err = format!("Cannot set head to non-existant ref: {}", name);
            Err(OxenError::basic_str(&err))
        }
    }

    pub fn create_branch(&self, name: &str, commit_id: &str) -> Result<(), OxenError> {
        // Only create branch if it does not exist already
        if self.has_branch(name) {
            let err = format!("Branch already exists: {}", name);
            Err(OxenError::basic_str(&err))
        } else {
            self.set_branch_commit_id(name, commit_id)?;
            Ok(())
        }
    }

    pub fn set_branch_commit_id(&self, name: &str, commit_id: &str) -> Result<(), OxenError> {
        self.refs_db.put(name, commit_id)?;
        Ok(())
    }

    pub fn set_head_commit_id(&self, commit_id: &str) -> Result<(), OxenError> {
        let head_ref = self.read_head_ref()?;
        self.set_branch_commit_id(&head_ref, commit_id)?;
        Ok(())
    }

    pub fn list_branches(&self) -> Result<Vec<Branch>, OxenError> {
        let mut branch_names: Vec<Branch> = vec![];
        let head_ref = self.read_head_ref()?;
        let iter = self.refs_db.iterator(IteratorMode::Start);
        for (key, value) in iter {
            match (str::from_utf8(&*key), str::from_utf8(&*value)) {
                (Ok(key_str), Ok(value)) => {
                    let ref_name = String::from(key_str);
                    let id = String::from(value);
                    branch_names.push(Branch {
                        name: ref_name.clone(),
                        commit_id: id.clone(),
                        is_head: (ref_name == head_ref),
                    });
                }
                _ => {
                    eprintln!("Could not read utf8 val...")
                }
            }
        }
        Ok(branch_names)
    }

    pub fn get_current_branch(&self) -> Result<Branch, OxenError> {
        let ref_name = self.read_head_ref()?;
        let id = self.get_commit_id(&ref_name)?;
        Ok(Branch {
            name: ref_name,
            commit_id: id,
            is_head: true,
        })
    }

    pub fn has_branch(&self, name: &str) -> bool {
        self.get_commit_id(name).is_ok()
    }

    pub fn get_commit_id(&self, name: &str) -> Result<String, OxenError> {
        let bytes = name.as_bytes();
        match self.refs_db.get(bytes) {
            Ok(Some(value)) => Ok(String::from(str::from_utf8(&*value)?)),
            Ok(None) => {
                let err = format!("ref not found: {}", name);
                Err(OxenError::basic_str(&err))
            }
            Err(err) => {
                let err = format!("{}", err);
                Err(OxenError::basic_str(&err))
            }
        }
    }

    pub fn head_commit_id(&self) -> Result<String, OxenError> {
        self.get_commit_id(&self.read_head_ref()?)
    }

    pub fn read_head_ref(&self) -> Result<String, OxenError> {
        util::fs::read_from_path(&self.head_file)
    }
}

#[cfg(test)]
mod tests {
    use crate::error::OxenError;
    use crate::test;

    #[test]
    fn test_default_head() -> Result<(), OxenError> {
        test::run_referencer_test(|referencer| {
            assert_eq!(
                referencer.read_head_ref()?,
                crate::index::referencer::DEFAULT_BRANCH
            );

            Ok(())
        })
    }

    #[test]
    fn test_create_branch_read_no_head() -> Result<(), OxenError> {
        test::run_referencer_test(|referencer| {
            let branch_name = "experiment/cat-dog";
            let commit_id = format!("{}", uuid::Uuid::new_v4());
            referencer.create_branch(branch_name, &commit_id)?;

            if referencer.head_commit_id().is_ok() {
                panic!("Should not be able to read head!")
            }

            Ok(())
        })
    }

    #[test]
    fn test_create_branch_set_head() -> Result<(), OxenError> {
        test::run_referencer_test(|referencer| {
            let branch_name = "experiment/cat-dog";
            let commit_id = format!("{}", uuid::Uuid::new_v4());
            referencer.create_branch(branch_name, &commit_id)?;
            referencer.set_head(branch_name)?;
            assert_eq!(referencer.head_commit_id()?, commit_id);

            Ok(())
        })
    }

    #[test]
    fn test_referencer_list_branches_empty() -> Result<(), OxenError> {
        test::run_referencer_test(|referencer| {
            let branches = referencer.list_branches()?;
            assert_eq!(branches.len(), 0);

            Ok(())
        })
    }

    #[test]
    fn test_referencer_list_branches_one() -> Result<(), OxenError> {
        test::run_referencer_test(|referencer| {
            let name = "my-branch";
            let commit_id = format!("{}", uuid::Uuid::new_v4());
            referencer.create_branch(name, &commit_id)?;
            let branches = referencer.list_branches()?;
            assert_eq!(branches.len(), 1);
            assert_eq!(branches[0].name, name);
            assert_eq!(branches[0].commit_id, commit_id);

            Ok(())
        })
    }

    #[test]
    fn test_referencer_list_branches_many() -> Result<(), OxenError> {
        test::run_referencer_test(|referencer| {
            referencer.create_branch("name_1", "1")?;
            referencer.create_branch("name_2", "2")?;
            referencer.create_branch("name_3", "3")?;
            let branches = referencer.list_branches()?;
            assert_eq!(branches.len(), 3);

            Ok(())
        })
    }

    #[test]
    fn test_referencer_create_branch_same_name() -> Result<(), OxenError> {
        test::run_referencer_test(|referencer| {
            referencer.create_branch("name", "1")?;

            if referencer.create_branch("name", "2").is_ok() {
                panic!("Should not be able to read head!")
            }

            // We should still only have one branch
            let branches = referencer.list_branches()?;
            assert_eq!(branches.len(), 1);

            Ok(())
        })
    }

    // Create branch (based on current commit, fail if no commit yet)
    // `git branch my_branch`

    // List all branches
    // `git branch -a`

    // Checkout branch (switches all files too, and reverts modifications, this the money)
    // `git checkout my_branch`

    // Create branch and check it out
    // git checkout -b my_branchie_poo

    // TODO on commit, make all them hard links to our mirror directory....
    // maybe we compress and hash? we'll see, don't compress at the start, KISS
}

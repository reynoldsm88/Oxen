use crate::constants::{HEAD_FILE, REFS_DIR};
use crate::db;
use crate::error::OxenError;
use crate::index::RefDBReader;
use crate::model::{Branch, LocalRepository};
use crate::util;

use rocksdb::{IteratorMode, DB};
use std::path::{Path, PathBuf};
use std::str;

pub struct RefWriter {
    refs_db: DB,
    head_file: PathBuf,
}

impl RefWriter {
    pub fn new(repository: &LocalRepository) -> Result<RefWriter, OxenError> {
        let refs_dir = util::fs::oxen_hidden_dir(&repository.path).join(Path::new(REFS_DIR));
        let head_filename = util::fs::oxen_hidden_dir(&repository.path).join(Path::new(HEAD_FILE));

        let opts = db::opts::default();
        Ok(RefWriter {
            refs_db: DB::open(&opts, refs_dir)?,
            head_file: head_filename,
        })
    }

    pub fn set_head(&self, name: &str) {
        util::fs::write_to_path(&self.head_file, name).expect("Could not write to head");
    }

    pub fn create_branch(&self, name: &str, commit_id: &str) -> Result<Branch, OxenError> {
        // Only create branch if it does not exist already
        log::debug!("create_branch {} -> {}", name, commit_id);
        if self.has_branch(name) {
            let err = format!("Branch already exists: {name}");
            Err(OxenError::basic_str(err))
        } else {
            self.set_branch_commit_id(name, commit_id)?;
            Ok(Branch {
                name: String::from(name),
                commit_id: String::from(commit_id),
                is_head: false,
            })
        }
    }

    pub fn delete_branch(&self, name: &str) -> Result<(), OxenError> {
        if !self.has_branch(name) {
            let err = format!("Branch does not exist: {name}");
            Err(OxenError::basic_str(err))
        } else {
            self.refs_db.delete(name)?;
            Ok(())
        }
    }

    pub fn set_branch_commit_id(&self, name: &str, commit_id: &str) -> Result<(), OxenError> {
        self.refs_db.put(name, commit_id)?;
        Ok(())
    }

    pub fn set_head_commit_id(&self, commit_id: &str) -> Result<(), OxenError> {
        // if we have head ref in HEAD file then write it to that db
        let head_val = self.read_head_ref()?; // could be branch name or commit ID
        if self.has_branch(&head_val) {
            self.set_head_branch_commit_id(commit_id)?;
        } else {
            self.set_head(commit_id);
        }

        Ok(())
    }

    pub fn set_head_branch_commit_id(&self, commit_id: &str) -> Result<(), OxenError> {
        let head_ref = self.read_head_ref()?;
        self.set_branch_commit_id(&head_ref, commit_id)?;
        Ok(())
    }

    pub fn list_branches(&self) -> Result<Vec<Branch>, OxenError> {
        let mut branch_names: Vec<Branch> = vec![];
        let head_ref = self.read_head_ref()?;
        let iter = self.refs_db.iterator(IteratorMode::Start);
        for (key, value) in iter {
            match (str::from_utf8(&key), str::from_utf8(&value)) {
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

    pub fn get_current_branch(&self) -> Result<Option<Branch>, OxenError> {
        let ref_name = self.read_head_ref()?;
        if let Some(id) = RefDBReader::get_commit_id_for_branch(&self.refs_db, &ref_name)? {
            Ok(Some(Branch {
                name: ref_name,
                commit_id: id,
                is_head: true,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn has_branch(&self, name: &str) -> bool {
        RefDBReader::has_branch(&self.refs_db, name)
    }

    pub fn get_branch_by_name(&self, name: &str) -> Result<Option<Branch>, OxenError> {
        let head_commit_id = self.head_commit_id()?;
        match self.get_commit_id_for_branch(name) {
            Ok(Some(commit_id)) => Ok(Some(Branch {
                name: name.to_string(),
                commit_id: commit_id.to_string(),
                is_head: commit_id == head_commit_id,
            })),
            Ok(None) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub fn get_commit_id_for_branch(&self, name: &str) -> Result<Option<String>, OxenError> {
        let bytes = name.as_bytes();
        match self.refs_db.get(bytes) {
            Ok(Some(value)) => Ok(Some(String::from(str::from_utf8(&value)?))),
            Ok(None) => Ok(None),
            Err(err) => {
                let err = format!("{err}");
                Err(OxenError::basic_str(err))
            }
        }
    }

    pub fn head_commit_id(&self) -> Result<String, OxenError> {
        let head_ref = self.read_head_ref()?;
        if let Some(commit_id) = self.get_commit_id_for_branch(&head_ref)? {
            Ok(commit_id)
        } else {
            Ok(head_ref)
        }
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
                crate::constants::DEFAULT_BRANCH_NAME
            );

            Ok(())
        })
    }

    #[test]
    fn test_create_branch_set_head() -> Result<(), OxenError> {
        test::run_referencer_test(|referencer| {
            let branch_name = "experiment/cat-dog";
            let commit_id = format!("{}", uuid::Uuid::new_v4());
            referencer.create_branch(branch_name, &commit_id)?;
            referencer.set_head(branch_name);
            assert_eq!(referencer.head_commit_id()?, commit_id);

            Ok(())
        })
    }

    #[test]
    fn test_ref_writer_list_branches_empty() -> Result<(), OxenError> {
        test::run_referencer_test(|referencer| {
            // always start with a default branch
            let branches = referencer.list_branches()?;
            assert_eq!(branches.len(), 1);

            Ok(())
        })
    }

    #[test]
    fn test_ref_writer_list_branches_one() -> Result<(), OxenError> {
        test::run_referencer_test(|referencer| {
            let name = "my-branch";
            let commit_id = format!("{}", uuid::Uuid::new_v4());
            referencer.create_branch(name, &commit_id)?;
            let branches = referencer.list_branches()?;
            // we always start with "main" branch
            assert_eq!(branches.len(), 2);

            Ok(())
        })
    }

    #[test]
    fn test_ref_writer_list_branches_many() -> Result<(), OxenError> {
        test::run_referencer_test(|referencer| {
            // we always start with a default branch
            referencer.create_branch("name_1", "1")?;
            referencer.create_branch("name_2", "2")?;
            referencer.create_branch("name_3", "3")?;
            let branches = referencer.list_branches()?;
            assert_eq!(branches.len(), 4);

            Ok(())
        })
    }

    #[test]
    fn test_ref_writer_create_branch_same_name() -> Result<(), OxenError> {
        test::run_referencer_test(|referencer| {
            referencer.create_branch("my-fun-name", "1")?;

            if referencer.create_branch("my-fun-name", "2").is_ok() {
                panic!("Should not be able to read head!")
            }

            // We should still only have two branches, default on and this one
            let branches = referencer.list_branches()?;
            assert_eq!(branches.len(), 2);

            Ok(())
        })
    }

    #[test]
    fn test_ref_writer_delete_branch() -> Result<(), OxenError> {
        test::run_referencer_test(|referencer| {
            let name = "my-branch-name";
            referencer.create_branch(name, "1")?;
            let og_branches = referencer.list_branches()?;
            let og_branch_count = og_branches.len();

            // Delete branch
            referencer.delete_branch(name)?;

            // Should have one less branch than after creation
            let branches = referencer.list_branches()?;
            assert_eq!(branches.len(), og_branch_count - 1);

            Ok(())
        })
    }
}

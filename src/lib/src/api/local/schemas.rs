use crate::api;

use crate::error::OxenError;
use crate::index::SchemaReader;
use crate::model::{LocalRepository, Schema};
use crate::util::resource;

pub fn list(repo: &LocalRepository, commit_id: Option<&str>) -> Result<Vec<Schema>, OxenError> {
    log::debug!("api::local::schemas::list for path {:?}", repo.path);
    if let Some(commit_id) = commit_id {
        if let Some(commit) = api::local::commits::commit_from_branch_or_commit_id(repo, commit_id)?
        {
            let schema_reader = SchemaReader::new(repo, &commit.id)?;
            schema_reader.list_schemas()
        } else {
            Err(OxenError::commit_id_does_not_exist(commit_id))
        }
    } else {
        let head_commit = resource::get_head_commit(repo)?;
        let schema_reader = SchemaReader::new(repo, &head_commit.id)?;
        schema_reader.list_schemas()
    }
}

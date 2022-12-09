use crate::app_data::OxenAppData;

use liboxen::api;

use actix_files::NamedFile;
use actix_web::HttpRequest;
use std::path::{Path, PathBuf};

use liboxen::model::LocalRepository;
use liboxen::util;

pub async fn get(req: HttpRequest) -> Result<NamedFile, actix_web::Error> {
    let app_data = req.app_data::<OxenAppData>().unwrap();

    let namespace: &str = req.match_info().get("namespace").unwrap();
    let name: &str = req.match_info().get("repo_name").unwrap();
    let resource: PathBuf = req.match_info().query("resource").parse().unwrap();

    log::debug!("file::get repo name [{}] resource [{:?}]", name, resource,);
    match api::local::repositories::get_by_namespace_and_name(&app_data.path, namespace, name) {
        Ok(Some(repo)) => {
            if let Ok(Some((commit_id, _, filepath))) =
                util::resource::parse_resource(&repo, &resource)
            {
                log::debug!(
                    "dir::get commit_id [{}] and filepath {:?}",
                    commit_id,
                    filepath
                );
                get_file_for_commit_id(&repo, &commit_id, &filepath)
            } else {
                log::debug!("file::get could not find resource from uri {:?}", resource);
                Ok(NamedFile::open("")?)
            }
        }
        Ok(None) => {
            log::debug!("file::get could not find repo with name {}", name);
            Ok(NamedFile::open("")?)
        }
        Err(err) => {
            log::error!("unable to get file {:?}. Err: {}", resource, err);
            Ok(NamedFile::open("")?)
        }
    }
}

fn get_file_for_commit_id(
    repo: &LocalRepository,
    commit_id: &str,
    filepath: &Path,
) -> Result<NamedFile, actix_web::Error> {
    match util::fs::version_path_for_commit_id(repo, commit_id, filepath) {
        Ok(version_path) => {
            log::debug!(
                "get_file_for_commit_id looking for {:?} -> {:?}",
                filepath,
                version_path
            );
            Ok(NamedFile::open(version_path)?)
        }
        Err(err) => {
            log::error!("get_file_for_commit_id get entry err: {:?}", err);
            // gives a 404
            Ok(NamedFile::open("")?)
        }
    }
}

use crate::project::{ProjectHandle, ProjectRequest, api::SerializableProjectResponse};
use crate::server::fs_api::{FsRequest, FsResponse};
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "requestType", rename_all = "camelCase")]
pub enum ServerRequest {
    /// Filesystem operation request
    Filesystem(FsRequest),
    /// Load a project
    LoadProject { path: String },
    /// Unload a project
    UnloadProject { handle: ProjectHandle },
    /// Project-specific request
    ProjectRequest {
        handle: ProjectHandle,
        request: ProjectRequest,
    },
    /// List available projects
    ListAvailableProjects,
    /// List loaded projects
    ListLoadedProjects,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "responseType", rename_all = "camelCase")]
pub enum ServerResponse {
    /// Filesystem operation response
    Filesystem(FsResponse),
    /// Response to LoadProject
    LoadProject { handle: ProjectHandle },
    /// Response to UnloadProject
    UnloadProject,
    /// Response to ProjectRequest
    ///
    /// Uses SerializableProjectResponse which wraps NodeDetail in SerializableNodeDetail
    /// to enable serialization of trait objects.
    ProjectRequest {
        response: SerializableProjectResponse,
    },
    /// Response to ListAvailableProjects
    ListAvailableProjects { projects: Vec<AvailableProject> },
    /// Response to ListLoadedProjects
    ListLoadedProjects { projects: Vec<LoadedProject> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableProject {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedProject {
    pub handle: ProjectHandle,
    pub path: String,
}

use crate::collaboration::{
    CollaborationAccess, LocalSessionHandle, RemoteCompletionIntent, SanitizedSessionMetadata,
};
use percent_encoding::percent_decode_str;
use reqwest::{
    blocking::{Client, Response},
    header::{HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE},
    redirect::Policy,
    StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fmt,
    io::Read,
    ops::Deref,
    sync::{Arc, Mutex},
    time::Duration,
};
use subtle::ConstantTimeEq;
use url::Url;
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

#[cfg(not(test))]
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
#[cfg(test)]
const REQUEST_TIMEOUT: Duration = Duration::from_millis(200);
const MAX_DISCOVERY_BYTES: usize = 16 * 1024;
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_REQUEST_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const MAX_WORKSPACE_URL_BYTES: usize = 4096;
const MAX_CAPABILITY_BYTES: usize = 1024;
const MAX_ACTOR_BYTES: usize = 128;
const MAX_PATH_BYTES: usize = 1024;
const MAX_WORKSPACE_ID_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CapabilityKind {
    Public,
    Read,
    Edit,
}

struct SecretCapability {
    kind: CapabilityKind,
    // This is the long-lived native session copy. It is zeroized when the
    // session is revoked/dropped. HTTP/url libraries still own unavoidable
    // bounded transient request representations during an active call.
    value: Zeroizing<String>,
}

impl SecretCapability {
    fn public() -> Self {
        Self {
            kind: CapabilityKind::Public,
            value: Zeroizing::new(String::new()),
        }
    }

    fn new(kind: CapabilityKind, value: Zeroizing<String>) -> Result<Self, MdsyncTransportError> {
        if value.is_empty() || value.len() > MAX_CAPABILITY_BYTES {
            return Err(MdsyncTransportError::invalid_url(
                "The workspace capability is empty or oversized",
            ));
        }
        Ok(Self { kind, value })
    }

    fn access(&self) -> CollaborationAccess {
        match self.kind {
            CapabilityKind::Public => CollaborationAccess::Public,
            CapabilityKind::Read => CollaborationAccess::Viewer,
            CapabilityKind::Edit => CollaborationAccess::Collaborator,
        }
    }

    fn is_edit(&self) -> bool {
        self.kind == CapabilityKind::Edit
    }

    fn reject_aliases<'a>(
        &self,
        candidates: impl IntoIterator<Item = &'a str>,
    ) -> Result<(), MdsyncTransportError> {
        if self.value.is_empty() {
            return Ok(());
        }

        let secret = self.value.as_bytes();
        let secret_digest = Zeroizing::new(Sha256::digest(secret).to_vec());
        let word_count = secret.len().div_ceil(u64::BITS as usize);
        // Shift-And masks let longer candidates be scanned in work determined
        // only by the already-bounded candidate and capability lengths. The
        // scan never exits at the first match, and all secret-derived buffers
        // are cleared when this boundary check returns.
        let mut masks = Zeroizing::new(vec![0_u64; 256 * word_count]);
        for (index, byte) in secret.iter().copied().enumerate() {
            masks[(byte as usize * word_count) + (index / u64::BITS as usize)] |=
                1_u64 << (index % u64::BITS as usize);
        }
        let target_word = (secret.len() - 1) / u64::BITS as usize;
        let target_mask = 1_u64 << ((secret.len() - 1) % u64::BITS as usize);
        let mut rejected = 0_u64;

        for candidate in candidates {
            if candidate.len() == secret.len() {
                // Preserve the fixed-size digest comparison for whole-field
                // aliases without comparing capability bytes directly.
                let candidate_digest =
                    Zeroizing::new(Sha256::digest(candidate.as_bytes()).to_vec());
                rejected |= u64::from(bool::from(
                    secret_digest.as_slice().ct_eq(&candidate_digest),
                ));
                continue;
            }

            if candidate.len() < secret.len() {
                continue;
            }

            let mut state = Zeroizing::new(vec![0_u64; word_count]);
            let mut matched = 0_u64;
            for byte in candidate.as_bytes().iter().copied() {
                let mask_offset = byte as usize * word_count;
                let mut carry = 1_u64;
                for word in 0..word_count {
                    let previous = state[word];
                    state[word] = ((previous << 1) | carry) & masks[mask_offset + word];
                    carry = previous >> (u64::BITS - 1);
                }
                matched |= state[target_word] & target_mask;
            }
            rejected |= u64::from(matched != 0);
        }

        if rejected != 0 {
            Err(MdsyncTransportError::capability_material_rejected())
        } else {
            Ok(())
        }
    }
}

impl fmt::Debug for SecretCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SecretCapability")
            .field("kind", &self.kind)
            .field("value", &"[REDACTED]")
            .finish()
    }
}

impl Drop for SecretCapability {
    fn drop(&mut self) {
        self.value.zeroize();
    }
}

struct NativeSession {
    project_key: String,
    workspace_id: String,
    web_origin: String,
    api_origin: String,
    actor: String,
    capability: SecretCapability,
}

impl fmt::Debug for NativeSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeSession")
            .field("project_key", &"[SANITIZED]")
            .field("workspace_id", &"[SANITIZED]")
            .field("web_origin", &"[SANITIZED]")
            .field("api_origin", &"[SANITIZED]")
            .field("actor", &"[SANITIZED]")
            .field("capability", &self.capability)
            .finish()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MdsyncSessionContext {
    pub(crate) workspace_id: String,
    pub(crate) actor: String,
    pub(crate) access: CollaborationAccess,
}

#[derive(Debug, Default)]
struct SessionState {
    active_project: Option<String>,
    generation: u64,
    active_mutations: usize,
    sessions: HashMap<String, Arc<NativeSession>>,
}

struct SessionSnapshot {
    generation: u64,
    session: Arc<NativeSession>,
}

struct MutationLease<'a> {
    store: &'a MdsyncSessionStore,
    snapshot: SessionSnapshot,
}

impl Drop for MutationLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut state) = self.store.state.lock() {
            state.active_mutations = state.active_mutations.saturating_sub(1);
        }
    }
}

#[derive(Debug)]
pub(crate) struct MdsyncSessionStore {
    client: Client,
    state: Mutex<SessionState>,
}

impl Default for MdsyncSessionStore {
    fn default() -> Self {
        Self::new().expect("the fixed native MDSync HTTP policy must be valid")
    }
}

impl MdsyncSessionStore {
    pub(crate) fn new() -> Result<Self, MdsyncTransportError> {
        let client = Client::builder()
            .redirect(Policy::none())
            .connect_timeout(REQUEST_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|_| MdsyncTransportError::transport())?;
        Ok(Self {
            client,
            state: Mutex::new(SessionState::default()),
        })
    }

    pub(crate) fn connect(
        &self,
        project_key: String,
        mut workspace_url: Zeroizing<String>,
        actor: String,
    ) -> Result<SanitizedSessionMetadata, MdsyncTransportError> {
        validate_project_key(&project_key)?;
        validate_actor(&actor)?;
        let parsed = parse_workspace_url(&workspace_url)?;
        workspace_url.zeroize();
        let generation = self.select_project(&project_key)?;
        let discovery_url = parsed
            .pasted_origin
            .join("/.well-known/mdsync.json")
            .map_err(|_| MdsyncTransportError::invalid_url("Invalid discovery URL"))?;
        let response = self
            .client
            .get(discovery_url)
            .header(ACCEPT, "application/json")
            .send()
            .map_err(map_request_error)?;
        ensure_no_redirect(&response)?;
        if response.status() != StatusCode::OK {
            return Err(if response.status() == StatusCode::SERVICE_UNAVAILABLE {
                MdsyncTransportError::discovery_unconfigured()
            } else {
                MdsyncTransportError::transport()
            });
        }
        let discovery: DiscoveryResponse = read_json(response, MAX_DISCOVERY_BYTES)?;
        discovery.validate(&parsed.pasted_origin)?;

        let session_id = format!("local-session-{}", Uuid::new_v4().simple());
        let handle = LocalSessionHandle::parse(session_id.clone())
            .map_err(|_| MdsyncTransportError::internal())?;
        let metadata = SanitizedSessionMetadata::new(
            handle,
            parsed.workspace_id.clone(),
            discovery.web_origin.to_string(),
            discovery.api_origin.to_string(),
            parsed.capability.access(),
            actor.clone(),
        )
        .map_err(|_| MdsyncTransportError::invalid_discovery())?;
        parsed.capability.reject_aliases(
            metadata
                .capability_alias_candidates()
                .into_iter()
                .chain([project_key.as_str()]),
        )?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| MdsyncTransportError::internal())?;
        if state.active_project.as_deref() != Some(project_key.as_str())
            || state.generation != generation
        {
            return Err(MdsyncTransportError::selection_changed());
        }
        state.sessions.insert(
            session_id,
            Arc::new(NativeSession {
                project_key,
                workspace_id: parsed.workspace_id,
                web_origin: discovery.web_origin.to_string(),
                api_origin: discovery.api_origin.to_string(),
                actor,
                capability: parsed.capability,
            }),
        );
        Ok(metadata)
    }

    pub(crate) fn activate_project(&self, project_key: &str) -> Result<(), MdsyncTransportError> {
        validate_project_key(project_key)?;
        self.select_project(project_key).map(|_| ())
    }

    fn select_project(&self, project_key: &str) -> Result<u64, MdsyncTransportError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| MdsyncTransportError::internal())?;
        if state.active_project.as_deref() != Some(project_key) {
            if state.active_mutations != 0 {
                return Err(MdsyncTransportError::project_busy());
            }
            state.sessions.clear();
            state.active_project = Some(project_key.to_owned());
            state.generation = state.generation.wrapping_add(1).max(1);
        }
        Ok(state.generation)
    }

    pub(crate) fn disconnect(
        &self,
        project_key: &str,
        session_id: &str,
    ) -> Result<(), MdsyncTransportError> {
        validate_session_id(session_id)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| MdsyncTransportError::internal())?;
        if state.active_mutations != 0 {
            return Err(MdsyncTransportError::project_busy());
        }
        let belongs_to_project = state
            .sessions
            .get(session_id)
            .map(|session| session.project_key == project_key)
            .unwrap_or(false);
        if !belongs_to_project {
            return Err(MdsyncTransportError::session_not_found());
        }
        state
            .sessions
            .get(session_id)
            .ok_or_else(MdsyncTransportError::session_not_found)?
            .reject_capability_aliases(session_id)?;
        state.sessions.remove(session_id);
        if state.sessions.is_empty() {
            state.active_project = None;
        }
        Ok(())
    }

    pub(crate) fn list_files(
        &self,
        project_key: &str,
        session_id: &str,
    ) -> Result<MdsyncFileListing, MdsyncTransportError> {
        let snapshot = self.session_snapshot(project_key, session_id)?;
        let url = snapshot.session.api_url("tree", None)?;
        let response = snapshot.session.send_read(&self.client, url)?;
        let listing: MdsyncFileListing = read_success_json(response)?;
        listing.validate(&snapshot.session.workspace_id)?;
        snapshot
            .session
            .capability
            .reject_aliases(listing.capability_alias_candidates())?;
        reject_capability_leak(&snapshot.session, &listing)?;
        self.ensure_current(&snapshot)?;
        Ok(listing)
    }

    pub(crate) fn session_context(
        &self,
        project_key: &str,
        session_id: &str,
    ) -> Result<MdsyncSessionContext, MdsyncTransportError> {
        let metadata = self.sanitized_session_metadata(project_key, session_id)?;
        Ok(MdsyncSessionContext {
            workspace_id: metadata.workspace_id,
            actor: metadata.actor,
            access: metadata.access,
        })
    }

    pub(crate) fn sanitized_session_metadata(
        &self,
        project_key: &str,
        session_id: &str,
    ) -> Result<SanitizedSessionMetadata, MdsyncTransportError> {
        let snapshot = self.session_snapshot(project_key, session_id)?;
        // Reconstructing the sanitized metadata here reuses the collaboration
        // boundary's origin and actor validation without exposing capability
        // material to envelope policy or IPC.
        let metadata = SanitizedSessionMetadata::new(
            LocalSessionHandle::parse(session_id.to_owned())
                .map_err(|_| MdsyncTransportError::session_not_found())?,
            snapshot.session.workspace_id.clone(),
            snapshot.session.web_origin.clone(),
            snapshot.session.api_origin.clone(),
            snapshot.session.capability.access(),
            snapshot.session.actor.clone(),
        )
        .map_err(|_| MdsyncTransportError::invalid_response())?;
        snapshot
            .session
            .capability
            .reject_aliases(metadata.capability_alias_candidates())?;
        self.ensure_current(&snapshot)?;
        Ok(metadata)
    }

    pub(crate) fn validate_completion_intent_for_persistence(
        &self,
        project_key: &str,
        session_id: &str,
        intent: &RemoteCompletionIntent,
    ) -> Result<(), MdsyncTransportError> {
        let snapshot = self.session_snapshot(project_key, session_id)?;
        snapshot
            .session
            .capability
            .reject_aliases(intent.capability_alias_candidates())?;
        self.ensure_current(&snapshot)
    }

    #[cfg(test)]
    pub(crate) fn insert_forged_session_for_test(
        &self,
        project_key: &str,
        session_id: &str,
        workspace_id: &str,
        actor: &str,
        capability: &str,
    ) -> Result<(), MdsyncTransportError> {
        validate_project_key(project_key)?;
        validate_session_id(session_id)?;
        validate_workspace_id(workspace_id)?;
        validate_actor(actor)?;
        let native_capability =
            SecretCapability::new(CapabilityKind::Edit, Zeroizing::new(capability.to_owned()))?;
        self.select_project(project_key)?;
        self.state
            .lock()
            .map_err(|_| MdsyncTransportError::internal())?
            .sessions
            .insert(
                session_id.to_owned(),
                Arc::new(NativeSession {
                    project_key: project_key.to_owned(),
                    workspace_id: workspace_id.to_owned(),
                    web_origin: "https://app.example.test".into(),
                    api_origin: "https://api.example.test".into(),
                    actor: actor.to_owned(),
                    capability: native_capability,
                }),
            );
        Ok(())
    }

    pub(crate) fn read_file(
        &self,
        project_key: &str,
        session_id: &str,
        path: &str,
    ) -> Result<MdsyncFile, MdsyncTransportError> {
        validate_file_path(path)?;
        let snapshot = self.session_snapshot(project_key, session_id)?;
        snapshot.session.capability.reject_aliases([path])?;
        let url = snapshot.session.api_url("files", Some(path))?;
        let response = snapshot.session.send_read(&self.client, url)?;
        let file: MdsyncFile = read_success_json(response)?;
        file.validate(&snapshot.session.workspace_id, Some(path))?;
        snapshot
            .session
            .capability
            .reject_aliases(file.capability_alias_candidates())?;
        reject_capability_leak(&snapshot.session, &file)?;
        self.ensure_current(&snapshot)?;
        Ok(file)
    }

    pub(crate) fn write_file(
        &self,
        project_key: &str,
        session_id: &str,
        input: MdsyncWriteInput,
    ) -> Result<MdsyncWriteResult, MdsyncTransportError> {
        input.validate()?;
        let lease = self.begin_mutation(project_key, session_id)?;
        let session = &lease.snapshot.session;
        session
            .capability
            .reject_aliases(input.capability_alias_candidates())?;
        reject_capability_leak(session, &input)?;
        let url = session.api_url("files", None)?;
        let body = WriteRequest {
            actor: &session.actor,
            base_version: input.base_version,
            content: &input.content,
            content_type: input
                .content_type
                .as_deref()
                .unwrap_or_else(|| content_type_for_path(&input.path)),
            path: &input.path,
        };
        let encoded = serde_json::to_vec(&body).map_err(|_| MdsyncTransportError::internal())?;
        if encoded.len() > MAX_REQUEST_BYTES {
            return Err(MdsyncTransportError::request_too_large());
        }
        let response = session.send_edit(&self.client, url, encoded)?;
        if response.status() == StatusCode::CONFLICT {
            return Err(parse_conflict(response, session, &input.path)?);
        }
        let result: MdsyncWriteResult = read_success_json(response)?;
        result.validate(&session.workspace_id, &input.path)?;
        session
            .capability
            .reject_aliases(result.capability_alias_candidates())?;
        reject_capability_leak(session, &result)?;
        self.ensure_current(&lease.snapshot)?;
        Ok(result)
    }

    pub(crate) fn write_file_with_readback(
        &self,
        project_key: &str,
        session_id: &str,
        input: MdsyncWriteInput,
        expected_post_version: u64,
    ) -> Result<MdsyncCommittedWrite, MdsyncTransportError> {
        if expected_post_version == 0 {
            return Err(MdsyncTransportError::invalid_input(
                "Expected post-write version must be positive",
            ));
        }
        let context = self.session_context(project_key, session_id)?;
        let expected_path = input.path.clone();
        let expected_content = input.content.clone();
        let expected_content_type = input
            .content_type
            .clone()
            .unwrap_or_else(|| content_type_for_path(&expected_path).into());
        match self.write_file(project_key, session_id, input) {
            Ok(result) => {
                if result.version != expected_post_version {
                    return Err(MdsyncTransportError::invalid_response());
                }
                Ok(MdsyncCommittedWrite {
                    path: result.path,
                    version: result.version,
                    recovered_from_readback: false,
                })
            }
            Err(write_failure) => {
                // A failed response does not prove the server rejected the
                // PUT. Perform exactly one bounded GET, never another PUT, and
                // accept only an exact actor-attributed committed file.
                let readback = self.read_file(project_key, session_id, &expected_path);
                if let Ok(file) = readback {
                    let committed = file.path == expected_path
                        && file.content == expected_content
                        && file.content_type == expected_content_type
                        && file.updated_by.as_deref() == Some(context.actor.as_str())
                        && file.version == expected_post_version
                        && file.workspace_id == context.workspace_id;
                    if committed {
                        return Ok(MdsyncCommittedWrite {
                            path: file.path,
                            version: file.version,
                            recovered_from_readback: true,
                        });
                    }
                }
                Err(write_failure)
            }
        }
    }

    fn session_snapshot(
        &self,
        project_key: &str,
        session_id: &str,
    ) -> Result<SessionSnapshot, MdsyncTransportError> {
        validate_session_id(session_id)?;
        let state = self
            .state
            .lock()
            .map_err(|_| MdsyncTransportError::internal())?;
        let session = state
            .sessions
            .get(session_id)
            .filter(|session| session.project_key == project_key)
            .ok_or_else(MdsyncTransportError::session_not_found)?;
        session.reject_capability_aliases(session_id)?;
        Ok(SessionSnapshot {
            generation: state.generation,
            session: Arc::clone(session),
        })
    }

    fn begin_mutation(
        &self,
        project_key: &str,
        session_id: &str,
    ) -> Result<MutationLease<'_>, MdsyncTransportError> {
        validate_session_id(session_id)?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| MdsyncTransportError::internal())?;
        let session = state
            .sessions
            .get(session_id)
            .filter(|session| session.project_key == project_key)
            .cloned()
            .ok_or_else(MdsyncTransportError::session_not_found)?;
        session.reject_capability_aliases(session_id)?;
        if !session.capability.is_edit() {
            return Err(MdsyncTransportError::access_denied());
        }
        let generation = state.generation;
        state.active_mutations += 1;
        Ok(MutationLease {
            store: self,
            snapshot: SessionSnapshot {
                generation,
                session,
            },
        })
    }

    fn ensure_current(&self, snapshot: &SessionSnapshot) -> Result<(), MdsyncTransportError> {
        let state = self
            .state
            .lock()
            .map_err(|_| MdsyncTransportError::internal())?;
        if state.generation != snapshot.generation
            || state.active_project.as_deref() != Some(snapshot.session.project_key.as_str())
        {
            Err(MdsyncTransportError::selection_changed())
        } else {
            Ok(())
        }
    }
}

impl Drop for MdsyncSessionStore {
    fn drop(&mut self) {
        if let Ok(state) = self.state.get_mut() {
            state.sessions.clear();
            state.active_project = None;
        }
    }
}

impl NativeSession {
    fn reject_capability_aliases(&self, session_id: &str) -> Result<(), MdsyncTransportError> {
        self.capability.reject_aliases([
            session_id,
            self.project_key.as_str(),
            self.workspace_id.as_str(),
            self.web_origin.as_str(),
            self.api_origin.as_str(),
            self.actor.as_str(),
        ])
    }

    fn api_url(&self, route: &str, path: Option<&str>) -> Result<Url, MdsyncTransportError> {
        let mut url =
            Url::parse(&self.api_origin).map_err(|_| MdsyncTransportError::invalid_discovery())?;
        url.path_segments_mut()
            .map_err(|_| MdsyncTransportError::invalid_discovery())?
            .clear()
            .push("api")
            .push("workspaces")
            .push(&self.workspace_id)
            .push(route);
        if let Some(path) = path {
            url.query_pairs_mut().append_pair("path", path);
        }
        if self.capability.kind == CapabilityKind::Read {
            url.query_pairs_mut()
                .append_pair("k", self.capability.value.as_str());
        }
        Ok(url)
    }

    fn send_read(&self, client: &Client, url: Url) -> Result<Response, MdsyncTransportError> {
        let mut request = client.get(url).header(ACCEPT, "application/json");
        if self.capability.kind == CapabilityKind::Edit {
            request = request.header(AUTHORIZATION, self.authorization_header()?);
        }
        request.send().map_err(map_request_error)
    }

    fn send_edit(
        &self,
        client: &Client,
        url: Url,
        body: Vec<u8>,
    ) -> Result<Response, MdsyncTransportError> {
        client
            .put(url)
            .header(ACCEPT, "application/json")
            .header(CONTENT_TYPE, "application/json")
            .header(AUTHORIZATION, self.authorization_header()?)
            .body(body)
            .send()
            .map_err(map_request_error)
    }

    fn authorization_header(&self) -> Result<HeaderValue, MdsyncTransportError> {
        let bearer = Zeroizing::new(format!("Bearer {}", self.capability.value.as_str()));
        let mut header = HeaderValue::from_str(&bearer).map_err(|_| {
            MdsyncTransportError::invalid_input(
                "Capability cannot be represented as an authorization header",
            )
        })?;
        header.set_sensitive(true);
        Ok(header)
    }
}

#[derive(Debug)]
struct ParsedWorkspaceUrl {
    pasted_origin: Url,
    workspace_id: String,
    capability: SecretCapability,
}

struct SensitiveUrl(Option<Url>);

impl SensitiveUrl {
    fn parse(input: &str) -> Result<Self, MdsyncTransportError> {
        Url::parse(input).map(|url| Self(Some(url))).map_err(|_| {
            MdsyncTransportError::invalid_url("Expected an absolute MDSync workspace URL")
        })
    }
}

impl Deref for SensitiveUrl {
    type Target = Url;

    fn deref(&self) -> &Self::Target {
        self.0
            .as_ref()
            .expect("sensitive URL remains present until drop")
    }
}

impl fmt::Debug for SensitiveUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SensitiveUrl([REDACTED])")
    }
}

impl Drop for SensitiveUrl {
    fn drop(&mut self) {
        if let Some(url) = self.0.take() {
            // url::Url owns one backing string. Taking and zeroizing that
            // string clears the bounded capability-bearing parse copy.
            let mut serialized = String::from(url);
            serialized.zeroize();
        }
    }
}

fn parse_workspace_url(input: &str) -> Result<ParsedWorkspaceUrl, MdsyncTransportError> {
    if input.len() > MAX_WORKSPACE_URL_BYTES || input.trim() != input || input.is_empty() {
        return Err(MdsyncTransportError::invalid_url(
            "Expected an absolute MDSync workspace URL",
        ));
    }
    let url = SensitiveUrl::parse(input)?;
    if !url.username().is_empty() || url.password().is_some() || url.fragment().is_some() {
        return Err(MdsyncTransportError::invalid_url(
            "Workspace URLs cannot contain credentials or fragments",
        ));
    }
    validate_allowed_origin(&url)?;
    let segments = url
        .path_segments()
        .ok_or_else(|| MdsyncTransportError::invalid_url("Expected a workspace route"))?
        .collect::<Vec<_>>();
    if segments.len() < 2 || segments[0] != "w" || segments[1].is_empty() {
        return Err(MdsyncTransportError::invalid_url(
            "Expected a supported MDSync workspace route",
        ));
    }
    validate_route(&segments[2..])?;
    let workspace_id = decode_component(segments[1], "workspace id")?;
    validate_workspace_id(&workspace_id)?;

    let mut edit: Option<Zeroizing<String>> = None;
    let mut read: Option<Zeroizing<String>> = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "edit" => {
                if edit.is_some() {
                    return Err(MdsyncTransportError::invalid_url(
                        "Workspace URL capabilities are ambiguous",
                    ));
                }
                if value.is_empty() || value.len() > MAX_CAPABILITY_BYTES {
                    return Err(MdsyncTransportError::invalid_url(
                        "Workspace capability is empty or oversized",
                    ));
                }
                edit = Some(Zeroizing::new(value.into_owned()));
            }
            "k" => {
                if read.is_some() {
                    return Err(MdsyncTransportError::invalid_url(
                        "Workspace URL capabilities are ambiguous",
                    ));
                }
                if value.is_empty() || value.len() > MAX_CAPABILITY_BYTES {
                    return Err(MdsyncTransportError::invalid_url(
                        "Workspace capability is empty or oversized",
                    ));
                }
                read = Some(Zeroizing::new(value.into_owned()));
            }
            _ => {
                return Err(MdsyncTransportError::invalid_url(
                    "Workspace URL contains an unsupported query parameter",
                ))
            }
        }
    }
    if edit.is_some() && read.is_some() {
        return Err(MdsyncTransportError::invalid_url(
            "Workspace URL capabilities are ambiguous",
        ));
    }
    let capability = if let Some(value) = edit {
        SecretCapability::new(CapabilityKind::Edit, value)?
    } else if let Some(value) = read {
        SecretCapability::new(CapabilityKind::Read, value)?
    } else {
        SecretCapability::public()
    };
    let pasted_origin = origin_url(&url)?;
    Ok(ParsedWorkspaceUrl {
        pasted_origin,
        workspace_id,
        capability,
    })
}

fn validate_route(segments: &[&str]) -> Result<(), MdsyncTransportError> {
    match segments {
        [] | [""] | ["work"] | ["activity"] | ["settings"] | ["files"] | ["raw"] => Ok(()),
        [kind, rest @ ..] if (*kind == "files" || *kind == "raw") && !rest.is_empty() => {
            for segment in rest {
                let decoded = decode_component(segment, "file path")?;
                if decoded.is_empty() || decoded.chars().any(char::is_control) {
                    return Err(MdsyncTransportError::invalid_url(
                        "Workspace file path is invalid",
                    ));
                }
            }
            Ok(())
        }
        _ => Err(MdsyncTransportError::invalid_url(
            "Unsupported MDSync workspace route",
        )),
    }
}

fn decode_component(value: &str, field: &str) -> Result<String, MdsyncTransportError> {
    percent_decode_str(value)
        .decode_utf8()
        .map(String::from)
        .map_err(|_| MdsyncTransportError::invalid_url(format!("Invalid {field} encoding")))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DiscoveryWire {
    api_origin: String,
    discovery_version: u8,
    product: String,
    web_origin: String,
}

struct DiscoveryResponse {
    api_origin: Url,
    web_origin: Url,
}

impl<'de> Deserialize<'de> for DiscoveryResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = DiscoveryWire::deserialize(deserializer)?;
        if wire.discovery_version != 1 || wire.product != "mdsync" {
            return Err(serde::de::Error::custom("unsupported discovery contract"));
        }
        let api_origin = Url::parse(&wire.api_origin)
            .map_err(|_| serde::de::Error::custom("invalid API origin"))?;
        let web_origin = Url::parse(&wire.web_origin)
            .map_err(|_| serde::de::Error::custom("invalid Web origin"))?;
        Ok(Self {
            api_origin,
            web_origin,
        })
    }
}

impl DiscoveryResponse {
    fn validate(&self, pasted_origin: &Url) -> Result<(), MdsyncTransportError> {
        validate_origin_only(&self.api_origin)?;
        validate_origin_only(&self.web_origin)?;
        validate_allowed_origin(&self.api_origin)?;
        validate_allowed_origin(&self.web_origin)?;
        let production_web = Url::parse("https://sync.ha2ha.md").expect("fixed production origin");
        let production_api =
            Url::parse("https://sync-api.ha2ha.md").expect("fixed production origin");
        let is_production_paste =
            pasted_origin == &production_web || pasted_origin == &production_api;
        if is_production_paste {
            if self.web_origin != production_web || self.api_origin != production_api {
                return Err(MdsyncTransportError::origin_mismatch());
            }
            return Ok(());
        }
        if is_loopback_origin(pasted_origin) {
            if !is_loopback_origin(&self.web_origin) || !is_loopback_origin(&self.api_origin) {
                return Err(MdsyncTransportError::origin_mismatch());
            }
            return Ok(());
        }
        // Unknown/custom deployments may advertise a separate browser origin,
        // but bearer material is forwarded only when the API is the exact
        // origin from which the capability-bearing URL was pasted.
        if pasted_origin != &self.api_origin {
            return Err(MdsyncTransportError::origin_mismatch());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MdsyncFileEntry {
    content_type: String,
    path: String,
    updated_at: String,
    updated_by: Option<String>,
    version: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MdsyncFileListing {
    files: Vec<MdsyncFileEntry>,
    workspace_id: String,
}

impl MdsyncFileListing {
    fn validate(&self, workspace_id: &str) -> Result<(), MdsyncTransportError> {
        if self.workspace_id != workspace_id {
            return Err(MdsyncTransportError::invalid_response());
        }
        for file in &self.files {
            validate_file_path(&file.path)?;
            validate_nonempty("content type", &file.content_type, 256)?;
            validate_nonempty("updated timestamp", &file.updated_at, 128)?;
            if file.version == 0 {
                return Err(MdsyncTransportError::invalid_response());
            }
            if let Some(actor) = file.updated_by.as_deref() {
                validate_actor(actor)?;
            }
        }
        Ok(())
    }

    fn capability_alias_candidates(&self) -> Vec<&str> {
        let mut candidates = vec![self.workspace_id.as_str()];
        for file in &self.files {
            candidates.extend([
                file.content_type.as_str(),
                file.path.as_str(),
                file.updated_at.as_str(),
            ]);
            if let Some(actor) = file.updated_by.as_deref() {
                candidates.push(actor);
            }
        }
        candidates
    }

    pub(crate) fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.iter().map(|file| file.path.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MdsyncFile {
    content: String,
    content_type: String,
    path: String,
    updated_at: String,
    updated_by: Option<String>,
    version: u64,
    workspace_id: String,
}

impl MdsyncFile {
    fn validate(
        &self,
        workspace_id: &str,
        expected_path: Option<&str>,
    ) -> Result<(), MdsyncTransportError> {
        if self.workspace_id != workspace_id
            || expected_path.is_some_and(|path| path != self.path)
            || self.version == 0
        {
            return Err(MdsyncTransportError::invalid_response());
        }
        validate_file_path(&self.path)?;
        validate_nonempty("content type", &self.content_type, 256)?;
        validate_nonempty("updated timestamp", &self.updated_at, 128)?;
        if let Some(actor) = self.updated_by.as_deref() {
            validate_actor(actor)?;
        }
        Ok(())
    }

    fn capability_alias_candidates(&self) -> Vec<&str> {
        let mut candidates = vec![
            self.content.as_str(),
            self.content_type.as_str(),
            self.path.as_str(),
            self.updated_at.as_str(),
            self.workspace_id.as_str(),
        ];
        if let Some(actor) = self.updated_by.as_deref() {
            candidates.push(actor);
        }
        candidates
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn content(&self) -> &str {
        &self.content
    }

    pub(crate) fn version(&self) -> u64 {
        self.version
    }

    pub(crate) fn matches_committed_write(
        &self,
        workspace_id: &str,
        path: &str,
        content: &str,
        content_type: &str,
        actor: &str,
        version: u64,
    ) -> bool {
        self.workspace_id == workspace_id
            && self.path == path
            && self.content == content
            && self.content_type == content_type
            && self.updated_by.as_deref() == Some(actor)
            && self.version == version
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MdsyncWriteInput {
    pub(crate) path: String,
    pub(crate) content: String,
    pub(crate) content_type: Option<String>,
    pub(crate) base_version: Option<u64>,
}

impl MdsyncWriteInput {
    fn validate(&self) -> Result<(), MdsyncTransportError> {
        validate_file_path(&self.path)?;
        if self.base_version == Some(0) {
            return Err(MdsyncTransportError::invalid_input(
                "baseVersion must be positive when provided",
            ));
        }
        if let Some(content_type) = self.content_type.as_deref() {
            validate_nonempty("content type", content_type, 256)?;
        }
        if self.content.len() > MAX_REQUEST_BYTES {
            return Err(MdsyncTransportError::request_too_large());
        }
        Ok(())
    }

    fn capability_alias_candidates(&self) -> Vec<&str> {
        let mut candidates = vec![self.path.as_str(), self.content.as_str()];
        if let Some(content_type) = self.content_type.as_deref() {
            candidates.push(content_type);
        }
        candidates
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WriteRequest<'a> {
    actor: &'a str,
    base_version: Option<u64>,
    content: &'a str,
    content_type: &'a str,
    path: &'a str,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MdsyncWriteResult {
    path: String,
    updated_at: Option<String>,
    updated_by: Option<String>,
    version: u64,
    workspace_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MdsyncCommittedWrite {
    path: String,
    version: u64,
    recovered_from_readback: bool,
}

impl MdsyncCommittedWrite {
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn version(&self) -> u64 {
        self.version
    }

    pub(crate) fn recovered_from_readback(&self) -> bool {
        self.recovered_from_readback
    }
}

impl MdsyncWriteResult {
    fn validate(&self, workspace_id: &str, path: &str) -> Result<(), MdsyncTransportError> {
        if self.workspace_id != workspace_id || self.path != path || self.version == 0 {
            return Err(MdsyncTransportError::invalid_response());
        }
        validate_file_path(&self.path)?;
        if let Some(updated_at) = self.updated_at.as_deref() {
            validate_nonempty("updated timestamp", updated_at, 128)?;
        }
        if let Some(actor) = self.updated_by.as_deref() {
            validate_actor(actor)?;
        }
        Ok(())
    }

    fn capability_alias_candidates(&self) -> Vec<&str> {
        let mut candidates = vec![self.path.as_str(), self.workspace_id.as_str()];
        if let Some(updated_at) = self.updated_at.as_deref() {
            candidates.push(updated_at);
        }
        if let Some(actor) = self.updated_by.as_deref() {
            candidates.push(actor);
        }
        candidates
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    pub(crate) fn version(&self) -> u64 {
        self.version
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConflictWire {
    error: String,
    latest: Option<MdsyncFile>,
    message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MdsyncConflictDetails {
    latest: Option<MdsyncLatestCoordinate>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MdsyncLatestCoordinate {
    path: String,
    version: u64,
    workspace_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum MdsyncTransportErrorClass {
    InvalidInput,
    CapabilityMaterial,
    AccessDenied,
    SessionNotFound,
    Discovery,
    OriginMismatch,
    Transport,
    Timeout,
    ResponseTooLarge,
    Protocol,
    VersionConflict,
    ProjectBusy,
    SelectionChanged,
    Internal,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MdsyncTransportError {
    class: MdsyncTransportErrorClass,
    code: &'static str,
    message: &'static str,
    conflict: Option<MdsyncConflictDetails>,
}

impl MdsyncTransportError {
    fn new(class: MdsyncTransportErrorClass, code: &'static str, message: &'static str) -> Self {
        Self {
            class,
            code,
            message,
            conflict: None,
        }
    }

    fn invalid_url(_detail: impl AsRef<str>) -> Self {
        Self::new(
            MdsyncTransportErrorClass::InvalidInput,
            "invalid_workspace_url",
            "The MDSync workspace URL is invalid",
        )
    }

    pub(crate) fn workspace_url_too_large() -> Self {
        Self::new(
            MdsyncTransportErrorClass::InvalidInput,
            "workspace_url_too_large",
            "The MDSync workspace URL exceeded the allowed size",
        )
    }

    pub(crate) fn invalid_project() -> Self {
        Self::new(
            MdsyncTransportErrorClass::InvalidInput,
            "invalid_project",
            "The MDSync project root is invalid",
        )
    }

    fn invalid_input(_detail: impl AsRef<str>) -> Self {
        Self::new(
            MdsyncTransportErrorClass::InvalidInput,
            "invalid_input",
            "The MDSync operation input is invalid",
        )
    }

    fn capability_material_rejected() -> Self {
        Self::new(
            MdsyncTransportErrorClass::CapabilityMaterial,
            "capability_material_rejected",
            "Capability material cannot cross the native session boundary",
        )
    }

    fn access_denied() -> Self {
        Self::new(
            MdsyncTransportErrorClass::AccessDenied,
            "access_denied",
            "Collaborator access is required for this operation",
        )
    }

    fn session_not_found() -> Self {
        Self::new(
            MdsyncTransportErrorClass::SessionNotFound,
            "session_not_found",
            "The native MDSync session is unavailable",
        )
    }

    fn project_busy() -> Self {
        Self::new(
            MdsyncTransportErrorClass::ProjectBusy,
            "project_busy",
            "Project selection cannot change while a remote mutation is in flight",
        )
    }

    fn selection_changed() -> Self {
        Self::new(
            MdsyncTransportErrorClass::SelectionChanged,
            "selection_changed",
            "Project selection changed before the MDSync operation completed",
        )
    }

    fn discovery_unconfigured() -> Self {
        Self::new(
            MdsyncTransportErrorClass::Discovery,
            "discovery_unconfigured",
            "MDSync discovery is unavailable",
        )
    }

    fn invalid_discovery() -> Self {
        Self::new(
            MdsyncTransportErrorClass::Discovery,
            "invalid_discovery",
            "MDSync discovery returned an unsupported contract",
        )
    }

    fn origin_mismatch() -> Self {
        Self::new(
            MdsyncTransportErrorClass::OriginMismatch,
            "origin_mismatch",
            "MDSync discovery does not match the pasted origin",
        )
    }

    fn transport() -> Self {
        Self::new(
            MdsyncTransportErrorClass::Transport,
            "transport_unavailable",
            "The MDSync transport is unavailable",
        )
    }

    fn timeout() -> Self {
        Self::new(
            MdsyncTransportErrorClass::Timeout,
            "timeout",
            "The MDSync operation exceeded its bounded timeout",
        )
    }

    fn response_too_large() -> Self {
        Self::new(
            MdsyncTransportErrorClass::ResponseTooLarge,
            "response_too_large",
            "The MDSync response exceeded the allowed size",
        )
    }

    fn request_too_large() -> Self {
        Self::new(
            MdsyncTransportErrorClass::InvalidInput,
            "request_too_large",
            "The MDSync request exceeded the allowed size",
        )
    }

    fn invalid_response() -> Self {
        Self::new(
            MdsyncTransportErrorClass::Protocol,
            "invalid_response",
            "MDSync returned an invalid response",
        )
    }

    fn internal() -> Self {
        Self::new(
            MdsyncTransportErrorClass::Internal,
            "internal_error",
            "The native MDSync transport failed safely",
        )
    }

    fn version_conflict(latest: Option<MdsyncLatestCoordinate>) -> Self {
        Self {
            class: MdsyncTransportErrorClass::VersionConflict,
            code: "version_conflict",
            message: "The remote file changed after the supplied base version",
            conflict: Some(MdsyncConflictDetails { latest }),
        }
    }

    pub(crate) fn class(&self) -> MdsyncTransportErrorClass {
        self.class
    }

    pub(crate) fn latest_version(&self) -> Option<u64> {
        self.conflict
            .as_ref()
            .and_then(|conflict| conflict.latest.as_ref())
            .map(|latest| latest.version)
    }
}

fn parse_conflict(
    response: Response,
    session: &NativeSession,
    expected_path: &str,
) -> Result<MdsyncTransportError, MdsyncTransportError> {
    let conflict: ConflictWire = read_json(response, MAX_RESPONSE_BYTES)?;
    if conflict.error != "version_conflict" || conflict.message.trim().is_empty() {
        return Err(MdsyncTransportError::invalid_response());
    }
    let latest = if let Some(file) = conflict.latest {
        file.validate(&session.workspace_id, Some(expected_path))?;
        session
            .capability
            .reject_aliases(file.capability_alias_candidates())?;
        reject_capability_leak(session, &file)?;
        Some(MdsyncLatestCoordinate {
            path: file.path,
            version: file.version,
            workspace_id: file.workspace_id,
        })
    } else {
        None
    };
    Ok(MdsyncTransportError::version_conflict(latest))
}

fn read_success_json<T: DeserializeOwned>(response: Response) -> Result<T, MdsyncTransportError> {
    ensure_no_redirect(&response)?;
    if !response.status().is_success() {
        return Err(MdsyncTransportError::transport());
    }
    read_json(response, MAX_RESPONSE_BYTES)
}

fn read_json<T: DeserializeOwned>(
    response: Response,
    limit: usize,
) -> Result<T, MdsyncTransportError> {
    if response
        .headers()
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok())
        .is_some_and(|length| length > limit)
    {
        return Err(MdsyncTransportError::response_too_large());
    }
    let mut bytes = Vec::new();
    response
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| MdsyncTransportError::transport())?;
    if bytes.len() > limit {
        return Err(MdsyncTransportError::response_too_large());
    }
    serde_json::from_slice(&bytes).map_err(|_| MdsyncTransportError::invalid_response())
}

fn ensure_no_redirect(response: &Response) -> Result<(), MdsyncTransportError> {
    if response.status().is_redirection() {
        Err(MdsyncTransportError::transport())
    } else {
        Ok(())
    }
}

fn map_request_error(error: reqwest::Error) -> MdsyncTransportError {
    if error.is_timeout() {
        MdsyncTransportError::timeout()
    } else {
        MdsyncTransportError::transport()
    }
}

fn reject_capability_leak(
    session: &NativeSession,
    value: &impl Serialize,
) -> Result<(), MdsyncTransportError> {
    let serialized =
        Zeroizing::new(serde_json::to_string(value).map_err(|_| MdsyncTransportError::internal())?);
    session.capability.reject_aliases([serialized.as_str()])
}

fn validate_allowed_origin(url: &Url) -> Result<(), MdsyncTransportError> {
    match url.scheme() {
        "https" => Ok(()),
        "http"
            if matches!(
                url.host_str(),
                Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
            ) =>
        {
            Ok(())
        }
        _ => Err(MdsyncTransportError::invalid_url(
            "HTTPS is required except for explicit localhost development",
        )),
    }
}

fn is_loopback_origin(url: &Url) -> bool {
    matches!(
        url.host_str(),
        Some("localhost" | "127.0.0.1" | "::1" | "[::1]")
    )
}

fn validate_origin_only(url: &Url) -> Result<(), MdsyncTransportError> {
    if url.username().is_empty()
        && url.password().is_none()
        && url.path() == "/"
        && url.query().is_none()
        && url.fragment().is_none()
        && url.host_str().is_some()
    {
        Ok(())
    } else {
        Err(MdsyncTransportError::invalid_discovery())
    }
}

fn origin_url(url: &SensitiveUrl) -> Result<Url, MdsyncTransportError> {
    // Origin serialization excludes username/password, path, query, and
    // fragment, so no capability-bearing Url clone leaves the parser.
    let origin = Url::parse(&url.origin().ascii_serialization())
        .map_err(|_| MdsyncTransportError::invalid_url("Invalid workspace origin"))?;
    validate_origin_only(&origin)?;
    Ok(origin)
}

fn validate_workspace_id(value: &str) -> Result<(), MdsyncTransportError> {
    validate_nonempty("workspace id", value, MAX_WORKSPACE_ID_BYTES)?;
    if value.contains(['/', '\\', '?', '#'])
        || value.chars().any(char::is_control)
        || value == "."
        || value == ".."
    {
        return Err(MdsyncTransportError::invalid_url("Invalid workspace id"));
    }
    Ok(())
}

fn validate_project_key(value: &str) -> Result<(), MdsyncTransportError> {
    validate_nonempty("project key", value, 4096)
}

fn validate_actor(value: &str) -> Result<(), MdsyncTransportError> {
    validate_nonempty("actor", value, MAX_ACTOR_BYTES)?;
    if value.trim() != value || value.chars().any(char::is_control) {
        return Err(MdsyncTransportError::invalid_input("Invalid actor"));
    }
    Ok(())
}

fn validate_file_path(value: &str) -> Result<(), MdsyncTransportError> {
    validate_nonempty("file path", value, MAX_PATH_BYTES)?;
    if value.starts_with('/')
        || value.starts_with('\\')
        || value
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        || value.chars().any(char::is_control)
    {
        return Err(MdsyncTransportError::invalid_input("Invalid file path"));
    }
    Ok(())
}

fn validate_session_id(value: &str) -> Result<(), MdsyncTransportError> {
    LocalSessionHandle::parse(value.to_owned())
        .map(|_| ())
        .map_err(|_| MdsyncTransportError::session_not_found())
}

fn validate_nonempty(
    _field: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), MdsyncTransportError> {
    if value.is_empty() || value.len() > max_bytes || value.chars().any(char::is_control) {
        Err(MdsyncTransportError::invalid_input("Invalid bounded value"))
    } else {
        Ok(())
    }
}

fn content_type_for_path(path: &str) -> &'static str {
    if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".md") || path.ends_with(".mdx") {
        "text/markdown; charset=utf-8"
    } else {
        "text/plain; charset=utf-8"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        collaboration::{
            run_after_local_commit, ClaimResult, CollaborationMode, CollaborationPort,
            CompletionArtifact, EvidenceHandoffResult, PostLocalCommitCollaborationContext,
            PreRunCollaborationContext, ReconciliationState, RemoteCompletionIntent,
            RemoteTaskBinding, SharedExecutionBinding,
        },
        ha2ha_envelope::{
            project_post_run_reconciliation, project_task_claim, project_workspace,
            ProjectionInput, RemoteWorkspaceFile as EnvelopeRemoteWorkspaceFile,
            ResolverTaskStatus,
        },
        local_collaboration_binding, MdsyncClaimPort, OperationRegistry, SharedClaimState,
    };
    use std::{
        fs,
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        process::Command,
        sync::{atomic::AtomicBool, Arc, Mutex},
        thread,
        time::Instant,
    };

    const READ_SECRET: &str = "read-capability-DO-NOT-LEAK";
    const EDIT_SECRET: &str = "edit-capability-DO-NOT-LEAK";

    struct FixtureResponse {
        status: u16,
        headers: Vec<(&'static str, String)>,
        body: Vec<u8>,
        delay: Duration,
    }

    impl FixtureResponse {
        fn json(status: u16, body: impl Into<Vec<u8>>) -> Self {
            Self {
                status,
                headers: vec![("Content-Type", "application/json".into())],
                body: body.into(),
                delay: Duration::ZERO,
            }
        }

        fn redirect(location: String) -> Self {
            Self {
                status: 302,
                headers: vec![("Location", location)],
                body: Vec::new(),
                delay: Duration::ZERO,
            }
        }
    }

    struct FixtureServer {
        base_url: String,
        requests: Arc<Mutex<Vec<String>>>,
        worker: Option<thread::JoinHandle<()>>,
    }

    impl FixtureServer {
        fn start(mut responses: Vec<FixtureResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let base_url = format!("http://{}", listener.local_addr().unwrap());
            for response in &mut responses {
                if let Ok(body) = std::str::from_utf8(&response.body) {
                    response.body = body.replace("{BASE_URL}", &base_url).into_bytes();
                }
                for (_, value) in &mut response.headers {
                    *value = value.replace("{BASE_URL}", &base_url);
                }
            }
            let requests = Arc::new(Mutex::new(Vec::new()));
            let captured = Arc::clone(&requests);
            let worker = thread::spawn(move || {
                for response in responses.drain(..) {
                    let (mut stream, _) = listener.accept().unwrap();
                    let request = read_request(&mut stream);
                    captured.lock().unwrap().push(request);
                    if !response.delay.is_zero() {
                        thread::sleep(response.delay);
                    }
                    write_response(&mut stream, response);
                }
            });
            Self {
                base_url,
                requests,
                worker: Some(worker),
            }
        }

        fn workspace_url(&self, query: &str) -> String {
            format!("{}/w/workspace-1{query}", self.base_url)
        }

        fn requests(&self) -> Vec<String> {
            self.requests.lock().unwrap().clone()
        }

        fn wait_for_requests(&self, count: usize) {
            let deadline = Instant::now() + Duration::from_secs(2);
            while self.requests.lock().unwrap().len() < count {
                assert!(Instant::now() < deadline, "fixture request did not arrive");
                thread::sleep(Duration::from_millis(5));
            }
        }
    }

    impl Drop for FixtureServer {
        fn drop(&mut self) {
            if let Some(worker) = self.worker.take() {
                worker.join().unwrap();
            }
        }
    }

    fn read_request(stream: &mut TcpStream) -> String {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = stream.read(&mut buffer).unwrap_or(0);
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(header_end) = find_header_end(&bytes) {
                let headers = String::from_utf8_lossy(&bytes[..header_end]);
                let content_length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(str::trim)
                            .and_then(|value| value.parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if bytes.len() >= header_end + 4 + content_length {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn find_header_end(bytes: &[u8]) -> Option<usize> {
        bytes.windows(4).position(|window| window == b"\r\n\r\n")
    }

    fn write_response(stream: &mut TcpStream, response: FixtureResponse) {
        let reason = match response.status {
            200 => "OK",
            302 => "Found",
            409 => "Conflict",
            503 => "Service Unavailable",
            _ => "Error",
        };
        let mut head = format!("HTTP/1.1 {} {}\r\n", response.status, reason);
        for (name, value) in response.headers {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
        head.push_str(&format!(
            "Content-Length: {}\r\nConnection: close\r\n\r\n",
            response.body.len()
        ));
        let _ = stream.write_all(head.as_bytes());
        let _ = stream.write_all(&response.body);
    }

    fn discovery_json(api_origin: &str, web_origin: &str) -> String {
        serde_json::json!({
            "apiOrigin": api_origin,
            "discoveryVersion": 1,
            "product": "mdsync",
            "webOrigin": web_origin,
        })
        .to_string()
    }

    fn discovery_response() -> FixtureResponse {
        FixtureResponse::json(200, discovery_json("{BASE_URL}", "{BASE_URL}"))
    }

    fn file_json(content: &str, version: u64) -> String {
        serde_json::json!({
            "content": content,
            "contentType": "text/markdown; charset=utf-8",
            "path": "STATUS.md",
            "updatedAt": "2026-07-23T10:00:00.000Z",
            "updatedBy": "agent-a",
            "version": version,
            "workspaceId": "workspace-1",
        })
        .to_string()
    }

    fn task_file_json(content: &str, version: u64, actor: &str) -> String {
        serde_json::json!({
            "content": content,
            "contentType": "text/markdown; charset=utf-8",
            "path": "tasks/BR-015.md",
            "updatedAt": "2026-07-23T10:00:00.000Z",
            "updatedBy": actor,
            "version": version,
            "workspaceId": "workspace-1",
        })
        .to_string()
    }

    fn workspace_file_json(
        path: &str,
        content: &str,
        content_type: &str,
        version: u64,
        actor: &str,
    ) -> String {
        serde_json::json!({
            "content": content,
            "contentType": content_type,
            "path": path,
            "updatedAt": "2026-07-23T10:00:00.000Z",
            "updatedBy": actor,
            "version": version,
            "workspaceId": "workspace-1",
        })
        .to_string()
    }

    fn workspace_listing_json(files: &[EnvelopeRemoteWorkspaceFile], actor: &str) -> String {
        serde_json::json!({
            "files": files
                .iter()
                .map(|file| serde_json::json!({
                    "contentType": if file.path.ends_with(".json") {
                        "application/json"
                    } else {
                        "text/markdown; charset=utf-8"
                    },
                    "path": file.path,
                    "updatedAt": "2026-07-23T10:00:00.000Z",
                    "updatedBy": actor,
                    "version": file.version,
                }))
                .collect::<Vec<_>>(),
            "workspaceId": "workspace-1",
        })
        .to_string()
    }

    fn connect(
        store: &MdsyncSessionStore,
        server: &FixtureServer,
        project: &str,
        query: &str,
    ) -> SanitizedSessionMetadata {
        store
            .connect(
                project.into(),
                Zeroizing::new(server.workspace_url(query)),
                "agent-a".into(),
            )
            .unwrap()
    }

    fn completion_intent_fixture() -> RemoteCompletionIntent {
        RemoteCompletionIntent::new(
            "workspace-1".into(),
            "agent-a".into(),
            "BR-017".into(),
            "tasks/BR-017.md".into(),
            2,
            format!("sha256:{}", "a".repeat(64)),
            "tasks/issues/017.md".into(),
            format!("sha256:{}", "b".repeat(64)),
            format!("sha256:{}", "c".repeat(64)),
            "0123456789abcdef0123456789abcdef".into(),
            1,
            format!("evidence-{}", "1".repeat(32)),
            format!("evidence/BR-017/completion-{}.md", "1".repeat(32)),
            format!("handoff-{}", "2".repeat(32)),
            format!("logs/BR-017-handoff-{}.md", "2".repeat(32)),
            vec![CompletionArtifact {
                path: "tasks/issues/017.md".into(),
                sha256: format!("sha256:{}", "b".repeat(64)),
            }],
        )
        .unwrap()
    }

    #[test]
    fn parses_only_supported_secure_workspace_urls() {
        for url in [
            "https://app.example.com/w/workspace-1",
            "https://app.example.com/w/workspace-1/work?edit=write",
            "https://app.example.com/w/workspace-1/files/tasks%2FTASK-001.md?k=read",
            "https://api.example.com/w/workspace-1/raw/STATUS.md",
            "http://localhost:3200/w/workspace-1/activity",
            "http://127.0.0.1:3200/w/workspace-1/settings",
            "http://[::1]:3200/w/workspace-1",
        ] {
            assert!(parse_workspace_url(url).is_ok(), "{url}");
        }
        assert_eq!(
            parse_workspace_url("https://app.example.com/w/workspace-1")
                .unwrap()
                .capability
                .access(),
            CollaborationAccess::Public
        );
        assert_eq!(
            parse_workspace_url("https://app.example.com/w/workspace-1?k=read")
                .unwrap()
                .capability
                .access(),
            CollaborationAccess::Viewer
        );
        assert_eq!(
            parse_workspace_url("https://app.example.com/w/workspace-1?edit=write")
                .unwrap()
                .capability
                .access(),
            CollaborationAccess::Collaborator
        );
        for url in [
            "",
            " https://app.example.com/w/workspace-1",
            "http://app.example.com/w/workspace-1?edit=secret",
            "https://user:password@app.example.com/w/workspace-1",
            "https://app.example.com/w/workspace-1#fragment",
            "https://app.example.com/w/workspace-1?edit=",
            "https://app.example.com/w/workspace-1?k=",
            "https://app.example.com/w/workspace-1?edit=one&edit=two",
            "https://app.example.com/w/workspace-1?edit=one&k=two",
            "https://app.example.com/w/workspace-1?token=secret",
            "https://app.example.com/w/workspace%2Fescape",
            "https://app.example.com/w/workspace-1/unsupported",
        ] {
            let error = parse_workspace_url(url).unwrap_err();
            let serialized = serde_json::to_string(&error).unwrap();
            assert!(!serialized.contains("secret"), "{url}");
            assert!(!serialized.contains("password"), "{url}");
        }
    }

    #[test]
    fn discovery_is_strict_origin_bound_redirect_free_and_bounded() {
        let mismatched = FixtureServer::start(vec![FixtureResponse::json(
            200,
            discovery_json("https://api.example.com", "https://web.example.com"),
        )]);
        assert_eq!(
            MdsyncSessionStore::new()
                .unwrap()
                .connect(
                    "project-a".into(),
                    Zeroizing::new(mismatched.workspace_url("")),
                    "agent-a".into()
                )
                .unwrap_err()
                .code,
            "origin_mismatch"
        );

        let malformed = FixtureServer::start(vec![FixtureResponse::json(
            200,
            r#"{"apiOrigin":"http://localhost:1","discoveryVersion":2,"product":"mdsync","webOrigin":"http://localhost:1"}"#,
        )]);
        assert_eq!(
            MdsyncSessionStore::new()
                .unwrap()
                .connect(
                    "project-a".into(),
                    Zeroizing::new(malformed.workspace_url("")),
                    "agent-a".into()
                )
                .unwrap_err()
                .code,
            "invalid_response"
        );

        let redirect_target = "http://127.0.0.1:9/should-not-follow".to_owned();
        let redirect = FixtureServer::start(vec![FixtureResponse::redirect(redirect_target)]);
        assert_eq!(
            MdsyncSessionStore::new()
                .unwrap()
                .connect(
                    "project-a".into(),
                    Zeroizing::new(redirect.workspace_url("")),
                    "agent-a".into()
                )
                .unwrap_err()
                .code,
            "transport_unavailable"
        );

        let oversized = FixtureServer::start(vec![FixtureResponse::json(
            200,
            vec![b' '; MAX_DISCOVERY_BYTES + 1],
        )]);
        assert_eq!(
            MdsyncSessionStore::new()
                .unwrap()
                .connect(
                    "project-a".into(),
                    Zeroizing::new(oversized.workspace_url("")),
                    "agent-a".into()
                )
                .unwrap_err()
                .code,
            "response_too_large"
        );
    }

    #[test]
    fn discovery_trust_policy_rejects_attacker_to_private_api_before_session_auth() {
        let attacker = Url::parse("https://attacker.example").unwrap();
        let private_api = Url::parse("https://127.0.0.1:9443").unwrap();
        let response = DiscoveryResponse {
            api_origin: private_api,
            web_origin: attacker.clone(),
        };
        assert_eq!(
            response.validate(&attacker).unwrap_err().code,
            "origin_mismatch"
        );

        let production_web = Url::parse("https://sync.ha2ha.md").unwrap();
        let production = DiscoveryResponse {
            api_origin: Url::parse("https://sync-api.ha2ha.md").unwrap(),
            web_origin: production_web.clone(),
        };
        assert!(production.validate(&production_web).is_ok());
        let substituted = DiscoveryResponse {
            api_origin: Url::parse("https://sync-api.ha2ha.md.attacker.example").unwrap(),
            web_origin: production_web.clone(),
        };
        assert_eq!(
            substituted.validate(&production_web).unwrap_err().code,
            "origin_mismatch"
        );

        let loopback = DiscoveryResponse {
            api_origin: Url::parse("http://127.0.0.1:3200").unwrap(),
            web_origin: Url::parse("http://[::1]:5173").unwrap(),
        };
        assert!(loopback
            .validate(&Url::parse("http://localhost:3000").unwrap())
            .is_ok());
    }

    #[test]
    fn capability_bearing_parse_url_is_debug_redacted_and_origin_is_query_free() {
        let sensitive = SensitiveUrl::parse(&format!(
            "https://api.example.com/w/workspace-1?edit={EDIT_SECRET}"
        ))
        .unwrap();
        assert_eq!(format!("{sensitive:?}"), "SensitiveUrl([REDACTED])");
        let origin = origin_url(&sensitive).unwrap();
        assert_eq!(origin.as_str(), "https://api.example.com/");
        assert!(origin.query().is_none());
        assert!(!origin.as_str().contains(EDIT_SECRET));
    }

    #[test]
    fn discovery_timeout_is_typed_and_secret_free() {
        let mut delayed = FixtureResponse::json(200, "{}");
        delayed.delay = Duration::from_millis(350);
        let server = FixtureServer::start(vec![delayed]);
        let error = MdsyncSessionStore::new()
            .unwrap()
            .connect(
                "project-a".into(),
                Zeroizing::new(server.workspace_url(&format!("?edit={EDIT_SECRET}"))),
                "agent-a".into(),
            )
            .unwrap_err();
        assert_eq!(error.code, "timeout");
        assert!(!serde_json::to_string(&error).unwrap().contains(EDIT_SECRET));
    }

    #[test]
    fn capability_alias_guard_rejects_exact_prefix_suffix_and_infix_forms() {
        const OPAQUE_ALIAS: &str = "K4mQ8vR2pN7xT1wF6dH9jL3sB5cG0yZu";
        let capability =
            SecretCapability::new(CapabilityKind::Edit, Zeroizing::new(OPAQUE_ALIAS.into()))
                .unwrap();

        for candidate in [
            OPAQUE_ALIAS.to_owned(),
            format!("{OPAQUE_ALIAS}-reviewer"),
            format!("reviewer-{OPAQUE_ALIAS}"),
            format!("reviewer-{OPAQUE_ALIAS}-remote"),
        ] {
            let error = capability.reject_aliases([candidate.as_str()]).unwrap_err();
            assert_eq!(error.code, "capability_material_rejected");
            let serialized = serde_json::to_string(&error).unwrap();
            assert!(!serialized.contains(OPAQUE_ALIAS));
            assert!(!serialized.contains(&candidate));
        }

        capability
            .reject_aliases(["reviewer-K4mQ8vR2", "pN7xT1wF6dH9jL3sB5cG0yZu"])
            .unwrap();
        assert!(!format!("{capability:?}").contains(OPAQUE_ALIAS));
    }

    #[test]
    fn opaque_capability_alias_is_rejected_before_connect_metadata_is_exposed() {
        const OPAQUE_ALIAS: &str = "N4qv7Zp2Ls9Kd3Mx8Wc6Rf1Hy5Tg0BjU";
        let server = FixtureServer::start(vec![discovery_response()]);
        let store = MdsyncSessionStore::new().unwrap();

        let error = store
            .connect(
                "project-capability-alias".into(),
                Zeroizing::new(server.workspace_url(&format!("?edit={OPAQUE_ALIAS}"))),
                OPAQUE_ALIAS.into(),
            )
            .unwrap_err();

        assert_eq!(error.code, "capability_material_rejected");
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains(OPAQUE_ALIAS));
        assert!(store.state.lock().unwrap().sessions.is_empty());
    }

    #[test]
    fn embedded_capability_alias_is_rejected_before_connect_metadata_is_exposed() {
        const OPAQUE_ALIAS: &str = "R8vQ2mK7pN4xT9wF3dH6jL1sB5cG0yZu";
        let server = FixtureServer::start(vec![discovery_response()]);
        let store = MdsyncSessionStore::new().unwrap();

        let error = store
            .connect(
                "project-embedded-capability-alias".into(),
                Zeroizing::new(server.workspace_url(&format!("?edit={OPAQUE_ALIAS}"))),
                format!("reviewer-{OPAQUE_ALIAS}"),
            )
            .unwrap_err();

        assert_eq!(error.code, "capability_material_rejected");
        let serialized = serde_json::to_string(&error).unwrap();
        assert!(!serialized.contains(OPAQUE_ALIAS));
        assert!(!serialized.contains("reviewer-"));
        assert!(store.state.lock().unwrap().sessions.is_empty());
    }

    #[test]
    fn forged_native_session_embedded_capability_alias_is_rejected_on_metadata_read() {
        const OPAQUE_ALIAS: &str = "X7mQ2vL9pR4sT8wY3kN6dF1hJ5cB0gZu";
        let store = MdsyncSessionStore::new().unwrap();
        let project_key = "project-forged-alias";
        let session_id = format!("local-session-{}", "f".repeat(32));
        store.select_project(project_key).unwrap();
        store.state.lock().unwrap().sessions.insert(
            session_id.clone(),
            Arc::new(NativeSession {
                project_key: project_key.into(),
                workspace_id: "workspace-1".into(),
                web_origin: "https://app.example.test".into(),
                api_origin: "https://api.example.test".into(),
                actor: format!("{OPAQUE_ALIAS}-reviewer"),
                capability: SecretCapability::new(
                    CapabilityKind::Edit,
                    Zeroizing::new(OPAQUE_ALIAS.into()),
                )
                .unwrap(),
            }),
        );

        let error = store
            .sanitized_session_metadata(project_key, &session_id)
            .unwrap_err();

        assert_eq!(error.code, "capability_material_rejected");
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains(OPAQUE_ALIAS));
    }

    #[test]
    fn completion_intent_guard_covers_every_string_field_and_artifact_coordinate() {
        let intent = completion_intent_fixture();
        let candidates = intent.capability_alias_candidates();
        assert_eq!(
            candidates,
            vec![
                intent.workspace_id.as_str(),
                intent.actor.as_str(),
                intent.task_id.as_str(),
                intent.remote_task_path.as_str(),
                intent.source_task_sha256.as_str(),
                intent.local_task_path.as_str(),
                intent.local_task_sha256.as_str(),
                intent.repository_id.as_str(),
                intent.run_id.as_str(),
                intent.evidence_id.as_str(),
                intent.evidence_path.as_str(),
                intent.handoff_id.as_str(),
                intent.handoff_path.as_str(),
                intent.artifacts[0].path.as_str(),
                intent.artifacts[0].sha256.as_str(),
            ]
        );

        for alias in candidates {
            let capability =
                SecretCapability::new(CapabilityKind::Edit, Zeroizing::new(alias.to_owned()))
                    .unwrap();
            let error = capability
                .reject_aliases(intent.capability_alias_candidates())
                .unwrap_err();
            assert_eq!(error.code, "capability_material_rejected");
            assert!(!serde_json::to_string(&error).unwrap().contains(alias));
        }
    }

    #[test]
    fn embedded_capability_alias_inputs_are_rejected_before_transport() {
        const OPAQUE_ALIAS: &str = "G6rP2xN9vQ4mT7kL3wF8dH1jS5cB0yZu";
        let server = FixtureServer::start(vec![discovery_response()]);
        let store = MdsyncSessionStore::new().unwrap();
        let metadata = connect(
            &store,
            &server,
            "project-path-alias",
            &format!("?edit={OPAQUE_ALIAS}"),
        );

        let read_error = store
            .read_file(
                "project-path-alias",
                metadata.session_id.as_str(),
                &format!("docs/{OPAQUE_ALIAS}-read.md"),
            )
            .unwrap_err();
        assert_eq!(read_error.code, "capability_material_rejected");
        let write_error = store
            .write_file(
                "project-path-alias",
                metadata.session_id.as_str(),
                MdsyncWriteInput {
                    path: "STATUS.md".into(),
                    content: format!("# unsafe prefix-{OPAQUE_ALIAS}-suffix"),
                    content_type: None,
                    base_version: None,
                },
            )
            .unwrap_err();
        assert_eq!(write_error.code, "capability_material_rejected");
        assert_eq!(server.requests().len(), 1);
        for output in [
            serde_json::to_string(&read_error).unwrap(),
            serde_json::to_string(&write_error).unwrap(),
        ] {
            assert!(!output.contains(OPAQUE_ALIAS));
        }
    }

    #[test]
    fn embedded_capability_alias_in_remote_actor_is_not_returned() {
        const OPAQUE_ALIAS: &str = "V3mK8qR1tN6xP4wY9dF2hJ7sL5cB0gZu";
        let embedded_actor = format!("reviewer-{OPAQUE_ALIAS}-remote");
        let listing = serde_json::json!({
            "files": [{
                "contentType": "text/markdown; charset=utf-8",
                "path": "STATUS.md",
                "updatedAt": "2026-07-23T10:00:00.000Z",
                "updatedBy": embedded_actor,
                "version": 1
            }],
            "workspaceId": "workspace-1"
        });
        let server = FixtureServer::start(vec![
            discovery_response(),
            FixtureResponse::json(200, listing.to_string()),
        ]);
        let store = MdsyncSessionStore::new().unwrap();
        let metadata = connect(
            &store,
            &server,
            "project-result-alias",
            &format!("?edit={OPAQUE_ALIAS}"),
        );

        let error = store
            .list_files("project-result-alias", metadata.session_id.as_str())
            .unwrap_err();

        assert_eq!(error.code, "capability_material_rejected");
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains(OPAQUE_ALIAS));
    }

    #[test]
    fn embedded_capability_alias_in_completion_intent_is_rejected_before_persistence() {
        const OPAQUE_ALIAS: &str = "D9mQ3vR7pN2xT8wF4dH6jL1sB5cG0yZu";
        let store = MdsyncSessionStore::new().unwrap();
        let project_key = "project-completion-alias";
        let session_id = format!("local-session-{}", "e".repeat(32));
        store
            .insert_forged_session_for_test(
                project_key,
                &session_id,
                "workspace-1",
                "agent-a",
                OPAQUE_ALIAS,
            )
            .unwrap();
        let mut intent = completion_intent_fixture();
        intent.actor = format!("reviewer-{OPAQUE_ALIAS}");

        let error = store
            .validate_completion_intent_for_persistence(project_key, &session_id, &intent)
            .unwrap_err();

        assert_eq!(error.code, "capability_material_rejected");
        assert!(!serde_json::to_string(&error)
            .unwrap()
            .contains(OPAQUE_ALIAS));
    }

    #[test]
    fn viewer_reads_and_lists_without_exposing_capability() {
        let server = FixtureServer::start(vec![
            discovery_response(),
            FixtureResponse::json(
                200,
                serde_json::json!({
                    "files": [{
                        "contentType": "text/markdown; charset=utf-8",
                        "path": "STATUS.md",
                        "updatedAt": "2026-07-23T10:00:00.000Z",
                        "updatedBy": null,
                        "version": 1
                    }],
                    "workspaceId": "workspace-1"
                })
                .to_string(),
            ),
            FixtureResponse::json(200, file_json("# Status\n", 1)),
        ]);
        let store = MdsyncSessionStore::new().unwrap();
        let metadata = connect(&store, &server, "project-a", &format!("?k={READ_SECRET}"));
        assert_eq!(metadata.access, CollaborationAccess::Viewer);
        let id = metadata.session_id.as_str();
        let listing = store.list_files("project-a", id).unwrap();
        let file = store.read_file("project-a", id, "STATUS.md").unwrap();
        assert_eq!(listing.files.len(), 1);
        assert_eq!(file.content, "# Status\n");
        for value in [
            serde_json::to_string(&metadata).unwrap(),
            serde_json::to_string(&listing).unwrap(),
            serde_json::to_string(&file).unwrap(),
            format!("{store:?}"),
        ] {
            assert!(!value.contains(READ_SECRET));
            assert!(!value.contains("?k="));
        }
        let requests = server.requests();
        assert!(requests[1].contains(&format!("k={READ_SECRET}")));
        assert!(requests[2].contains(&format!("k={READ_SECRET}")));
        assert!(!requests[1].to_ascii_lowercase().contains("authorization:"));
    }

    #[test]
    fn access_lifecycle_write_and_conflict_are_enforced() {
        let server = FixtureServer::start(vec![
            discovery_response(),
            FixtureResponse::json(
                200,
                serde_json::json!({
                    "path": "STATUS.md",
                    "updatedAt": "2026-07-23T10:01:00.000Z",
                    "updatedBy": "agent-a",
                    "version": 2,
                    "workspaceId": "workspace-1"
                })
                .to_string(),
            ),
            FixtureResponse::json(
                409,
                serde_json::json!({
                    "error": "version_conflict",
                    "latest": serde_json::from_str::<serde_json::Value>(&file_json("# latest\n", 3)).unwrap(),
                    "message": "File already changed."
                })
                .to_string(),
            ),
        ]);
        let store = MdsyncSessionStore::new().unwrap();
        let metadata = connect(
            &store,
            &server,
            "project-a",
            &format!("?edit={EDIT_SECRET}"),
        );
        assert_eq!(metadata.access, CollaborationAccess::Collaborator);
        let id = metadata.session_id.as_str().to_owned();
        let input = MdsyncWriteInput {
            path: "STATUS.md".into(),
            content: "# Status\n".into(),
            content_type: None,
            base_version: Some(1),
        };
        assert_eq!(
            store
                .write_file("project-a", &id, input.clone())
                .unwrap()
                .version,
            2
        );
        let conflict = store.write_file("project-a", &id, input).unwrap_err();
        assert_eq!(conflict.class, MdsyncTransportErrorClass::VersionConflict);
        assert_eq!(
            conflict
                .conflict
                .as_ref()
                .and_then(|details| details.latest.as_ref())
                .map(|latest| latest.version),
            Some(3)
        );
        let serialized = serde_json::to_string(&conflict).unwrap();
        assert!(!serialized.contains(EDIT_SECRET));
        assert!(!serialized.contains("# latest"));
        let requests = server.requests();
        for request in &requests[1..] {
            assert!(request.contains(&format!("authorization: Bearer {EDIT_SECRET}")));
            assert!(request.contains(r#""baseVersion":1"#));
            assert!(!request.lines().next().unwrap().contains(EDIT_SECRET));
        }

        store.disconnect("project-a", &id).unwrap();
        assert_eq!(
            store.list_files("project-a", &id).unwrap_err().code,
            "session_not_found"
        );
    }

    #[test]
    fn task_claim_transport_sends_one_exact_versioned_write() {
        let claimed = "---\nid: BR-016\ntitle: Shared collaborator execution\nstate: claimed\nowner: agent-a\nupdated_by: agent-a\nevidence:\n  - evidence/BR-016/claim.md\n---\n";
        let server = FixtureServer::start(vec![
            discovery_response(),
            FixtureResponse::json(
                200,
                serde_json::json!({
                    "path": "tasks/BR-016.md",
                    "updatedAt": "2026-07-23T10:01:00.000Z",
                    "updatedBy": "agent-a",
                    "version": 8,
                    "workspaceId": "workspace-1"
                })
                .to_string(),
            ),
        ]);
        let store = MdsyncSessionStore::new().unwrap();
        let metadata = connect(
            &store,
            &server,
            "project-a",
            &format!("?edit={EDIT_SECRET}"),
        );

        let result = store
            .write_file(
                "project-a",
                metadata.session_id.as_str(),
                MdsyncWriteInput {
                    path: "tasks/BR-016.md".into(),
                    content: claimed.into(),
                    content_type: None,
                    base_version: Some(7),
                },
            )
            .unwrap();

        assert_eq!(result.path(), "tasks/BR-016.md");
        assert_eq!(result.version(), 8);
        let requests = server.requests();
        assert_eq!(requests.len(), 2);
        let request = &requests[1];
        assert!(request.starts_with("PUT /api/workspaces/workspace-1/files HTTP/1.1"));
        assert!(request.contains(r#""baseVersion":7"#));
        assert!(request.contains(r#""path":"tasks/BR-016.md""#));
        assert!(request.contains("state: claimed"));
        assert!(request.contains("owner: agent-a"));
        assert!(request.contains("updated_by: agent-a"));
        assert!(!request.lines().next().unwrap().contains(EDIT_SECRET));
        let serialized = serde_json::to_string(&result).unwrap();
        assert!(!serialized.contains(EDIT_SECRET));
        assert!(!serialized.to_ascii_lowercase().contains("authorization"));
    }

    #[test]
    fn concrete_claim_port_revalidates_writes_reads_back_and_claims_once() {
        let project = tempfile::tempdir().unwrap();
        assert!(Command::new("git")
            .args(["init", "-q"])
            .current_dir(project.path())
            .status()
            .unwrap()
            .success());
        fs::create_dir_all(project.path().join("tasks/issues")).unwrap();
        fs::write(
            project.path().join("tasks/issues/016-fixture.md"),
            "# 016: Fixture\n\nStatus: ready\n",
        )
        .unwrap();
        let root = fs::canonicalize(project.path()).unwrap();
        let local = local_collaboration_binding(
            &root,
            "tasks/issues/016-fixture.md",
            "# 016: Fixture\n\nStatus: ready\n",
        )
        .unwrap();
        let workspace = project_workspace(ProjectionInput {
            workspace_id: "workspace-1".into(),
            actor: "agent-a".into(),
            task_id: "BR-016".into(),
            title: "Claim one shared task".into(),
            status: ResolverTaskStatus::Ready,
            requirement_basis: vec!["tasks/issues/016-fixture.md".into()],
            local: local.clone(),
        })
        .unwrap();
        let mut remote_files = workspace
            .files
            .iter()
            .map(|file| EnvelopeRemoteWorkspaceFile {
                path: file.path.clone(),
                content: file.content.clone(),
                version: 1,
            })
            .collect::<Vec<_>>();
        remote_files.sort_by(|left, right| left.path.cmp(&right.path));
        let remote = RemoteTaskBinding {
            task_id: "BR-016".into(),
            task_path: workspace.task_path.clone(),
            base_version: 1,
        };
        let remote_task = remote_files
            .iter()
            .find(|file| file.path == remote.task_path)
            .unwrap();
        let claim = project_task_claim("agent-a", &remote, remote_task).unwrap();

        let mut responses = vec![
            discovery_response(),
            FixtureResponse::json(200, workspace_listing_json(&remote_files, "agent-a")),
        ];
        responses.extend(remote_files.iter().map(|file| {
            FixtureResponse::json(
                200,
                workspace_file_json(
                    &file.path,
                    &file.content,
                    if file.path.ends_with(".json") {
                        "application/json"
                    } else {
                        "text/markdown; charset=utf-8"
                    },
                    file.version,
                    "agent-a",
                ),
            )
        }));
        responses.push(FixtureResponse::json(
            200,
            serde_json::json!({
                "path": claim.path,
                "updatedAt": "2026-07-23T10:01:00.000Z",
                "updatedBy": "agent-a",
                "version": claim.expected_post_version,
                "workspaceId": "workspace-1"
            })
            .to_string(),
        ));
        responses.push(FixtureResponse::json(
            200,
            workspace_file_json(
                &claim.path,
                &claim.content,
                &claim.content_type,
                claim.expected_post_version,
                "agent-a",
            ),
        ));
        let server = FixtureServer::start(responses);
        let store = MdsyncSessionStore::new().unwrap();
        let project_key = root.to_string_lossy().to_string();
        let session = connect(
            &store,
            &server,
            &project_key,
            &format!("?edit={EDIT_SECRET}"),
        );
        let binding =
            SharedExecutionBinding::new(session.clone(), local.clone(), remote.clone()).unwrap();
        let port = MdsyncClaimPort::new(
            &store,
            project_key,
            session.session_id.as_str().into(),
            binding.clone(),
            Arc::new(OperationRegistry::default()),
        );

        let outcome = port
            .before_runtime(
                &PreRunCollaborationContext {
                    mode: CollaborationMode::SharedCollaborator,
                    session: Some(session),
                    local,
                    remote: Some(remote),
                },
                &AtomicBool::new(false),
            )
            .unwrap();

        assert_eq!(outcome.reconciliation, ReconciliationState::Claimed);
        assert_eq!(
            outcome.claim,
            ClaimResult::Claimed {
                remote_version: claim.expected_post_version,
            }
        );
        assert_eq!(
            port.state().unwrap(),
            SharedClaimState::Claimed {
                remote_version: claim.expected_post_version,
                recovered_from_readback: false,
            }
        );
        let requests = server.requests();
        assert_eq!(requests.len(), 11);
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.starts_with("PUT "))
                .count(),
            1
        );
        let put = requests
            .iter()
            .find(|request| request.starts_with("PUT "))
            .unwrap();
        assert!(put.contains(r#""baseVersion":1"#));
        assert!(put.contains("state: claimed"));
        assert!(put.contains(r#"owner: \"agent-a\""#));
        let serialized = serde_json::to_string(&(binding, port.state().unwrap())).unwrap();
        assert!(!serialized.contains(EDIT_SECRET));
        assert!(!serialized.to_ascii_lowercase().contains("authorization"));
    }

    #[test]
    fn concrete_post_commit_port_writes_exact_sanitized_sequence_without_runtime() {
        let project = tempfile::tempdir().unwrap();
        assert!(Command::new("git")
            .args(["init", "-q"])
            .current_dir(project.path())
            .status()
            .unwrap()
            .success());
        fs::create_dir_all(project.path().join("tasks/issues")).unwrap();
        let task_path = "tasks/issues/017-fixture.md";
        let ready_task = "# 017: Fixture\n\nStatus: ready\n";
        let completed_task = "# 017: Fixture\n\nStatus: complete\n\n## Acceptance Criteria\n\n- [x] repository proof\n\n## Evidence Log\n\n| command | result | notes |\n| --- | --- | --- |\n| cargo test | pass | deterministic |\n\n## Verification Summary\n\nPassed.\n";
        fs::write(project.path().join(task_path), ready_task).unwrap();
        let root = fs::canonicalize(project.path()).unwrap();
        let source_local = local_collaboration_binding(&root, task_path, ready_task).unwrap();
        fs::write(project.path().join(task_path), completed_task).unwrap();
        let completed_local =
            local_collaboration_binding(&root, task_path, completed_task).unwrap();
        fs::write(project.path().join(task_path), ready_task).unwrap();

        let workspace = project_workspace(ProjectionInput {
            workspace_id: "workspace-1".into(),
            actor: "agent-a".into(),
            task_id: "BR-017".into(),
            title: "Complete one shared task".into(),
            status: ResolverTaskStatus::Ready,
            requirement_basis: vec![task_path.into()],
            local: source_local.clone(),
        })
        .unwrap();
        let mut remote_files = workspace
            .files
            .iter()
            .map(|file| EnvelopeRemoteWorkspaceFile {
                path: file.path.clone(),
                content: file.content.clone(),
                version: 1,
            })
            .collect::<Vec<_>>();
        remote_files.sort_by(|left, right| left.path.cmp(&right.path));
        let remote = RemoteTaskBinding {
            task_id: "BR-017".into(),
            task_path: workspace.task_path.clone(),
            base_version: 1,
        };
        let claim = project_task_claim(
            "agent-a",
            &remote,
            remote_files
                .iter()
                .find(|file| file.path == remote.task_path)
                .unwrap(),
        )
        .unwrap();
        let mut claimed_files = remote_files.clone();
        let claimed_task = claimed_files
            .iter_mut()
            .find(|file| file.path == remote.task_path)
            .unwrap();
        claimed_task.content = claim.content.clone();
        claimed_task.version = claim.expected_post_version;
        let intent = RemoteCompletionIntent::new(
            "workspace-1".into(),
            "agent-a".into(),
            "BR-017".into(),
            remote.task_path.clone(),
            claim.expected_post_version,
            source_local.task_sha256.clone(),
            completed_local.task_path.clone(),
            completed_local.task_sha256.clone(),
            completed_local.repository_id.clone(),
            "0123456789abcdef0123456789abcdef".into(),
            1,
            format!("evidence-{}", "1".repeat(32)),
            format!("evidence/BR-017/completion-{}.md", "1".repeat(32)),
            format!("handoff-{}", "2".repeat(32)),
            format!("logs/BR-017-handoff-{}.md", "2".repeat(32)),
            vec![CompletionArtifact {
                path: task_path.into(),
                sha256: completed_local.task_sha256.clone(),
            }],
        )
        .unwrap();
        let post_plan = project_post_run_reconciliation(&intent, &claimed_files).unwrap();
        assert_eq!(post_plan.writes.len(), 4);

        let mut responses = vec![
            discovery_response(),
            FixtureResponse::json(200, workspace_listing_json(&remote_files, "agent-a")),
        ];
        responses.extend(remote_files.iter().map(|file| {
            FixtureResponse::json(
                200,
                workspace_file_json(
                    &file.path,
                    &file.content,
                    if file.path.ends_with(".json") {
                        "application/json"
                    } else {
                        "text/markdown; charset=utf-8"
                    },
                    file.version,
                    "agent-a",
                ),
            )
        }));
        responses.push(FixtureResponse::json(
            200,
            serde_json::json!({
                "path": claim.path,
                "updatedAt": "2026-07-23T10:01:00.000Z",
                "updatedBy": "agent-a",
                "version": claim.expected_post_version,
                "workspaceId": "workspace-1"
            })
            .to_string(),
        ));
        responses.push(FixtureResponse::json(
            200,
            workspace_file_json(
                &claim.path,
                &claim.content,
                &claim.content_type,
                claim.expected_post_version,
                "agent-a",
            ),
        ));
        responses.push(FixtureResponse::json(
            200,
            workspace_listing_json(&claimed_files, "agent-a"),
        ));
        responses.extend(claimed_files.iter().map(|file| {
            FixtureResponse::json(
                200,
                workspace_file_json(
                    &file.path,
                    &file.content,
                    if file.path.ends_with(".json") {
                        "application/json"
                    } else {
                        "text/markdown; charset=utf-8"
                    },
                    file.version,
                    "agent-a",
                ),
            )
        }));
        responses.extend(post_plan.writes.iter().map(|write| {
            FixtureResponse::json(
                200,
                serde_json::json!({
                    "path": write.path,
                    "updatedAt": "2026-07-23T10:02:00.000Z",
                    "updatedBy": "agent-a",
                    "version": write.expected_post_version,
                    "workspaceId": "workspace-1"
                })
                .to_string(),
            )
        }));

        let server = FixtureServer::start(responses);
        let store = MdsyncSessionStore::new().unwrap();
        let project_key = root.to_string_lossy().to_string();
        let session = connect(
            &store,
            &server,
            &project_key,
            &format!("?edit={EDIT_SECRET}"),
        );
        let binding =
            SharedExecutionBinding::new(session.clone(), source_local.clone(), remote.clone())
                .unwrap();
        let port = MdsyncClaimPort::new(
            &store,
            project_key,
            session.session_id.as_str().into(),
            binding,
            Arc::new(OperationRegistry::default()),
        );
        port.before_runtime(
            &PreRunCollaborationContext {
                mode: CollaborationMode::SharedCollaborator,
                session: Some(session.clone()),
                local: source_local,
                remote: Some(remote.clone()),
            },
            &AtomicBool::new(false),
        )
        .unwrap();
        fs::write(project.path().join(task_path), completed_task).unwrap();

        let outcome = run_after_local_commit(
            &port,
            &PostLocalCommitCollaborationContext {
                mode: CollaborationMode::SharedCollaborator,
                session: Some(session),
                local: completed_local,
                remote: Some(remote),
                run_id: intent.run_id.clone(),
                intent: Some(intent),
            },
        )
        .unwrap();

        assert_eq!(outcome.reconciliation, ReconciliationState::Reconciled);
        assert!(matches!(
            outcome.evidence_handoff,
            EvidenceHandoffResult::Synchronized {
                remote_version: 3,
                ..
            }
        ));
        let requests = server.requests();
        let puts = requests
            .iter()
            .filter(|request| request.starts_with("PUT "))
            .collect::<Vec<_>>();
        assert_eq!(puts.len(), 5);
        assert!(puts[1].contains(r#""baseVersion":null"#));
        assert!(puts[1].contains("completion-11111111111111111111111111111111.md"));
        assert!(puts[1].contains("Source summary:"));
        assert!(puts[2].contains(r#""baseVersion":2"#));
        assert!(puts[2].contains("state: done"));
        assert!(puts[3].contains(r#""baseVersion":null"#));
        assert!(puts[3].contains("logs/BR-017-handoff-"));
        assert!(puts[4].contains(r#""baseVersion":1"#));
        assert!(puts[4].contains("build-right-completion:evidence-"));
        let serialized_post_run_requests = puts[1..]
            .iter()
            .map(|request| {
                request
                    .split_once("\r\n\r\n")
                    .map(|(_, body)| body)
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join("\n")
            .to_ascii_lowercase();
        for forbidden in [
            EDIT_SECRET.to_ascii_lowercase(),
            "authorization".into(),
            "bearer ".into(),
            "provider payload".into(),
            "raw payload".into(),
        ] {
            assert!(
                !serialized_post_run_requests.contains(&forbidden),
                "{forbidden}"
            );
        }
    }

    #[test]
    fn committed_write_with_lost_response_is_reconciled_by_one_exact_read() {
        let content = "---\nid: BR-015\nstate: ready\n---\n";
        let server = FixtureServer::start(vec![
            discovery_response(),
            FixtureResponse::json(200, "not-json"),
            FixtureResponse::json(200, task_file_json(content, 1, "agent-a")),
        ]);
        let store = MdsyncSessionStore::new().unwrap();
        let metadata = connect(
            &store,
            &server,
            "project-a",
            &format!("?edit={EDIT_SECRET}"),
        );
        let result = store
            .write_file_with_readback(
                "project-a",
                metadata.session_id.as_str(),
                MdsyncWriteInput {
                    path: "tasks/BR-015.md".into(),
                    content: content.into(),
                    content_type: Some("text/markdown; charset=utf-8".into()),
                    base_version: None,
                },
                1,
            )
            .unwrap();
        assert_eq!(result.path(), "tasks/BR-015.md");
        assert_eq!(result.version(), 1);
        assert!(result.recovered_from_readback());
        let requests = server.requests();
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.starts_with("PUT "))
                .count(),
            1
        );
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.starts_with("GET "))
                .count(),
            2
        );
    }

    #[test]
    fn lost_create_response_rejects_identical_readback_at_later_version() {
        let content = "---\nid: BR-015\nstate: ready\n---\n";
        let server = FixtureServer::start(vec![
            discovery_response(),
            FixtureResponse::json(200, "not-json"),
            FixtureResponse::json(200, task_file_json(content, 2, "agent-a")),
        ]);
        let store = MdsyncSessionStore::new().unwrap();
        let metadata = connect(
            &store,
            &server,
            "project-a",
            &format!("?edit={EDIT_SECRET}"),
        );
        let error = store
            .write_file_with_readback(
                "project-a",
                metadata.session_id.as_str(),
                MdsyncWriteInput {
                    path: "tasks/BR-015.md".into(),
                    content: content.into(),
                    content_type: Some("text/markdown; charset=utf-8".into()),
                    base_version: None,
                },
                1,
            )
            .unwrap_err();
        assert_eq!(error.code, "invalid_response");
        let requests = server.requests();
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.starts_with("PUT "))
                .count(),
            1
        );
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.starts_with("GET "))
                .count(),
            2
        );
    }

    #[test]
    fn write_readback_mismatch_does_not_claim_commit() {
        let server = FixtureServer::start(vec![
            discovery_response(),
            FixtureResponse::json(200, "not-json"),
            FixtureResponse::json(200, task_file_json("different", 1, "agent-a")),
        ]);
        let store = MdsyncSessionStore::new().unwrap();
        let metadata = connect(
            &store,
            &server,
            "project-a",
            &format!("?edit={EDIT_SECRET}"),
        );
        let error = store
            .write_file_with_readback(
                "project-a",
                metadata.session_id.as_str(),
                MdsyncWriteInput {
                    path: "tasks/BR-015.md".into(),
                    content: "expected".into(),
                    content_type: Some("text/markdown; charset=utf-8".into()),
                    base_version: None,
                },
                1,
            )
            .unwrap_err();
        assert_eq!(error.code, "invalid_response");
        assert_eq!(server.requests().len(), 3);
    }

    #[test]
    fn write_readback_failure_does_not_retry_mutation_or_claim_commit() {
        let server = FixtureServer::start(vec![
            discovery_response(),
            FixtureResponse::json(200, "not-json"),
            FixtureResponse::json(503, "{}"),
        ]);
        let store = MdsyncSessionStore::new().unwrap();
        let metadata = connect(
            &store,
            &server,
            "project-a",
            &format!("?edit={EDIT_SECRET}"),
        );
        let error = store
            .write_file_with_readback(
                "project-a",
                metadata.session_id.as_str(),
                MdsyncWriteInput {
                    path: "tasks/BR-015.md".into(),
                    content: "expected".into(),
                    content_type: Some("text/markdown; charset=utf-8".into()),
                    base_version: None,
                },
                1,
            )
            .unwrap_err();
        assert_eq!(error.code, "invalid_response");
        let requests = server.requests();
        assert_eq!(
            requests
                .iter()
                .filter(|request| request.starts_with("PUT "))
                .count(),
            1
        );
        assert_eq!(requests.len(), 3);
    }

    #[test]
    fn viewer_mutation_is_denied_before_network_and_project_switch_clears_sessions() {
        let server = FixtureServer::start(vec![discovery_response()]);
        let store = MdsyncSessionStore::new().unwrap();
        let viewer = connect(&store, &server, "project-a", &format!("?k={READ_SECRET}"));
        let input = MdsyncWriteInput {
            path: "STATUS.md".into(),
            content: "# Status\n".into(),
            content_type: None,
            base_version: Some(1),
        };
        assert_eq!(
            store
                .write_file("project-a", viewer.session_id.as_str(), input)
                .unwrap_err()
                .code,
            "access_denied"
        );
        assert_eq!(server.requests().len(), 1);
        store.activate_project("project-b").unwrap();
        assert_eq!(
            store
                .list_files("project-a", viewer.session_id.as_str())
                .unwrap_err()
                .code,
            "session_not_found"
        );
    }

    #[test]
    fn capability_material_is_rejected_from_inputs_results_errors_and_debug() {
        let server = FixtureServer::start(vec![
            discovery_response(),
            FixtureResponse::json(
                200,
                file_json(&format!("unsafe prefix-{EDIT_SECRET}-suffix"), 1),
            ),
        ]);
        let store = MdsyncSessionStore::new().unwrap();
        let metadata = connect(
            &store,
            &server,
            "project-a",
            &format!("?edit={EDIT_SECRET}"),
        );
        let error = store
            .read_file("project-a", metadata.session_id.as_str(), "STATUS.md")
            .unwrap_err();
        assert_eq!(error.code, "capability_material_rejected");
        for output in [
            serde_json::to_string(&metadata).unwrap(),
            serde_json::to_string(&error).unwrap(),
            format!("{error:?}"),
            format!("{store:?}"),
        ] {
            assert!(!output.contains(EDIT_SECRET));
            assert!(!output.to_ascii_lowercase().contains("authorization"));
        }

        let write_error = store
            .write_file(
                "project-a",
                metadata.session_id.as_str(),
                MdsyncWriteInput {
                    path: "STATUS.md".into(),
                    content: format!("unsafe {EDIT_SECRET}"),
                    content_type: None,
                    base_version: Some(1),
                },
            )
            .unwrap_err();
        assert_eq!(write_error.code, "capability_material_rejected");
        assert_eq!(server.requests().len(), 2);
    }

    #[test]
    fn operation_responses_are_size_bounded() {
        let server = FixtureServer::start(vec![
            discovery_response(),
            FixtureResponse::json(200, vec![b' '; MAX_RESPONSE_BYTES + 1]),
        ]);
        let store = MdsyncSessionStore::new().unwrap();
        let metadata = connect(&store, &server, "project-a", "");
        assert_eq!(
            store
                .read_file("project-a", metadata.session_id.as_str(), "STATUS.md")
                .unwrap_err()
                .code,
            "response_too_large"
        );
    }

    #[test]
    fn raw_urls_and_decoded_capabilities_are_bounded_before_retention() {
        let huge = format!(
            "https://app.example.com/w/workspace-1?edit={}",
            "x".repeat(2 * 1024 * 1024)
        );
        assert_eq!(
            parse_workspace_url(&huge).unwrap_err().code,
            "invalid_workspace_url"
        );
        let oversized_capability = format!(
            "https://app.example.com/w/workspace-1?edit={}",
            "x".repeat(MAX_CAPABILITY_BYTES + 1)
        );
        assert_eq!(
            parse_workspace_url(&oversized_capability).unwrap_err().code,
            "invalid_workspace_url"
        );
        let encoded_oversized = format!(
            "https://app.example.com/w/workspace-1?k={}",
            "%41".repeat(MAX_CAPABILITY_BYTES + 1)
        );
        assert_eq!(
            parse_workspace_url(&encoded_oversized).unwrap_err().code,
            "invalid_workspace_url"
        );
    }

    #[test]
    fn slow_connect_cannot_resurrect_a_session_after_project_switch() {
        let mut discovery = discovery_response();
        discovery.delay = Duration::from_millis(100);
        let server = FixtureServer::start(vec![discovery]);
        let store = Arc::new(MdsyncSessionStore::new().unwrap());
        let worker_store = Arc::clone(&store);
        let url = Zeroizing::new(server.workspace_url(&format!("?edit={EDIT_SECRET}")));
        let worker =
            thread::spawn(move || worker_store.connect("project-a".into(), url, "agent-a".into()));
        server.wait_for_requests(1);
        store.activate_project("project-b").unwrap();
        let error = worker.join().unwrap().unwrap_err();
        assert_eq!(error.code, "selection_changed");
        assert!(!serde_json::to_string(&error).unwrap().contains(EDIT_SECRET));
    }

    #[test]
    fn project_switch_fails_closed_while_remote_mutation_is_in_flight() {
        let mut write = FixtureResponse::json(
            200,
            serde_json::json!({
                "path": "STATUS.md",
                "updatedAt": "2026-07-23T10:01:00.000Z",
                "updatedBy": "agent-a",
                "version": 2,
                "workspaceId": "workspace-1"
            })
            .to_string(),
        );
        write.delay = Duration::from_millis(100);
        let server = FixtureServer::start(vec![discovery_response(), write]);
        let store = Arc::new(MdsyncSessionStore::new().unwrap());
        let metadata = connect(
            &store,
            &server,
            "project-a",
            &format!("?edit={EDIT_SECRET}"),
        );
        let session_id = metadata.session_id.as_str().to_owned();
        let worker_store = Arc::clone(&store);
        let worker = thread::spawn(move || {
            worker_store.write_file(
                "project-a",
                &session_id,
                MdsyncWriteInput {
                    path: "STATUS.md".into(),
                    content: "# Status\n".into(),
                    content_type: None,
                    base_version: Some(1),
                },
            )
        });
        server.wait_for_requests(2);
        assert_eq!(
            store.activate_project("project-b").unwrap_err().code,
            "project_busy"
        );
        assert_eq!(worker.join().unwrap().unwrap().version, 2);
        store.activate_project("project-b").unwrap();
    }
}

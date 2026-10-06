//! Local NTFS metadata adapter. Application authorization is an injected seam;
//! no default permit-all host, IPC command, content handle or UI activation.
use crate::{
    AbsolutePath, AuthorityFence, CapturedInput, Child, Context, MetadataPort, ObjectIdentity,
    Operations, PortError, Qualified, RequestFailure, SourceFingerprint, UncheckedReason,
};
use std::sync::Arc;

// All unsafe operations are confined to this reviewed syscall module.
#[allow(unsafe_code)]
mod ffi;
#[cfg(test)]
mod tests;

/// The future host verifies enabled/current root, location, snapshot, displayed
/// fingerprint and session under its DB guard, then releases it before return.
/// Revocation must also signal RequestControl during a blocked syscall.
pub trait CurrentAuthorization {
    fn current(&mut self, context: &Context, fence: &AuthorityFence) -> Result<(), RequestFailure>;
}

pub struct WindowsPort<F> {
    authorization: F,
    #[cfg(test)]
    hook: Option<TestHook>,
    #[cfg(test)]
    attributes_filter: Option<AttributeFilter>,
}
impl<F: CurrentAuthorization> WindowsPort<F> {
    pub fn new(authorization: F) -> Self {
        Self {
            authorization,
            #[cfg(test)]
            hook: None,
            #[cfg(test)]
            attributes_filter: None,
        }
    }

    fn current(&mut self, context: &Context, fence: &AuthorityFence) -> Result<(), PortError> {
        self.authorization
            .current(context, fence)
            .map_err(PortError::Request)
    }

    fn open_child(
        &mut self,
        parent: &Arc<DirectoryNode>,
        name: &str,
        wanted_directory: Option<bool>,
        operations: &mut Operations,
    ) -> Result<Option<(ffi::Handle, Metadata)>, PortError> {
        // Defense in depth for direct trusted Rust callers: never pass arbitrary
        // text to an API whose relative name could contain another path.
        let parsed = AbsolutePath::parse(&format!("C:\\{name}")).map_err(PortError::Unchecked)?;
        if parsed.components().len() != 1 {
            return Err(ffi::unchecked(UncheckedReason::UnsupportedPathSyntax));
        }
        validate_directory(parent, operations)?;
        #[cfg(test)]
        self.fire(TestStage::BeforeQuery, name);
        let attributes =
            operations.perform(|| ffi::attributes(&parent.handle, name, !parent.sensitive))?;
        let Some(attributes) = attributes else {
            validate_directory(parent, operations)?;
            return Ok(None);
        };
        #[cfg(test)]
        let attributes = {
            let mut attributes = attributes;
            if let Some(filter) = &mut self.attributes_filter {
                filter(name, &mut attributes);
            }
            attributes
        };
        eligible_attributes(attributes.attributes)?;
        let directory = attributes.attributes & ffi::DIRECTORY != 0;
        if wanted_directory.is_some_and(|wanted| wanted != directory) {
            return Err(ffi::unchecked(UncheckedReason::NotRegularFile));
        }
        #[cfg(test)]
        self.fire(TestStage::BeforeOpen, name);
        let token = operations.acquire_handle()?;
        let handle = operations.perform(|| {
            ffi::open(
                Some(&parent.handle),
                name,
                !parent.sensitive,
                directory,
                token,
            )
        })?;
        #[cfg(test)]
        self.fire(TestStage::AfterOpen, name);
        let metadata = observe(&handle, operations)?;
        if metadata.directory != directory || metadata.identity.volume != parent.identity.volume {
            return Err(ffi::unchecked(UncheckedReason::UnqualifiedFilesystem));
        }
        validate_directory(parent, operations)?;
        Ok(Some((handle, metadata)))
    }

    #[cfg(test)]
    fn fire(&mut self, stage: TestStage, name: &str) {
        if let Some(hook) = &mut self.hook {
            hook(stage, name);
        }
    }
}

// Private inputs/guards intentionally have no Debug or raw-handle accessor.
pub struct WindowsAuthority {
    context: Context,
    fence: AuthorityFence,
    drive: u8,
    mapping: String,
    root_chain: Vec<Arc<DirectoryNode>>,
    source_parent: Arc<DirectoryNode>,
    source_name: String,
    source: ffi::Handle,
}
pub struct WindowsDirectory {
    node: Arc<DirectoryNode>,
}
struct DirectoryNode {
    handle: ffi::Handle,
    identity: ObjectIdentity,
    sensitive: bool,
    parent: Option<Arc<DirectoryNode>>,
    name: String,
}
struct Metadata {
    identity: ObjectIdentity,
    size: u64,
    modified_ns: i64,
    directory: bool,
}

fn stale() -> PortError {
    PortError::Request(RequestFailure::Stale)
}
fn eligible_attributes(attributes: u32) -> Result<(), PortError> {
    if attributes & ffi::EXCLUDED != 0 {
        return Err(ffi::unchecked(UncheckedReason::ReparseOrOffline));
    }
    if attributes & 0x40 != 0 {
        return Err(ffi::unchecked(UncheckedReason::NotRegularFile));
    }
    Ok(())
}
fn observe(handle: &ffi::Handle, operations: &mut Operations) -> Result<Metadata, PortError> {
    let basic = operations.perform(|| ffi::basic(handle))?;
    eligible_attributes(basic.attributes)?;
    let standard = operations.perform(|| ffi::standard(handle))?;
    if standard.size < 0 || standard.delete_pending != 0 || standard.directory > 1 {
        return Err(ffi::unchecked(UncheckedReason::UnqualifiedFilesystem));
    }
    let directory = basic.attributes & ffi::DIRECTORY != 0;
    if directory != (standard.directory == 1) {
        return Err(ffi::unchecked(UncheckedReason::UnqualifiedFilesystem));
    }
    let identity = operations.perform(|| ffi::identity(handle))?;
    let modified_ns =
        i64::try_from((i128::from(basic.modified) - 116_444_736_000_000_000i128) * 100)
            .map_err(|_| ffi::unchecked(UncheckedReason::UnqualifiedFilesystem))?;
    Ok(Metadata {
        identity,
        size: standard.size as u64,
        modified_ns,
        directory,
    })
}
fn validate_directory(node: &DirectoryNode, operations: &mut Operations) -> Result<(), PortError> {
    let basic = operations.perform(|| ffi::basic(&node.handle))?;
    if eligible_attributes(basic.attributes).is_err() || basic.attributes & ffi::DIRECTORY == 0 {
        return Err(stale());
    }
    let identity = operations.perform(|| ffi::identity(&node.handle))?;
    let sensitive = operations.perform(|| ffi::case_sensitive(&node.handle))?;
    if identity != node.identity || sensitive != node.sensitive {
        return Err(stale());
    }
    Ok(())
}
fn directory_node(
    handle: ffi::Handle,
    metadata: Metadata,
    parent: Option<Arc<DirectoryNode>>,
    name: String,
    operations: &mut Operations,
) -> Result<Arc<DirectoryNode>, PortError> {
    if !metadata.directory {
        return Err(ffi::unchecked(UncheckedReason::NotRegularFile));
    }
    let sensitive = operations.perform(|| ffi::case_sensitive(&handle))?;
    Ok(Arc::new(DirectoryNode {
        handle,
        identity: metadata.identity,
        sensitive,
        parent,
        name,
    }))
}
fn fingerprint_matches(metadata: &Metadata, expected: &SourceFingerprint) -> bool {
    !metadata.directory
        && metadata.identity == expected.identity
        && metadata.size == expected.byte_size
        && metadata.modified_ns == expected.modified_at_ns
}

impl<F: CurrentAuthorization> MetadataPort for WindowsPort<F> {
    type Authority = WindowsAuthority;
    type Directory = WindowsDirectory;

    fn qualify(
        &mut self,
        input: &CapturedInput,
        operations: &mut Operations,
    ) -> Result<Qualified<WindowsAuthority, WindowsDirectory>, PortError> {
        self.current(input.context(), input.fence())?;
        let root = input.root();
        let source = input.source();
        if source.drive() != root.drive()
            || source.components().len() <= root.components().len()
            || !source.components().starts_with(root.components())
        {
            return Err(stale());
        }
        let mapping = operations.perform(|| ffi::drive_mapping(root.drive()))?;
        let token = operations.acquire_handle()?;
        let handle = operations
            .perform(|| ffi::open(None, &(mapping.clone() + "\\"), false, true, token))?;
        operations.perform(|| ffi::filesystem(&handle))?;
        let metadata = observe(&handle, operations)?;
        let anchor = directory_node(handle, metadata, None, String::new(), operations)?;
        let mut root_chain = vec![anchor];
        for name in root.components() {
            let parent = root_chain.last().ok_or_else(stale)?;
            let (handle, metadata) = self
                .open_child(parent, name, Some(true), operations)?
                .ok_or_else(stale)?;
            root_chain.push(directory_node(
                handle,
                metadata,
                Some(parent.clone()),
                name.clone(),
                operations,
            )?);
        }
        let root_node = root_chain.last().ok_or_else(stale)?.clone();
        if input
            .fence()
            .root_identity
            .as_ref()
            .is_some_and(|expected| expected != &root_node.identity)
        {
            return Err(stale());
        }
        let mut source_parent = root_node.clone();
        let suffix = &source.components()[root.components().len()..];
        for name in &suffix[..suffix.len() - 1] {
            let (handle, metadata) = self
                .open_child(&source_parent, name, Some(true), operations)?
                .ok_or_else(stale)?;
            source_parent = directory_node(
                handle,
                metadata,
                Some(source_parent.clone()),
                name.clone(),
                operations,
            )?;
        }
        let source_name = suffix.last().ok_or_else(stale)?.clone();
        let (source_handle, metadata) = self
            .open_child(&source_parent, &source_name, Some(false), operations)?
            .ok_or_else(stale)?;
        if !fingerprint_matches(&metadata, &input.fence().source) {
            return Err(stale());
        }
        if operations.perform(|| ffi::drive_mapping(root.drive()))? != mapping {
            return Err(stale());
        }
        self.current(input.context(), input.fence())?;
        Ok(Qualified {
            authority: WindowsAuthority {
                context: input.context().clone(),
                fence: input.fence().clone(),
                drive: root.drive(),
                mapping,
                root_chain,
                source_parent,
                source_name,
                source: source_handle,
            },
            root_directory: WindowsDirectory { node: root_node },
        })
    }

    fn root_component_matches(
        &mut self,
        authority: &WindowsAuthority,
        index: usize,
        saved: &str,
        operations: &mut Operations,
    ) -> Result<bool, PortError> {
        let node = authority.root_chain.get(index + 1).ok_or_else(stale)?;
        if saved == node.name {
            return Ok(true);
        }
        let parent = node.parent.as_ref().ok_or_else(stale)?;
        if !parent.sensitive
            && operations.perform(|| ffi::equal_ignoring_case(saved, &node.name))?
        {
            // OS casing alone is not proof of this volume's on-disk equivalence.
            // Do not open an unproven root variant from a broader ancestor.
            return Err(ffi::unchecked(UncheckedReason::UnsupportedCaseMode));
        }
        Ok(false)
    }

    fn child(
        &mut self,
        authority: &WindowsAuthority,
        parent: &WindowsDirectory,
        name: &str,
        operations: &mut Operations,
    ) -> Result<Child<WindowsDirectory>, PortError> {
        self.current(&authority.context, &authority.fence)?;
        let result = self.open_child(&parent.node, name, None, operations)?;
        self.current(&authority.context, &authority.fence)?;
        match result {
            None => Ok(Child::Absent),
            Some((handle, metadata)) if metadata.directory => {
                Ok(Child::Directory(WindowsDirectory {
                    node: directory_node(
                        handle,
                        metadata,
                        Some(parent.node.clone()),
                        name.into(),
                        operations,
                    )?,
                }))
            }
            Some(_) => Ok(Child::RegularFile),
        }
    }

    fn revalidate(
        &mut self,
        input: &CapturedInput,
        authority: &WindowsAuthority,
        operations: &mut Operations,
    ) -> Result<(), PortError> {
        self.current(input.context(), input.fence())?;
        if input.context() != &authority.context || input.fence() != &authority.fence {
            return Err(stale());
        }
        if operations.perform(|| ffi::drive_mapping(authority.drive))? != authority.mapping {
            return Err(stale());
        }
        // No-delete-sharing directory pins retain every ancestor binding while
        // allowing ordinary descendant writes. Check held identity/attributes/
        // case without enumerating or reopening a path from strings.
        let mut node = Some(authority.source_parent.clone());
        while let Some(current) = node {
            validate_directory(&current, operations)?;
            node = current.parent.clone();
        }
        let root = authority.root_chain.last().ok_or_else(stale)?;
        if let Some(parent) = &root.parent {
            let (_, observed) = self
                .open_child(parent, &root.name, Some(true), operations)?
                .ok_or_else(stale)?;
            if observed.identity != root.identity {
                return Err(stale());
            }
        }
        let held = observe(&authority.source, operations)?;
        if !fingerprint_matches(&held, &input.fence().source) {
            return Err(stale());
        }
        // A source handle permits delete sharing: old handle survival alone is
        // insufficient. Reopen the exact binding from its pinned trusted parent.
        let (_, bound) = self
            .open_child(
                &authority.source_parent,
                &authority.source_name,
                Some(false),
                operations,
            )?
            .ok_or_else(stale)?;
        if !fingerprint_matches(&bound, &input.fence().source) {
            return Err(stale());
        }
        if operations.perform(|| ffi::drive_mapping(authority.drive))? != authority.mapping {
            return Err(stale());
        }
        self.current(input.context(), input.fence())
    }
}

#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum TestStage {
    BeforeQuery,
    BeforeOpen,
    AfterOpen,
}

#[cfg(test)]
type TestHook = Box<dyn FnMut(TestStage, &str)>;
#[cfg(test)]
type AttributeFilter = Box<dyn FnMut(&str, &mut ffi::Basic)>;

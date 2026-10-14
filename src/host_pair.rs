//! Desktop pairing entry point; provider preparation requires explicit workspace attachment.

use crate::{
    error::ZenpiError,
    remote::{
        auth::{DeviceChallenge, DeviceRegistry},
        frame::{read_frame, write_frame},
        host_adapter::HostAdapter,
        listener::{ListenerOptions, RemoteListener},
        pairing::{HostIdentity, InviteIssuer, PairingInvite},
        protocol::{Channel, Envelope, ErrorCode, ExecutionState, Message},
    },
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use qrcode::{QrCode, types::Color as QrColor};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use std::{
    io::{self, IsTerminal, Write},
    net::{IpAddr, SocketAddr, UdpSocket},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPairOptions {
    pub help: bool,
    pub bind: SocketAddr,
    pub advertise: Option<IpAddr>,
    pub identity: PathBuf,
    pub duration_seconds: u64,
    pub plain: bool,
    pub desktop: bool,
    pub list_devices: bool,
    pub json: bool,
    pub list_projects: bool,
    pub revoke_device: Option<String>,
    pub workspace: Option<PathBuf>,
    pub session: Option<PathBuf>,
    pub grant_device: Option<String>,
    pub grant_projects: Vec<String>,
    pub grant_control: bool,
    pub grant_approval: bool,
    pub list_worktree_intents: bool,
    pub reconcile_worktree: Option<String>,
    pub list_quota_intents: bool,
    pub reconcile_quota: Option<String>,
    pub reconcile_device: Option<String>,
}

impl HostPairOptions {
    pub fn parse(args: impl Iterator<Item = String>) -> Result<Self, ZenpiError> {
        let args: Vec<_> = args.collect();
        let help = args.len() == 1 && matches!(args[0].as_str(), "--help" | "-h");
        let home = match std::env::var_os("HOME") {
            Some(home) => home,
            None if help => Default::default(),
            None => return Err(ZenpiError::arguments("HOME unavailable")),
        };
        let mut options = Self {
            help,
            bind: "0.0.0.0:0".parse().expect("bind"),
            advertise: None,
            identity: PathBuf::from(home).join(".zenpi/remote/identity.json"),
            duration_seconds: 300,
            plain: false,
            desktop: false,
            list_devices: false,
            json: false,
            list_projects: false,
            revoke_device: None,
            workspace: None,
            session: None,
            grant_device: None,
            grant_projects: vec![],
            grant_control: false,
            grant_approval: false,
            list_worktree_intents: false,
            reconcile_worktree: None,
            list_quota_intents: false,
            reconcile_quota: None,
            reconcile_device: None,
        };
        if help {
            return Ok(options);
        }
        let mut args = args.into_iter().peekable();
        while let Some(argument) = args.next() {
            if argument == "--plain" {
                options.plain = true;
                continue;
            }
            if argument == "--desktop" {
                options.desktop = true;
                continue;
            }
            if argument == "--list-devices" {
                options.list_devices = true;
                continue;
            }
            if argument == "--json" {
                options.json = true;
                continue;
            }
            if argument == "--list-projects" {
                options.list_projects = true;
                continue;
            }
            if argument == "--list-worktree-intents" {
                options.list_worktree_intents = true;
                continue;
            }
            if argument == "--list-quota-intents" {
                options.list_quota_intents = true;
                continue;
            }
            let value = args
                .next()
                .ok_or_else(|| ZenpiError::arguments(format!("{argument} requires a value")))?;
            match argument.as_str() {
                "--bind" => {
                    options.bind = value
                        .parse()
                        .map_err(|_| ZenpiError::arguments("--bind requires IP:PORT"))?
                }
                "--advertise" => {
                    options.advertise =
                        Some(value.parse().map_err(|_| {
                            ZenpiError::arguments("--advertise requires a literal IP")
                        })?)
                }
                "--identity" => options.identity = PathBuf::from(value),
                "--workspace" => options.workspace = Some(PathBuf::from(value)),
                "--session" => options.session = Some(PathBuf::from(value)),
                "--grant-device-read"
                | "--grant-device-control"
                | "--grant-device-approval"
                | "--grant-device-control-approval" => {
                    if options.grant_device.is_some() {
                        return Err(ZenpiError::arguments(
                            "only one device grant action is allowed",
                        ));
                    }
                    options.grant_control = matches!(
                        argument.as_str(),
                        "--grant-device-control" | "--grant-device-control-approval"
                    );
                    options.grant_approval = matches!(
                        argument.as_str(),
                        "--grant-device-approval" | "--grant-device-control-approval"
                    );
                    options.grant_device = Some(value);
                }
                "--project" => options.grant_projects.push(value),
                "--revoke-device" => options.revoke_device = Some(value),
                "--reconcile-worktree" => options.reconcile_worktree = Some(value),
                "--reconcile-quota" => options.reconcile_quota = Some(value),
                "--device" => options.reconcile_device = Some(value),
                "--duration-seconds" => {
                    options.duration_seconds = value
                        .parse()
                        .map_err(|_| ZenpiError::arguments("invalid duration"))?
                }
                _ => {
                    return Err(ZenpiError::arguments(format!(
                        "unknown host pair argument: {argument}"
                    )));
                }
            }
        }
        if !(1..=300).contains(&options.duration_seconds) {
            return Err(ZenpiError::arguments(
                "pairing duration must be 1..300 seconds",
            ));
        }
        if options.list_devices && options.revoke_device.is_some() {
            return Err(ZenpiError::arguments(
                "list and revoke are mutually exclusive",
            ));
        }
        if options.json && !options.list_devices {
            return Err(ZenpiError::arguments("--json requires --list-devices"));
        }
        if options.workspace.is_some() != options.session.is_some() {
            return Err(ZenpiError::arguments(
                "--workspace and --session must be supplied together",
            ));
        }
        let repair_actions = [
            options.list_worktree_intents,
            options.reconcile_worktree.is_some(),
            options.list_quota_intents,
            options.reconcile_quota.is_some(),
        ]
        .into_iter()
        .filter(|value| *value)
        .count();
        let repair = repair_actions > 0;
        let repair_request = options
            .reconcile_worktree
            .as_deref()
            .or(options.reconcile_quota.as_deref());
        if options.list_projects
            && (options.workspace.is_none()
                || options.desktop
                || options.plain
                || options.list_devices
                || options.grant_device.is_some()
                || options.revoke_device.is_some()
                || repair
                || options.reconcile_device.is_some())
        {
            return Err(ZenpiError::arguments(
                "--list-projects requires exclusive --workspace PATH --session PATH",
            ));
        }
        if repair
            && (options.workspace.is_none()
                || options.desktop
                || options.plain
                || options.list_devices
                || options.grant_device.is_some()
                || options.revoke_device.is_some()
                || repair_actions > 1)
            || repair_request.is_some() != options.reconcile_device.is_some()
            || repair_request.is_some_and(|id| !crate::remote::auth::valid_identifier(id))
            || options.reconcile_device.as_deref().is_some_and(|id| {
                id.len() != 64
                    || !id
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        {
            return Err(ZenpiError::arguments(
                "repair requires exclusive --workspace/--session, one intent action and --device DEVICE for reconciliation",
            ));
        }
        if options.desktop
            && (options.workspace.is_none()
                || options.plain
                || options.list_devices
                || options.revoke_device.is_some()
                || options.grant_device.is_some())
        {
            return Err(ZenpiError::arguments(
                "--desktop requires --workspace and --session, without --plain or device actions",
            ));
        }
        if options.grant_device.is_some() != !options.grant_projects.is_empty()
            || [
                options.list_devices,
                options.revoke_device.is_some(),
                options.grant_device.is_some(),
            ]
            .into_iter()
            .filter(|value| *value)
            .count()
                > 1
        {
            return Err(ZenpiError::arguments(
                "device grant/list/revoke policy conflict",
            ));
        }
        if options
            .revoke_device
            .as_deref()
            .or(options.grant_device.as_deref())
            .is_some_and(|id| {
                id.len() != 64
                    || !id
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
        {
            return Err(ZenpiError::arguments("device ID must be lowercase SHA-256"));
        }
        options.validate_addresses()?;
        Ok(options)
    }

    fn validate_addresses(&self) -> Result<(), ZenpiError> {
        // Port zero is allocated by the listener; all other invite constraints
        // can be checked before creating identity files or opening a socket.
        let mut bound = self.bind;
        bound.set_port(1);
        if !bound.ip().is_unspecified() && !crate::remote::pairing::valid_address(bound)
            || matches!(bound, SocketAddr::V6(value) if value.scope_id() != 0 || value.flowinfo() != 0)
        {
            return Err(ZenpiError::arguments("invalid pairing listener address"));
        }
        if let Some(ip) = self.advertise
            && (!crate::remote::pairing::valid_address(SocketAddr::new(ip, 1))
                || ip.is_ipv4() != bound.is_ipv4()
                || !bound.ip().is_unspecified() && ip != bound.ip())
        {
            return Err(ZenpiError::arguments("invalid pairing advertised address"));
        }
        Ok(())
    }
}

fn advertise_address(
    options: &HostPairOptions,
    bound: SocketAddr,
) -> Result<SocketAddr, ZenpiError> {
    let ip = match options.advertise {
        Some(ip) => ip,
        None if !bound.ip().is_unspecified() => bound.ip(),
        None => {
            let interfaces = crate::remote::lan::local_addresses()?;
            // A routing hint sends no packet. Failure must not hide an offline LAN.
            let route = (|| -> io::Result<IpAddr> {
                let socket = UdpSocket::bind(if bound.is_ipv4() {
                    "0.0.0.0:0"
                } else {
                    "[::]:0"
                })?;
                socket.connect(if bound.is_ipv4() {
                    "192.0.2.1:9"
                } else {
                    "[2001:db8::1]:9"
                })?;
                Ok(socket.local_addr()?.ip())
            })()
            .ok();
            crate::remote::lan::select(bound.is_ipv4(), &interfaces, route)
                .map_err(ZenpiError::arguments)?
        }
    };
    if ip.is_unspecified()
        || ip.is_multicast()
        || ip.is_ipv4() != bound.is_ipv4()
        || (!bound.ip().is_unspecified() && ip != bound.ip())
    {
        return Err(ZenpiError::arguments(
            "advertised IP must match listener interface/address family",
        ));
    }
    Ok(SocketAddr::new(ip, bound.port()))
}

pub fn run(options: HostPairOptions) -> Result<(), ZenpiError> {
    if options.help {
        println!("{}", help());
        return Ok(());
    }
    options.validate_addresses()?;
    if options.list_projects {
        let projects = local_projects(&options)?;
        println!(
            "{}",
            serde_json::to_string(&projects)
                .map_err(|error| ZenpiError::Message(error.to_string()))?
        );
        return Ok(());
    }
    if options.list_worktree_intents
        || options.reconcile_worktree.is_some()
        || options.list_quota_intents
        || options.reconcile_quota.is_some()
    {
        // Deliberately before identity/listener creation: local repair is not
        // a network operation or permission grant. Stop the host first.
        let workspace = options.workspace.as_ref().unwrap().canonicalize()?;
        let inspected =
            crate::session::SessionStore::open_existing(options.session.as_ref().unwrap())?;
        if std::path::Path::new(&inspected.header().cwd).canonicalize()? != workspace {
            return Err(ZenpiError::arguments(
                "repair session does not belong to workspace",
            ));
        }
        let owner = crate::core::Agent::prepare_project_with_options(
            options.session.as_ref().unwrap(),
            &workspace,
            Default::default(),
            false,
        )?;
        let mut pool =
            crate::project_workspace::ProjectOwnerPool::new(Arc::new(std::sync::Mutex::new(owner)))
                .map_err(ZenpiError::Message)?;
        pool.restore_checkpoint().map_err(ZenpiError::Message)?;
        let value = if let Some(request) = options.reconcile_worktree.as_deref() {
            pool.reconcile_worktree_intent(options.reconcile_device.as_deref().unwrap(), request)
                .map_err(ZenpiError::Message)?
        } else if let Some(request) = options.reconcile_quota.as_deref() {
            pool.reconcile_quota_intent(options.reconcile_device.as_deref().unwrap(), request)
                .map_err(ZenpiError::Message)?
        } else if options.list_quota_intents {
            serde_json::json!(pool.unresolved_quota_intents())
        } else {
            serde_json::json!(pool.unresolved_worktree_intents())
        };
        println!(
            "{}",
            serde_json::to_string(&value).map_err(|e| ZenpiError::Message(e.to_string()))?
        );
        return Ok(());
    }
    let identity = HostIdentity::load_or_create(&options.identity)
        .map_err(|error| ZenpiError::Message(error.to_string()))?;
    let registry = Arc::new(
        DeviceRegistry::new(
            options.identity.with_extension("devices.json"),
            identity.fingerprint(),
        )
        .map_err(|error| ZenpiError::Message(error.to_string()))?,
    );
    if let Some(id) = options.revoke_device.as_deref() {
        registry
            .revoke(id)
            .map_err(|error| ZenpiError::Message(error.to_string()))?;
        println!("Device revoked: {id}");
        return Ok(());
    }
    if options.list_devices {
        let devices = registry
            .list()
            .map_err(|error| ZenpiError::Message(error.to_string()))?;
        if options.json {
            let value = serde_json::json!({"devices":devices.iter().map(|device|
                serde_json::json!({"id":device.id,"name":device.name,"revoked":device.revoked,
                    "revision":device.revision,"capabilities":device.capabilities,"projects":device.projects}))
                .collect::<Vec<_>>()});
            println!(
                "{}",
                serde_json::to_string(&value)
                    .map_err(|error| ZenpiError::Message(error.to_string()))?
            );
            return Ok(());
        }
        for device in devices {
            println!(
                "{} {} revoked={} revision={}",
                device.id, device.name, device.revoked, device.revision
            );
        }
        return Ok(());
    }
    if let Some(id) = options.grant_device.as_deref() {
        let record = registry
            .lookup(id)
            .map_err(|error| ZenpiError::Message(error.to_string()))?;
        let mut capabilities = vec![crate::remote::auth::Capability::Read];
        if options.grant_control {
            capabilities.push(crate::remote::auth::Capability::Control);
        }
        if options.grant_approval {
            capabilities.push(crate::remote::auth::Capability::Approval);
        }
        registry
            .set_permissions(id, record.revision, capabilities, options.grant_projects)
            .map_err(|error| ZenpiError::Message(error.to_string()))?;
        println!(
            "{} permission granted: {id}",
            if options.grant_control && options.grant_approval {
                "Read/Control/Approval"
            } else if options.grant_approval {
                "Read/Approval"
            } else if options.grant_control {
                "Read/Control"
            } else {
                "Read"
            }
        );
        return Ok(());
    }
    if options.desktop {
        let owner = crate::core::Agent::prepare_project_with_options(
            options.session.as_ref().unwrap(),
            options.workspace.as_ref().unwrap(),
            Default::default(),
            false,
        )?;
        let mut attached = options.clone();
        attached.workspace = None;
        attached.session = None;
        return crate::tui::run_async_with_remote(owner, None, Some(attached));
    }
    let mut headless_host = None;
    let adapter = if let (Some(workspace), Some(session)) = (&options.workspace, &options.session) {
        let owner = crate::core::Agent::prepare_project_with_options(
            session,
            workspace,
            Default::default(),
            false,
        )?;
        let mut pool =
            crate::project_workspace::ProjectOwnerPool::new(Arc::new(std::sync::Mutex::new(owner)))
                .map_err(ZenpiError::Message)?;
        pool.restore_checkpoint().map_err(ZenpiError::Message)?;
        let (adapter, host) = crate::remote::headless_host::HeadlessHost::start(pool);
        headless_host = Some(host);
        Some(adapter)
    } else {
        None
    };
    let mut host = PairingHost::start_with_registry(&options, identity, registry, adapter)?;
    let ttl = Duration::from_secs(options.duration_seconds);
    if options.plain || !io::stdout().is_terminal() || !io::stdin().is_terminal() {
        println!(
            "{}",
            host.invite
                .to_uri()
                .map_err(|error| ZenpiError::Message(error.to_string()))?
        );
        println!(
            "Safety code: {}\nExpires: {}\n{}",
            host.invite.safety_code(),
            host.invite.expires_at,
            if headless_host.is_some() {
                "Host controls require a local device grant."
            } else {
                "Host writes unavailable."
            }
        );
        io::stdout().flush()?;
        std::thread::sleep(ttl);
    } else {
        pairing_tui(host.invite.clone(), &host.issuer, ttl, &host.accepted)?;
    }
    let result = host.shutdown();
    if let Some(host) = headless_host {
        host.shutdown();
    }
    result
}

pub fn help() -> &'static str {
    "Usage: zenpi host pair [OPTIONS]

Pair an Android device with this Rust host over TLS on the local network.
Scan the desktop QR, compare the six-digit safety code, then confirm on Android.
Pairing registers a device; project access must be granted separately.

Connection:
  --desktop                         Attach the existing desktop TUI and QR view
  --workspace PATH --session PATH   Attach a host-owned project/session (both required)
  --bind IP:PORT                    Local listener address (default 0.0.0.0:0)
  --advertise IP                    Literal reachable address in the QR (no DNS)
  --duration-seconds N              Invitation/listener duration, 1..300 (default 300)
  --identity PATH                   Private identity file (default ~/.zenpi/remote/identity.json)
  --plain                           Print the invitation URI instead of the QR view
  --list-projects                   List attached project IDs without a listener

Devices (exclusive local actions; use the same --identity PATH as the host):
  --list-devices [--json]            Inspect registered devices and grants
  --revoke-device ID                Revoke one device
  --grant-device-read ID            Grant read access to --project ID (repeatable)
  --grant-device-control ID         Grant read/control access to --project ID
  --grant-device-approval ID        Grant read/approval access to --project ID
  --grant-device-control-approval ID Grant read/control/approval access to --project ID

Recovery (requires --workspace PATH --session PATH, no listener):
  --list-worktree-intents           Inspect unresolved worktree operations
  --reconcile-worktree REQUEST_ID --device DEVICE_ID
  --list-quota-intents              Inspect unresolved quota operations
  --reconcile-quota REQUEST_ID --device DEVICE_ID

Examples:
  zenpi host pair --desktop --workspace /path/project --session /path/session.jsonl
  zenpi host pair --bind 192.168.50.14:7777 --workspace /path/project --session /path/session.jsonl
  zenpi host pair --grant-device-control DEVICE_ID --project PROJECT_ID

QR does not provide Internet/NAT traversal. Both devices must reach the same LAN.
Keep invitation tokens and identity keys private. Plain output contains a one-use token.
Normal TUI/headless runs do not start a listener. Help creates no files or sockets.
Use --help or -h alone after host pair to display this help."
}

/// Local inspection does not prepare an Agent or create identity/checkpoint state.
fn local_projects(options: &HostPairOptions) -> Result<serde_json::Value, ZenpiError> {
    use crate::project_workspace::ProjectWorkspace;
    let root = options
        .workspace
        .as_ref()
        .ok_or_else(|| ZenpiError::arguments("workspace required"))?
        .canonicalize()?;
    let session_input = options
        .session
        .as_ref()
        .ok_or_else(|| ZenpiError::arguments("session required"))?;
    let inspected = crate::session::SessionStore::open_existing(session_input)?;
    let session = session_input.canonicalize()?;
    if PathBuf::from(&inspected.header().cwd).canonicalize()? != root {
        return Err(ZenpiError::arguments(
            "inspection session does not belong to workspace",
        ));
    }
    let checkpoint = session
        .parent()
        .ok_or_else(|| ZenpiError::arguments("session parent required"))?
        .join("project-workspace.json");
    let workspace = match crate::project_workspace::read_owner_checkpoint(&checkpoint)
        .map_err(ZenpiError::Message)?
    {
        Some(bytes) => {
            let saved: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|error| ZenpiError::Message(error.to_string()))?;
            if saved["schema_version"].as_u64() != Some(1) {
                return Err(ZenpiError::arguments(
                    "unsupported project owner checkpoint",
                ));
            }
            let workspace = ProjectWorkspace::from_json_bytes(
                &serde_json::to_vec(&saved["workspace"])
                    .map_err(|error| ZenpiError::Message(error.to_string()))?,
            )
            .map_err(|error| ZenpiError::Message(error.to_string()))?;
            let mappings = saved["sessions"]
                .as_object()
                .ok_or_else(|| ZenpiError::arguments("invalid project session mapping"))?;
            if mappings.len() != workspace.tabs().len()
                || mappings.iter().any(|(id, value)| {
                    !workspace.tabs().iter().any(|tab| tab.id().as_str() == id)
                        || value.as_str().is_none_or(|path| {
                            !std::path::Path::new(path).is_absolute()
                                || path.len() > crate::project_workspace::MAX_PROJECT_PATH_BYTES
                                || path.chars().any(char::is_control)
                        })
                })
            {
                return Err(ZenpiError::arguments("invalid project session mapping"));
            }
            let Some(tab) = workspace.tabs().iter().find(|tab| tab.cwd() == root) else {
                return Err(ZenpiError::arguments(
                    "workspace absent from project owner checkpoint",
                ));
            };
            if saved["sessions"][tab.id().as_str()]
                .as_str()
                .map(PathBuf::from)
                .as_deref()
                != Some(session.as_path())
            {
                return Err(ZenpiError::arguments(
                    "session differs from project owner checkpoint",
                ));
            }
            workspace
        }
        None => {
            ProjectWorkspace::default()
                .with_directory(Some(&root), &root)
                .map_err(|error| ZenpiError::Message(error.to_string()))?
                .0
        }
    };
    Ok(
        serde_json::json!({"projects":workspace.tabs().iter().map(|tab|
        serde_json::json!({"id":tab.id().as_str(),"cwd":tab.cwd(),"active":workspace.active().is_some_and(|active|active.id()==tab.id())}))
        .collect::<Vec<_>>()}),
    )
}

/// An opt-in listener that may retain a mailbox to an already-running desktop owner.
pub struct PairingHost {
    listener: RemoteListener,
    issuer: Arc<InviteIssuer>,
    invite: PairingInvite,
    accepted: Arc<AtomicUsize>,
}
impl PairingHost {
    pub fn start(options: &HostPairOptions, adapter: Arc<HostAdapter>) -> Result<Self, ZenpiError> {
        options.validate_addresses()?;
        if options.workspace.is_some()
            || options.session.is_some()
            || options.list_devices
            || options.list_projects
            || options.revoke_device.is_some()
            || options.grant_device.is_some()
        {
            return Err(ZenpiError::arguments(
                "attached desktop listener cannot prepare another owner or manage devices",
            ));
        }
        let identity = HostIdentity::load_or_create(&options.identity)
            .map_err(|error| ZenpiError::Message(error.to_string()))?;
        let registry = Arc::new(
            DeviceRegistry::new(
                options.identity.with_extension("devices.json"),
                identity.fingerprint(),
            )
            .map_err(|error| ZenpiError::Message(error.to_string()))?,
        );
        Self::start_with_registry(options, identity, registry, Some(adapter))
    }

    fn start_with_registry(
        options: &HostPairOptions,
        identity: HostIdentity,
        registry: Arc<DeviceRegistry>,
        adapter: Option<Arc<HostAdapter>>,
    ) -> Result<Self, ZenpiError> {
        options.validate_addresses()?;
        let issuer = Arc::new(InviteIssuer::new(&identity));
        let accepted = Arc::new(AtomicUsize::new(0));
        let admission = issuer.clone();
        let count = accepted.clone();
        let host_fingerprint = identity.fingerprint();
        let mut policy = ListenerOptions::loopback();
        policy.bind = options.bind;
        let listener = RemoteListener::start(
            policy,
            identity
                .server_config()
                .map_err(|error| ZenpiError::Message(error.to_string()))?,
            move |stream| {
                let hello = read_frame(stream)
                    .map_err(io::Error::other)?
                    .ok_or_else(|| io::Error::other("EOF"))?;
                if !matches!(hello.payload, Message::Hello { .. }) {
                    return Err(io::Error::other("hello required"));
                }
                write_frame(
                    stream,
                    &Envelope::new(
                        Channel::Control,
                        Message::Welcome {
                            selected_version: 1,
                        },
                    ),
                )
                .map_err(io::Error::other)?;
                stream.sock.set_deadline(Duration::from_secs(30))?;
                let mut challenge =
                    DeviceChallenge::new(host_fingerprint.clone()).map_err(io::Error::other)?;
                write_frame(
                    stream,
                    &Envelope::new(
                        Channel::Control,
                        Message::Challenge {
                            nonce: challenge.nonce().into(),
                            host_fingerprint: host_fingerprint.clone(),
                        },
                    ),
                )
                .map_err(io::Error::other)?;
                let request = read_frame(stream)
                    .map_err(io::Error::other)?
                    .ok_or_else(|| io::Error::other("EOF"))?;
                if let Message::RevokeStatus {
                    device_id,
                    request_id,
                    signature,
                } = &request.payload
                {
                    let result = challenge.revoke_status(
                        &registry,
                        device_id,
                        request_id,
                        request.id.as_deref().unwrap_or(""),
                        signature,
                    );
                    let response = match result {
                        Ok(committed) => {
                            let mut response = Envelope::new(
                                Channel::Control,
                                Message::Response {
                                    result: serde_json::json!({"device_id":device_id, "request_id":request_id, "committed":committed}),
                                },
                            );
                            response.id = request.id.clone();
                            response.execution_state = Some(ExecutionState::Succeeded);
                            response
                        }
                        Err(error) => HostAdapter::failure(&request, error.remote_code()),
                    };
                    write_frame(stream, &response).map_err(io::Error::other)?;
                    return Ok(());
                }
                if let Message::Authenticate {
                    device_id,
                    signature,
                } = &request.payload
                {
                    let authentication = challenge.authenticate(&registry, device_id, signature);
                    let refusal = authentication
                        .as_ref()
                        .err()
                        .map(|error| error.remote_code());
                    let authenticated = authentication.ok();
                    let mut response = Envelope::new(
                        Channel::Control,
                        Message::Response {
                            result: serde_json::json!({
                                "authenticated": authenticated.is_some(), "device_id": authenticated.as_ref().map(|value| value.id()),
                                "device_revision": authenticated.as_ref().map(|value| value.revision()),
                                "control_available": false,
                            }),
                        },
                    );
                    response.id = request.id;
                    response.execution_state = Some(if authenticated.is_some() {
                        ExecutionState::Succeeded
                    } else {
                        ExecutionState::Failed
                    });
                    if authenticated.is_none() {
                        response.code = refusal;
                    }
                    write_frame(stream, &response).map_err(io::Error::other)?;
                    if let Some(device) = authenticated {
                        loop {
                            stream.sock.set_deadline(Duration::from_secs(30))?;
                            let Some(request) = read_frame(stream).map_err(io::Error::other)?
                            else {
                                return Ok(());
                            };
                            let request_id = request.id.clone();
                            let is_write = matches!(&request.payload, Message::Request { operation, .. } if operation.is_write());
                            if matches!(
                                &request.payload,
                                Message::Request {
                                    operation: crate::remote::protocol::Operation::DeviceRevokeSelf,
                                    ..
                                }
                            ) {
                                let response = registry.dispatch_self_revoke(&device, &request);
                                write_frame(stream, &response).map_err(io::Error::other)?;
                                return Ok(());
                            }
                            let Some(adapter) = adapter.as_ref() else {
                                return Ok(());
                            };
                            if matches!(request.payload, Message::Subscribe { .. }) {
                                stream.sock.set_deadline(Duration::from_secs(7))?;
                                crate::remote::subscription::serve_authorized(
                                    &request, adapter, &registry, &device, stream,
                                )?;
                                continue;
                            }
                            let response = adapter.dispatch(&registry, &device, request);
                            for frame in
                                HostAdapter::wire_frames(response).map_err(io::Error::other)?
                            {
                                let refused_code = match registry.is_current(&device) {
                                    Ok(true) => None,
                                    Ok(false) => Some(ErrorCode::Unauthorized),
                                    Err(error) => Some(error.remote_code()),
                                };
                                if let Some(code) = refused_code {
                                    let refused = HostAdapter::delivery_refusal(
                                        request_id.clone(),
                                        is_write,
                                        code,
                                    );
                                    write_frame(stream, &refused).map_err(io::Error::other)?;
                                    return Ok(());
                                }
                                write_frame(stream, &frame).map_err(io::Error::other)?;
                            }
                            if !registry.is_current(&device).unwrap_or(false) {
                                return Ok(());
                            }
                        }
                    }
                    return Ok(());
                }
                let Message::Pair {
                    token,
                    fingerprint,
                    safety_code,
                    device_name,
                    public_key,
                    signature,
                } = request.payload
                else {
                    return Err(io::Error::other("pair required"));
                };
                let registration = registry.verify_pair_and_register(
                    &token,
                    request.id.as_deref().unwrap_or(""),
                    &device_name,
                    &public_key,
                    &signature,
                    || {
                        admission
                            .consume(&token, &fingerprint, &safety_code)
                            .is_ok()
                    },
                );
                let refusal = registration.as_ref().err().map(|error| error.remote_code());
                let device = registration.ok();
                let permitted = device.is_some();
                let mut response = Envelope::new(
                    Channel::Control,
                    Message::Response {
                        result: serde_json::json!({
                            "paired": permitted, "control_available": false, "device_registered": permitted,
                            "device_id": device.as_ref().map(|record| &record.id),
                            "device_revision": device.as_ref().map(|record| record.revision)
                        }),
                    },
                );
                response.id = request.id;
                response.execution_state = Some(if permitted {
                    ExecutionState::Succeeded
                } else {
                    ExecutionState::Failed
                });
                if !permitted {
                    response.code = refusal;
                }
                if permitted {
                    count.fetch_add(1, Ordering::SeqCst);
                }
                write_frame(stream, &response).map_err(io::Error::other)
            },
        )?;
        let address = advertise_address(options, listener.local_addr())?;
        let ttl = Duration::from_secs(options.duration_seconds);
        let invite = issuer
            .issue(address, ttl)
            .map_err(|error| ZenpiError::Message(error.to_string()))?;
        Ok(Self {
            listener,
            issuer,
            invite,
            accepted,
        })
    }

    pub fn invite(&self) -> &PairingInvite {
        &self.invite
    }

    pub fn refresh(&mut self, ttl: Duration) -> Result<(), ZenpiError> {
        self.invite = self
            .issuer
            .issue(self.invite.address, ttl)
            .map_err(|error| ZenpiError::Message(error.to_string()))?;
        Ok(())
    }

    pub fn cancel_invite(&self) -> Result<(), ZenpiError> {
        self.issuer
            .cancel()
            .map_err(|error| ZenpiError::Message(error.to_string()))
    }

    pub fn render(&self, frame: &mut ratatui::Frame<'_>) -> Result<(), ZenpiError> {
        render_pairing(
            frame,
            &self.invite,
            self.accepted.load(Ordering::SeqCst),
            true,
        )
    }

    pub fn shutdown(&mut self) -> Result<(), ZenpiError> {
        self.issuer
            .cancel()
            .map_err(|error| ZenpiError::Message(error.to_string()))?;
        self.listener.shutdown()?;
        Ok(())
    }
}

/// Quiet zone and half-block pixels retain the QR's square module geometry.
pub fn qr_lines(invite: &PairingInvite) -> Result<Vec<Line<'static>>, ZenpiError> {
    qr_lines_at(
        invite,
        crate::remote::pairing::unix_seconds()
            .map_err(|error| ZenpiError::Message(error.to_string()))?,
    )
}

fn qr_lines_at(invite: &PairingInvite, now: u64) -> Result<Vec<Line<'static>>, ZenpiError> {
    // Clock validation belongs to this render's observation; do not race a
    // second wall-clock read while serializing an invitation at its last second.
    let uri = invite
        .to_uri_at(now)
        .map_err(|error| ZenpiError::Message(error.to_string()))?;
    let code = QrCode::with_error_correction_level(uri.as_bytes(), qrcode::EcLevel::M)
        .map_err(|error| ZenpiError::Message(error.to_string()))?;
    let width = code.width() + 8;
    let color = |x: usize, y: usize| {
        if x >= 4
            && y >= 4
            && x < code.width() + 4
            && y < code.width() + 4
            && code[(x - 4, y - 4)] == QrColor::Dark
        {
            Color::Black
        } else {
            Color::White
        }
    };
    Ok((0..width)
        .step_by(2)
        .map(|y| {
            Line::from(
                (0..width)
                    .map(|x| {
                        Span::styled("▀", Style::default().fg(color(x, y)).bg(color(x, y + 1)))
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect())
}

struct TerminalRestore;

/// QR half-blocks require black/white cell colors even when ordinary text uses NO_COLOR.
pub(crate) struct QrColorGuard(bool);
impl QrColorGuard {
    pub(crate) fn enter() -> Self {
        let disabled = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
        crossterm::style::force_color_output(true);
        Self(disabled)
    }
}
impl Drop for QrColorGuard {
    fn drop(&mut self) {
        crossterm::style::force_color_output(!self.0);
    }
}
impl Drop for TerminalRestore {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

fn pairing_tui(
    mut invite: PairingInvite,
    issuer: &InviteIssuer,
    ttl: Duration,
    accepted: &AtomicUsize,
) -> Result<(), ZenpiError> {
    terminal::enable_raw_mode()?;
    let _restore = TerminalRestore;
    let _qr_colors = QrColorGuard::enter();
    execute!(io::stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut deadline = Instant::now() + ttl;
    loop {
        if Instant::now() >= deadline {
            break;
        }
        let mut render_result = Ok(());
        terminal.draw(|frame| {
            render_result = render_pairing(frame, &invite, accepted.load(Ordering::SeqCst), false);
        })?;
        render_result?;
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => break,
                KeyCode::Char('r') => {
                    invite = issuer
                        .issue(invite.address, ttl)
                        .map_err(|error| ZenpiError::Message(error.to_string()))?;
                    deadline = Instant::now() + ttl;
                }
                _ => {}
            }
        }
        if Instant::now() >= deadline {
            break;
        }
    }
    Ok(())
}

fn render_pairing(
    frame: &mut ratatui::Frame<'_>,
    invite: &PairingInvite,
    accepted: usize,
    desktop: bool,
) -> Result<(), ZenpiError> {
    render_pairing_at(
        frame,
        invite,
        accepted,
        desktop,
        crate::remote::pairing::unix_seconds().ok(),
    )
}

fn render_pairing_at(
    frame: &mut ratatui::Frame<'_>,
    invite: &PairingInvite,
    accepted: usize,
    desktop: bool,
    now: Option<u64>,
) -> Result<(), ZenpiError> {
    let area = frame.area();
    let actions = if desktop {
        "R: refresh invite    Esc: desktop    F3: reopen pairing from desktop"
    } else {
        "R: refresh invite    Esc: cancel"
    };
    let Some(now) = now else {
        frame.render_widget(
            Paragraph::new(format!("Zenpi Pair\nHost clock unavailable\n{actions}"))
                .wrap(Wrap { trim: false }),
            area,
        );
        return Ok(());
    };
    if invite.expires_at <= now {
        frame.render_widget(
            Paragraph::new(format!(
                "Zenpi Pair\nInvitation expired\nAccepted: {accepted}\n{actions}"
            ))
            .wrap(Wrap { trim: false }),
            area,
        );
        return Ok(());
    }
    if invite.expires_at - now > crate::remote::pairing::MAX_INVITE_TTL.as_secs() {
        frame.render_widget(
            Paragraph::new(format!("Zenpi Pair\nHost clock changed\n{actions}"))
                .wrap(Wrap { trim: false }),
            area,
        );
        return Ok(());
    }
    let qr = qr_lines_at(invite, now)?;
    let width = qr[0].width() as u16;
    let grant = if desktop {
        "Project access and controls require a local device grant."
    } else {
        "Read permission requires local project grant. Host writes unavailable."
    };
    let metadata = vec![
        "Zenpi Pair".to_owned(),
        invite.address.to_string(),
        format!("Safety code: {}", invite.safety_code()),
    ];
    let footer = vec![
        "Host certificate SHA-256:".to_owned(),
        invite.fingerprint.clone(),
        format!(
            "Expires in {}s    Accepted: {}",
            invite.expires_at - now,
            accepted
        ),
        grant.to_owned(),
        actions.to_owned(),
    ];
    // Text wraps independently: wrapping QR spans would deform its module grid.
    let wrap = |lines: &[String], columns: usize| -> Vec<Line<'static>> {
        lines
            .iter()
            .flat_map(|line| {
                line.as_bytes().chunks(columns).map(|chunk| {
                    Line::from(String::from_utf8(chunk.to_vec()).expect("pairing labels are ASCII"))
                })
            })
            .collect()
    };
    let minimum_width = width.max(64) + 2;
    let columns = usize::from(area.width.max(minimum_width));
    let top = wrap(&metadata, columns);
    let bottom = wrap(&footer, columns);
    let minimum_height = qr.len() + top.len() + bottom.len();
    if area.width < minimum_width || usize::from(area.height) < minimum_height {
        frame.render_widget(
            Paragraph::new(format!(
                "Zenpi Pair\nResize terminal to at least {} x {}.\n{}",
                minimum_width, minimum_height, actions
            ))
            .wrap(Wrap { trim: false }),
            area,
        );
        return Ok(());
    }
    let mut lines = top;
    lines.extend(qr);
    lines.extend(bottom);
    frame.render_widget(Paragraph::new(lines), area);
    Ok(())
}

#[cfg(test)]
mod lan_selection_tests {
    use super::*;

    #[test]
    fn expired_or_unavailable_clock_renders_recovery_without_qr_or_desktop_failure() {
        let identity = HostIdentity::generate().unwrap();
        let issuer = InviteIssuer::new(&identity);
        let invite = issuer
            .issue(
                "192.168.50.55:7777".parse().unwrap(),
                Duration::from_secs(60),
            )
            .unwrap();
        for desktop in [false, true] {
            for now in [None, Some(invite.expires_at), Some(invite.expires_at + 1)] {
                let mut terminal =
                    Terminal::new(ratatui::backend::TestBackend::new(80, 12)).unwrap();
                terminal
                    .draw(|frame| render_pairing_at(frame, &invite, 2, desktop, now).unwrap())
                    .unwrap();
                let buffer = terminal.backend().buffer();
                let text = buffer
                    .content
                    .iter()
                    .map(|cell| cell.symbol())
                    .collect::<String>();
                assert!(text.contains(if now.is_none() {
                    "Host clock unavailable"
                } else {
                    "Invitation expired"
                }));
                assert!(text.contains("R: refresh invite"));
                assert!(text.contains(if desktop {
                    "Esc: desktop"
                } else {
                    "Esc: cancel"
                }));
                assert!(!text.contains(&invite.token));
                assert!(!buffer.content.iter().any(|cell| cell.symbol() == "▀"));
            }
        }
        let refreshed = issuer
            .issue(invite.address, Duration::from_secs(60))
            .unwrap();
        let mut clock_changed = Terminal::new(ratatui::backend::TestBackend::new(80, 12)).unwrap();
        clock_changed
            .draw(|frame| {
                render_pairing_at(
                    frame,
                    &invite,
                    0,
                    true,
                    Some(invite.expires_at - crate::remote::pairing::MAX_INVITE_TTL.as_secs() - 1),
                )
                .unwrap()
            })
            .unwrap();
        assert!(
            clock_changed
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains("Host clock changed")
        );
        assert!(
            !clock_changed
                .backend()
                .buffer()
                .content
                .iter()
                .any(|cell| cell.symbol() == "▀")
        );
        assert_ne!(refreshed.token, invite.token);
        assert!(
            issuer
                .consume(&invite.token, &invite.fingerprint, &invite.safety_code())
                .is_err()
        );
        assert!(
            issuer
                .consume(
                    &refreshed.token,
                    &refreshed.fingerprint,
                    &refreshed.safety_code()
                )
                .is_ok()
        );
    }

    #[test]
    fn complete_pairing_frame_decodes_at_minimum_and_wide_sizes_without_clipped_actions() {
        let identity = HostIdentity::generate().unwrap();
        let issuer = InviteIssuer::new(&identity);
        for address in [
            "192.168.50.55:7777",
            "[fd00:1234:5678:9abc:def0:1234:5678:9abc]:65535",
        ] {
            let invite = issuer
                .issue(address.parse().unwrap(), Duration::from_secs(60))
                .unwrap();
            let now = invite.expires_at - 1;
            let qr = qr_lines_at(&invite, now).unwrap();
            let minimum_width = (qr[0].width() as u16).max(64) + 2;
            for desktop in [false, true] {
                for columns in [minimum_width, 100, 140] {
                    let mut terminal =
                        Terminal::new(ratatui::backend::TestBackend::new(columns, 80)).unwrap();
                    terminal
                        .draw(|frame| {
                            render_pairing_at(frame, &invite, 0, desktop, Some(now)).unwrap()
                        })
                        .unwrap();
                    let buffer = terminal.backend().buffer();
                    let text = buffer
                        .content
                        .iter()
                        .map(|cell| cell.symbol())
                        .collect::<String>();
                    assert!(text.contains(&invite.fingerprint));
                    let flattened = text.split_whitespace().collect::<String>();
                    assert!(flattened.contains(if desktop {
                        "F3:reopenpairingfromdesktop"
                    } else {
                        "Hostwritesunavailable."
                    }));
                    assert!(flattened.contains("R:refreshinvite"));
                    assert!(text.contains("Expires in 1s"));
                    assert!(!text.contains(&invite.token));
                    let scale = 4usize;
                    let width = usize::from(columns) * scale;
                    let height = 160 * scale;
                    let mut pixels = vec![255u8; width * height];
                    for y in 0..80usize {
                        for x in 0..usize::from(columns) {
                            let cell = &buffer[(x as u16, y as u16)];
                            if cell.symbol() != "▀" {
                                continue;
                            }
                            for (half, color) in [cell.fg, cell.bg].into_iter().enumerate() {
                                for dy in 0..scale {
                                    for dx in 0..scale {
                                        pixels[((y * 2 + half) * scale + dy) * width
                                            + x * scale
                                            + dx] = if color == Color::Black { 0 } else { 255 };
                                    }
                                }
                            }
                        }
                    }
                    let mut decoder = quircs::Quirc::default();
                    let codes = decoder.identify(width, height, &pixels).collect::<Vec<_>>();
                    assert_eq!(codes.len(), 1);
                    let decoded = codes[0].as_ref().unwrap().decode().unwrap();
                    assert_eq!(
                        String::from_utf8(decoded.payload).unwrap(),
                        invite.to_uri_at(now).unwrap()
                    );
                    let used_rows = buffer
                        .content
                        .chunks(usize::from(columns))
                        .rposition(|row| row.iter().any(|cell| cell.symbol() != " "))
                        .unwrap()
                        + 1;
                    let mut exact = Terminal::new(ratatui::backend::TestBackend::new(
                        columns,
                        used_rows as u16,
                    ))
                    .unwrap();
                    exact
                        .draw(|frame| {
                            render_pairing_at(frame, &invite, 0, desktop, Some(now)).unwrap()
                        })
                        .unwrap();
                    assert!(
                        exact
                            .backend()
                            .buffer()
                            .content
                            .iter()
                            .any(|cell| cell.symbol() == "▀")
                    );
                    let mut short = Terminal::new(ratatui::backend::TestBackend::new(
                        columns,
                        used_rows as u16 - 1,
                    ))
                    .unwrap();
                    short
                        .draw(|frame| {
                            render_pairing_at(frame, &invite, 0, desktop, Some(now)).unwrap()
                        })
                        .unwrap();
                    assert!(
                        !short
                            .backend()
                            .buffer()
                            .content
                            .iter()
                            .any(|cell| cell.symbol() == "▀")
                    );
                    assert!(
                        short
                            .backend()
                            .buffer()
                            .content
                            .iter()
                            .map(|cell| cell.symbol())
                            .collect::<String>()
                            .contains("Resize terminal")
                    );
                }
            }
        }
    }

    #[test]
    fn desktop_pairing_render_has_grant_and_reopen_state_without_false_write_denial() {
        let identity = HostIdentity::generate().unwrap();
        let issuer = InviteIssuer::new(&identity);
        let invite = issuer
            .issue(
                "192.168.50.55:7777".parse().unwrap(),
                Duration::from_secs(60),
            )
            .unwrap();
        let mut terminal = Terminal::new(ratatui::backend::TestBackend::new(100, 70)).unwrap();
        for desktop in [true, false] {
            terminal
                .draw(|frame| render_pairing(frame, &invite, 1, desktop).unwrap())
                .unwrap();
            let buffer = terminal.backend().buffer();
            let text = buffer
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(text.contains("192.168.50.55:7777"));
            assert!(text.contains(&invite.safety_code()));
            assert!(text.contains(&invite.fingerprint));
            assert!(!text.contains(&invite.token));
            assert_eq!(text.contains("Host writes unavailable."), !desktop);
            assert_eq!(text.contains("F3: reopen pairing from desktop"), desktop);
            assert!(
                buffer
                    .content
                    .iter()
                    .any(|cell| cell.symbol() == "▀" && cell.fg == Color::Black)
            );
        }
        let mut narrow = Terminal::new(ratatui::backend::TestBackend::new(40, 10)).unwrap();
        narrow
            .draw(|frame| render_pairing(frame, &invite, 0, true).unwrap())
            .unwrap();
        let text = narrow
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.contains("Resize terminal"));
        assert!(!text.contains(&invite.token));
    }

    #[test]
    fn explicit_binding_and_advertisement_do_not_depend_on_interface_discovery() {
        for bound in ["127.0.0.1:7777", "192.168.50.55:7777", "[fd00::1234]:7777"] {
            let bound = bound.parse().unwrap();
            let options = HostPairOptions {
                bind: bound,
                ..HostPairOptions::parse(std::iter::empty()).unwrap()
            };
            assert_eq!(advertise_address(&options, bound).unwrap(), bound);
        }
        let bound = "0.0.0.0:7777".parse().unwrap();
        let options = HostPairOptions {
            advertise: Some("192.168.50.55".parse().unwrap()),
            ..HostPairOptions::parse(std::iter::empty()).unwrap()
        };
        assert_eq!(
            advertise_address(&options, bound).unwrap(),
            "192.168.50.55:7777".parse().unwrap()
        );
    }

    #[test]
    fn explicit_advertisement_cannot_cross_bind_family_or_exact_interface() {
        for (bound, advertise) in [
            ("127.0.0.1:7777", "192.168.50.55"),
            ("0.0.0.0:7777", "fd00::1"),
            ("[::]:7777", "192.168.50.55"),
            ("0.0.0.0:7777", "0.0.0.0"),
            ("0.0.0.0:7777", "224.0.0.1"),
        ] {
            let options = HostPairOptions {
                advertise: Some(advertise.parse().unwrap()),
                ..HostPairOptions::parse(std::iter::empty()).unwrap()
            };
            assert!(advertise_address(&options, bound.parse().unwrap()).is_err());
        }
    }
}

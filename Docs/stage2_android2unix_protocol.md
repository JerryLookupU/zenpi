# Android / Unix Remote Protocol v1

Status: protocol/frame primitives and TLS pairing transport implemented. Device
ACL, durable request deduplication and host dispatch remain unfinished items.
These codecs alone do not authorize execution or establish a secure connection.

## Compatibility

Android resource metadata browsing is independent of prompt control. Read-only
devices have a browse entry and may refresh/page a scoped catalog while connected;
an active prompt does not itself disable catalog reads. Shared ResourceReadPolicy
fences scanning/reset/loading/pending writes/revocation and inconsistent selected
project/session. Inserting a resource into the draft additionally requires
prompt control and no accepted/running prompt. Reload requires its separate
advertised control and existing exact catalog/CAS checks, not prompt control.
Metadata reads never send a task, reload resources or disclose template bodies.
Compose read-only navigation tests are compiled source evidence, not device runs.

Android new session_resume admission requires the current displayed catalog to
match the selected project/session and its unique authorized listing entry.
Loading/failed/absent catalogs, changed snapshot digests, absent/duplicate/current
targets cannot persist a new resume intent. The dialog shares the catalog scope
and resumable-target predicates, alongside scan/reset/connection fences. Exact
unresolved saved-envelope retry remains before new-command checks and does not
rebuild or retarget the request. The current Rust/Kotlin session lane checks the
actual host catalog mutation with unchanged active session and rejects the old
new-command admission, while retaining the exact pending retry path.

Android new resource reload admission rechecks the displayed catalog's project,
session, generation and snapshot digest, and refuses missing/loading/failed
catalog state before persisting a new intent. Exact unresolved saved-envelope
retry remains separate. Desktop production mailbox reload now delegates to
ProjectRuntimeHost, retaining pairing/active-job/queued-input/worker and modal
fences and visible resource catalog refresh. The cross-language source lane
pages 40 real loaded templates, reloads edited metadata, refuses stale local
reload races and validates restored receipts without exposing template bodies.
This is not connected Android, TLS/LAN or trusted extension execution evidence.

New Android model/reasoning commands are admitted only with a scoped,
registry-bound model status, a matching selection revision and no model read
in progress or failed. When the live projection also carries model status, its
revision and registry binding must agree with the displayed page. Reasoning
choices, including distinct null/default and explicit none, must still be
offered by both observed statuses. A manually entered model remains host-
validated, not limited to one catalog page. An unresolved saved command bypasses
these new-command checks only for its exact envelope under the existing
foreground/network/reset/revocation fences; it is never retargeted or rebuilt.
The model dialog uses the same selection/choice predicates as dispatch, so its
model field, apply actions and reasoning menu items are disabled while the live
projection contradicts the displayed status. Existing Compose tests include
revision/registry changes and choices removed without a revision change; they
remain connected-device gates, not JVM execution evidence.

Desktop model/reasoning writes now route through ProjectRuntimeHost.dispatch_remote
for both the production owner mailbox and the current Android command interop
lane. The owner adapter preserves admitted-project, worker-busy and modal fences,
the shared selection CAS/durable receipt namespace, and refreshes the visible
desktop model/catalog after success. Production-only pending-input/pairing/active
job fences are passed into that adapter rather than duplicated in another model
mutation branch. This is not proof of Android UI or physical LAN behavior.

Android network inventory is published only after all current network/capability/
link/default reads complete. A RuntimeException during either initial or later
callback observation retires the monitor generation, unregisters both callbacks
and publishes offline rather than crashing Main or retaining a usable route
token. Queued callbacks from that retired generation cannot restore availability.
The next Activity start admits a fresh monitor cycle; no partial route fallback,
automatic write retry or cellular/VPN fallback is introduced.

For non-loopback targets Android now observes all matching explicit LAN routes,
selects one forwarding network, binds only the unconnected socket and repeats
the complete observation before returning to the TCP connect caller. Changed
network/prefix/forwarding/destination/gateway/interface sets, ambiguous or missing routes, and failed reads or
binding refuse that attempt. Enumeration order and duplicate routes do not
cause false conflicts. No replacement-network retry or process-wide binding is
performed. This detects changes observed across binding, not an atomic freeze
of Android routing; later OS route changes still require callback cancellation,
TLS pinning and connected-device recovery evidence.

Connection-level hello/welcome/challenge envelopes contain exactly
schema_version, channel and payload. Pair/authenticate/revoke_status add only
their required request id. Project/session/turn routing, sequence, owner,
revision, outcome and error fields are forbidden, even when explicitly null.
Both Rust framed decode and Android FrameCodec enforce this before handshake
dispatch; direct Rust envelope validation also refuses non-null routing.
These fields cannot grant authority or be silently discarded before consuming
a one-use invitation or authenticating a device. Routed workspace requests and
their existing optional-null compatibility are unchanged.

The Android full-message dialog can revisit up to seven previously validated
8 KiB pages (56 KiB plus the current page) without another host read. Previous
navigation requires the same project/session/turn/role/digest/total size and
contiguous offsets; forwarding again refetches through the normal digest-bound
read rather than trusting an unbounded future-page cache. The history is
transient, never part of encrypted offline display, never an ACK or transcript
cursor advance. Close, background, authority loss, write and scope/turn
replacement clear it. Loading, errors and unresolved requests fence navigation.

Foreground Android read reconnect uses floored-jitter exponential backoff: failure
ceilings 1/2/4/8/16/32/60 seconds, capped at 60 seconds, uniformly sampled from
250 milliseconds through the ceiling. Immediate route/connect refusal cannot
form a zero-delay retry loop; the positive floor applies even after a successful
publication resets the failure count. Successful persisted subscription publication or
final attach resets failures. Healthy subscription renewal stays at two seconds;
pending replay pages use 200 ms only without a failure. Background/network/
generation cancellation interrupts the wait. This policy does not schedule
unresolved writes, re-pair tokens or revocation submissions; explicit exact
saved-request retry remains separate. WorkManager keeps its own bounded retry
policy and does not inherit this foreground loop.
The background worker's own 20-second operation expiry is a transport timeout
with at most two scheduled retries. Parent/WorkManager cancellation, including
a parent timeout, propagates unchanged and never requests a retry. Existing-key
load/read failures are authentication failures for the opportunity, not ordinary
network failures; a missing key skips it without creating one. Observed
cancellation before the encrypted-record CAS commit prevents that commit.
The background commit rechecks cancellation and signing-device identity after
acquiring the shared storage monitor, then again immediately before starting
AtomicFile publication after cache encoding/encryption. Waiting for foreground
storage cannot turn a cancelled worker into a later cache writer. The actual
non-suspending file commit remains atomic once admitted; cancellation is not
claimed to roll back a commit already in progress.
Foreground pairing saves, projection commits, pending workspace/revoke intent
and confirmation use the same in-lock/pre-publication admission. Their callback
checks the captured coroutine context and connection generation; successful
host receipts also recheck captured device identity. Revocation completion
checks again immediately before key removal, retaining its durable intent when
refused. This does not cancel an already admitted AtomicFile/key operation;
existing reset and pairing reconciliation handle that outcome separately.
Verified same-project desktop session changes also use this foreground failure
budget. They discard the old projection/cursor and reauthenticate for a fresh
snapshot within the same generation, not a recursively created refresh task.
Only a persisted successful publication resets the budget; an unconfirmed
second race cannot grant authority or restart immediate recovery.

Android captures its verified saved signing-device ID once per connection.
Challenge and revocation-status signing recheck that identity on both sides of
the signature operation. Missing/deleted/replaced keys or local signing failure
fence authority before proof transmission, while cancellation propagates as
cancellation. All receipt/device-owner/ACK checks use that captured connection
identity; no signature retry, key creation or mutation replay is introduced.

Android authentication success requires exactly a control response envelope
with no project/session/turn/owner/revision/error metadata and an allowlisted
result containing authenticated=true, the exact saved signing-device ID and a
boolean control_available. Booleans are not coerced from strings/numbers. Optional
device_revision is a positive lexical u64; legacy absence permits ordinary
reads but cannot authorize self-revoke. Unknown result fields and null/zero/
fractional/overflow revision refuse before workspace/ACK/command followup.
control_available (pair and authenticate) is the host's truthful report that the
device currently holds Control on a project this host has mounted (ZE-514); the
per-project authority Android acts on still comes from fresh host ACL views.

Android ViewModel host callbacks (subscription projection, saved command
confirmation and prompt-status listing) dispatch from network IO to the Main
owner before checking their connection generation or changing UI state.
Subscription and final attach check generation again after suspended encrypted
storage; replaced/background connections cannot publish, reconcile message
pages or close a newly opened view. Persist-before-publish remains in place.
Callback cancellation or storage failure propagates to the reader and stops
that exchange; moving callbacks to Main does not authorize a write, advance an
ACK or transfer filesystem/network IO onto the UI thread.

Android treats typed unauthorized/revoked_device/owner_mismatch responses as
loss of the retained control view, not mere transport loss. Foreground refresh
and transient read jobs fence other readers/subscriptions, strip grants/owner
epochs/approval and terminal/model/stream previews through the cache codec,
and retain only bounded durable display/cursors. They do not wipe signing keys,
infer a successful revocation, or silently resolve an unknown saved command.
Write refusals carry a typed code; generic similarly worded exceptions and
timeouts do not trigger this policy. Error-phase steer/cancel/approve dispatch
also rechecks the current projection's grant, scope, owner/revision and target
instead of accepting a previously constructed stale command. An exact saved
unknown request can still be explicitly retried; the host remains authoritative.
Explicit refresh can reauthenticate and establish a fresh control view.

Android pairing and authenticated host operations retain their 15-second
absolute deadline and independent socket-close watchdog. Their own coroutine
deadline is converted to SocketTimeoutException, so UI read jobs clear loading
with a fixed diagnostic, ordinary foreground reads enter bounded backoff, and
unconfirmed writes/revokes retain their saved request identity as unknown.
Parent timeout, lifecycle cancellation and explicit user cancellation still
propagate as cancellation; they are not converted into retryable failures.
The deadline helper also prevents a blocking IO result returned after expiry
from being published as confirmed. No timeout implies rollback or automatic
resubmission of a write; exact saved-request retry remains explicit. Pairing
timeout cannot prove the one-use invitation was unconsumed.

Android same-identity re-pair may update the stored endpoint only after the
new invite passes expiry/pinned TLS/device-proof exchange. The encrypted
metadata store preserves its display/cursor record only when both host
fingerprint and device ID match; it reuses the strict cache codec so grants,
owner epochs, running-state authority, live text and terminal output do not
survive. A different identity clears that display. Unresolved workspace or
revocation intent refuses every re-pair save. Endpoint changes rewrite the
encrypted record and invalidate background CAS stamps and old-endpoint
foreground commits. Invalid QR parsing never cancels a current connection or
replaces its host/display. Cancelling a new confirmation restores the prior
offline display and reconciles any pairing save already holding the storage
lock. No invitation token is persisted, and re-pair never changes host ACLs or
restores a revoked device.

`diff` is a project-Read operation, advertised per authorized project through
`diff_available`. Its exact arguments are `subtab_id`, unsigned
`subtab_revision` and a relative `path` (`.` reviews the selected tab).
The envelope requires exact project/session routing and no owner/revision or
turn mutation fields. Stale selection/revision, unauthorized or busy owners,
escaped paths and unavailable roots refuse; the host never falls back to main.
It validates the same stable-ID execution root used by prompt admission and
reuses the bounded local Git diff implementation with a one-second cancellation
budget. Whole-tab reviews stay inside the selected child directory/worktree,
including bounded ordinary untracked files. Binary-file and truncation flags
are explicit. Diff/status are bounded to 64/32 KiB and redacted; responses
contain no execution root. Git fsmonitor/hooks, global/system configuration and
inherited root/index/config overrides are disabled on controlled review runs.
No provider request, session append, checkpoint write, staging or apply occurs.
This is a point-in-time working-tree review, not a transactional filesystem
snapshot or diff application protocol. Files can change after review. The
current response is bounded control data, not paginated bulk transfer.

Remote Diff also reuses the attachment/listing known-private-path policy.
Credentials/auth JSON, key/PEM/P12 files, SSH key names and private directories
are refused case-insensitively at any workspace-relative component. Directory
reviews exclude these paths from both Git status and tracked patches before
reading content, and from synthetic untracked patches. Remote Git rename/copy
detection is disabled so excluded names do not enter rename headers. Deleted
private files are excluded too. Explicit filenames remain literal even with
inherited Git pathspec modes. Desktop-local Diff retains its previous full
review behavior. This is not arbitrary-secret classification: ordinary-named
copies, hard links and user-selected content require separate policy review.

Android uses a separate authenticated cancellable read, checks exact project/
session/subtab/revision/path and strict field/flag/byte bounds, and displays a
scrollable selectable patch with path refresh, close, binary and truncation
states. Diff is not stored in the encrypted transcript cache or sent by
WorkManager. Foreground loss, network change, selection/revision change,
mutation, re-pair, forget and revoke fence/cancel the transient review.
The Android codec refuses known private paths in both requests and returned
path scope; it does not parse patch text as a substitute for host-side filtering.
Older hosts without the capability do not expose the control.

The host layer-2 domain has an independent checkpoint schema version 2 in
`project-workspace.json` under `subtabs`, keyed by the stable project ID. Each
state contains a monotonic revision, ordered tabs and `active_id`. A tab ID is
SHA-256 over the versioned project/kind/absolute-root identity, not its display
name or index. Rename, move, selection and concurrency changes preserve IDs;
the main tab remains first. Legacy TUI indices migrate once, all-or-nothing,
through the same atomic compare-and-replace owner checkpoint. Subsequent
desktop starts restore the host state instead of overwriting it from an older
UI checkpoint. Session replacement and project publication preserve retained
domains; closing a project removes only that project's domain. Concurrent
host writes fail CAS without overwriting the winner.

This domain does not itself prove Git side-effect safety. The TUI still uses
view indices at its rendering/input boundary, translating changes to host IDs.
`subtab_list` is a project-Read operation returning redacted ID/name/kind/
concurrency, stable selection and project/subtab revision, never a filesystem
root. Read/Control clients may select, rename and reorder existing tabs through
`subtab_select`, `subtab_rename`, `subtab_move`. Arguments require `subtab_id`
and integer `subtab_revision`; rename adds `name`, move adds nullable
`before_id` (null appends). Main remains first. Writes additionally require
the envelope owner/session/workspace revision. Unknown arguments are refused.
The device ACL is held through commit; running/queued/preparing discussion or
arch owners refuse changes. Desktop-admitted work is also fenced. The same
128-entry workspace receipt ledger stores exact device/request/content identity
and both revisions atomically with the subtab change. Exact retries return the
original result, including after host restart, but still require current ACL.
No-op commands also commit one receipt and revision. A reused ID with changed
content fails; old project/subtab revisions fail before mutation.

Android reads this projection on live attach and exposes selection, rename and
ordering. It saves the immutable pending command before send and resolves it
through the existing explicit same-ID retry path. Offline cache v2 retains
only the secondary display/selection with false control flags and no workspace
owner/revision; strict v1 display records still load without secondary tabs.
Fresh authenticated attach replaces cached display and supplies current
authority. `worktree_control` admits the managed Git workflow described below;
`concurrency_control` admits idle changes and live-lane changes through the
controller described below. Full S2-018 acceptance remains unfinished. Planned legacy in-place roots need not exist, but existing
roots must be canonical; destructive operations need stricter boundary checks.

The shared Rust BackgroundRunner now supports an explicit 1..64 execution
quota with bounded FIFO pending work, cancellation by job ID and one shutdown
deadline across all active jobs. Its ordinary spawn remains single-concurrency.
A quota-change receipt resolves only after scheduler application, not command
channel admission. Raising fills available slots from the pending queue;
lowering preserves already-running work and prevents new starts until the live
count falls below the new value. Independent jobs must own independent Agents
and session writers. The low-level runtime supports live resizing, but the
independent worker-owner adapter captures leased capacity at startup. It now
supports host-only lowering and raising within that capacity, as described
below. Remote idle quota changes connect checkpoints to the next adapter
startup. Live controller changes additionally use application/commit settlement.

ProjectOwnerPool can now prepare a bounded independent WorkerOwner for a
host-owned subtab and zero-based slot below its configured quota. Worker
journal identity hashes the project, master session, stable subtab and slot;
rename/selection do not change it. Journals and lifetime-exclusive lease files
are under the master's `.zenpi/workers` directory, already excluded from remote
file/diff/attachment paths. The worker directory must be private and canonical;
journals/leases refuse symbolic and hard-link aliases. Each worker resolves
the selected root's normal configuration and receives its own Agent, input
port and approval coordinator, with explicit host auto-approval policy.
Returning a retained worker never clones the session writer. The lease is held
until the last worker handle drops, including after its pool drops. Session
reopen retains identity/history; normal owner preparation may append extension
selection evidence, never replay completed turns automatically.

Subtab mutation and project replacement fence busy worker owners; quota
reduction/subtab removal close retired owners only after durable checkpoint
commit. Failed commits leave workers live. Master new/resume refuses while
worker owners are retained; hosts can explicitly retire all idle workers
first. Existing external handles then point to closed Agents but retain their
leases until released. These host-domain primitives still do not advertise
remote worker dispatch, worker event/approval multiplexing or live quota
acknowledgements on their own. Later host adapters below supply remote routing
and idle quota changes; persisted active scheduler admission remains unfinished.

WorktreeRunner now captures the durable subtab quota and leases that many
independent worker owners for a real BackgroundRunner. Jobs acquire a free
slot only when the bounded scheduler starts them; returning or panicking
returns the slot. Its production spawn_prompts adapter runs bounded text
through the existing Agent input/tool pipeline in the captured selected root,
with cooperative cancellation and completion markers. Shell shorthand and
private attachment references are refused, not delegated to arbitrary shell
execution. This adapter is host-domain execution, not yet a remote request
admission endpoint or durable request-ID receipt mechanism.

A runtime ownership claim fences all retained slots, including idle ones:
subtab mutation, project replacement and explicit owner retirement cannot
close them while a scheduler could start queued work. Duplicate schedulers
on the same slots are refused. Failed multi-slot startup releases partial
claims. Claims live in the worker closure shared with each executing thread,
so a noncooperative job detached during shutdown retains its claim and journal
lease until its actual thread exits. Orderly shutdown releases claims and lets
the host change/reconstruct the quota. Live quota resizing/receipts remain
unfinished; later sections define remote worker status/approval/event routing.

Idle quota control uses `set_concurrency` with exactly `subtab_id`, integer
`subtab_revision` and integer `concurrency` in 1..64. It follows the same current
Read/Control ACL, exact session/workspace owner, workspace/subtab CAS and atomic
checkpoint/receipt rules as other subtab writes. `concurrency_control` is true
only with a durable checkpoint and current Control grant. Busy master/arch,
queued/preparing work, retained runtime-owned worker slots and desktop local
admission refuse the mutation with `busy`; refusal does not alter the checkpoint.
The succeeded six-field subtab receipt acknowledges committed configuration,
not application to a running scheduler. After the lane has really completed
and released its claims, the next WorktreeRunner uses the committed count and
that many independent worker owners. Restart retains configuration; retry of
an older successful request returns its original receipt without reverting a
newer count. Revocation still refuses that retry.

Android renders a selected-tab 1..64 stepper with host-confirmed values, saves
the exact pending envelope before send and uses explicit same-ID retry after
ambiguous delivery. New writes require connected foreground/network authority;
pending command/revocation disables the stepper; active workers alone no longer
disable it. The host's busy fence remains authoritative when another device's
tasks are not visible. Display cache excludes the grant. Live resizing needs
worker-claim reconciliation and scheduler-confirmed effective quota receipts;
idle controls do not satisfy that requirement.

WorktreeRunner now offers host-only `try_set_concurrency` for limits 1 through
its already leased `quota()`. The latter remains ownership capacity, not a
scheduler-applied value. `applied_concurrency()` reads the last scheduler-applied
limit, initially the spawn limit, and changes only on scheduler application
before resolving its receipt. A dropped receipt does not lose that observation.
The snapshot is not a live thread count, runtime liveness proof, durable
configuration or remote succeeded receipt. Event backpressure can delay both
application and receipt. Shutdown refuses unapplied quota commands.

Lowering keeps every retained worker claim and all running turns; queued starts
wait until the live count is below the new limit. Raising within leased capacity
fills pending FIFO starts using the same exclusive Agent/session writers.
Zero or limits above capacity are refused before command submission, rather
than admitting jobs without independent owners. Claims survive detached jobs
until actual exit even when the applied limit was lowered. The host's existing
idle mutation fence is unchanged. Runtime-only adjustment does not advance
workspace/subtab revisions or write a configuration receipt; restart uses the
durable captured quota, not the previous temporary runtime limit. Remote live
resize still needs durable intent, rollback/unknown recovery, newly claimed
slots for growth and synchronized admission scope; no Android live-resize
capability is advertised by this primitive.

Quota changes now persist a `quota_intent` in the same bounded 128-entry
workspace receipt ledger before the later configuration commit. It contains
the exact request and full before/candidate subtab states, validated against
the envelope digest, owner/project/session, strict integer quota and exact
one-revision transition on restore. Reservation alone changes no quota or
revision. Once reserved, a failed later commit returns null/unknown, never
failed/no-effect; authenticated same-ID retries remain unknown without
reapplying. Changed content conflicts, current ACL still applies and no worker
is prepared by receipt queries. Existing checkpoints without this optional
field remain readable. This is the durable boundary for future live scheduler
application, not an implemented live-resize remote endpoint.

After stopping the old host, a local operator can inspect with
`zenpi host pair --workspace PATH --session JOURNAL --list-quota-intents` and
explicitly commit with `--reconcile-quota REQUEST --device DEVICE`. These
exclusive repair modes run before identity/listener creation. Reconciliation
requires matching saved session, before-state and workspace revision, idle
master/arch and no retained runtime worker claim. It locks any existing
external slot leases through commit, so detached/other-host writers cannot
be mistaken for stopped work. It refuses symlink/nonprivate lease directories
or aliased lease files. It never starts workers or replays scheduler commands;
it explicitly commits requested configuration for future startup. The final
receipt preserves the original request owner across restart. Newer host state
refuses stale repair instead of being overwritten. Quota intents are listed
separately from unresolved Git intents.

Host-only WorktreeRunner growth can now prepare slots against an unresolved
quota intent. It validates exact device/request/digest, current before-state,
master session, workspace/subtab revision and old configured capacity before
preparing any new owner. New writers use the intent's host-validated candidate
root and normal private journal/lease policy, never client paths. Master/arch
must be idle, and all new slots count against the existing 64-owner pool cap.
Every writer must acquire an exclusive runtime claim. Handles are published
only after the entire preparation succeeds; failure drops staged claims/leases
and leaves the original runner capacity/applied limit unchanged. Preparation
may leave private initialized journals, but never admits a turn or deletes them
as rollback. Retry after removing the preparation blocker reopens that history.

The runner's slot inventory is mutex-protected during growth, while jobs retain
only their own worker and return their slot on success/error/panic. All claims
still outlive a noncooperative detached job. Growing capacity does not change
the scheduler-applied limit, durable configuration or captured admission scope;
an explicit host runtime quota command is still required to start additional
queued jobs. Missing/foreign/stale/settled intent refuses preparation. This
primitive alone does not relax the remote idle-mutation fence. The later
controller combines it with live application/commit settlement and synchronized
admission scope before returning a success receipt.

Host-only `begin_reserved_quota` and `poll_reserved_quota` now join application
to configuration settlement for a captured WorktreeRunner. Begin validates the
unresolved exact intent, prepares ownership if needed, submits the scheduler
command and fences new task/direct quota/growth commands. Poll consumes only
the application receipt, not worker events, checks the last applied value and
commits the exact intent with matching runtime-owned slots. Ordinary subtab
mutations still refuse all runtime-owned workers; this exception is limited to
the validated quota-only intent and that runner's identical claimed handles.
Lowering retains excess running owners/claims until runtime shutdown rather
than closing executing writers. Foreign runtime owners remain busy fences.

Success atomically persists configuration and its original-owner six-field
receipt, then updates the runner's configured quota and subtab admission
revision. Ownership capacity remains separate; raising within retained capacity
does not recreate writers, and later growth starts beyond retained capacity.
Worker admission compares configured quota, not retained capacity. Progress
tracking permits a monotonic revision advance only for the same instance,
project, session and subtab; original tracked requests keep their identity.
An unapplied/backpressured command cannot commit. Checkpoint failure after
application retains the unknown intent, old configured scope and submission
fence; shutdown followed by explicit local recovery remains available. A second
runtime command cannot overwrite an unresolved application. This host-domain
API is used by the live remote controller described next.

Desktop ProjectRuntimeHost and HeadlessHost now route `set_concurrency` for an
existing live worktree lane through WorktreeHost. New requests validate exact
arguments, current Read/Control ACL under the registry revoke lock, workspace
owner/session and both revisions, idle master/arch and no already pending quota.
The controller reserves intent, prepares growth and submits exactly one runtime
command. Immediate or later same-ID explicit retry polls that original runtime
receipt and commits only under current ACL and the same lock. Until confirmed,
it returns `unknown_outcome` with null result, not succeeded or a fictitious
applied count. The committed six-field receipt proves application plus config
for a live lane; ordinary idle receipt remains configuration-only.

Background worker event polling does not settle quota, because it has no current
requesting-device revoke lock. Revocation after runtime submission cannot undo
already authorized scheduling effects, but refuses remote commit/retry and
leaves the intent and new-task fence for explicit local recovery after shutdown.
Pending quota keeps its lane alive even when all tasks finish, preventing
receipt loss through automatic idle retirement. Shutdown retains the durable
unknown reservation; restart does not reconstruct or resubmit its command.
Changed ID content conflicts, another quota request is busy and no new task
is admitted until settlement. Cancel/approval remain reachable. Once committed,
the same runner accepts fresh prompts with the new configured revision while
old admitted prompts retain their exact receipts. Android enables the existing
stepper during active workers and uses the existing persisted pending-envelope/
explicit-retry flow; counts remain host-confirmed and cache strips authority.
No automatic quota retry or success inferred from a displayed count is added.

Controller lifecycle regression now exercises completion before explicit quota
confirmation: finished worker tasks leave the pending lane/intent intact, exact
retry settles under current authority, and the next poll releases claims and
retires the lane. Its durable receipt then survives restart without preparing
workers. A separate remote lowering fixture holds three independent Agent
writers with two queued tasks, commits a limit of one while they are active,
and proves the queue does not fill freed slots until the live count drops
below one. Every original task reaches succeeded without cancellation and
normal retirement releases all retained owners. Android fragmented-response
validation refuses unknown/accepted/running outcomes even when the result
looks like a valid configuration receipt; only succeeded can confirm the
saved quota command. These are host/JVM regression fixtures, not device or LAN
acceptance evidence.

Live quota commits no longer lock unchanged sibling worktree Agents. The quota
intent must still match the entire current before-state, and the target runner
must supply identical runtime-owned handles. A sibling may continue only when
its stable ID, root/kind and full before/candidate tab are unchanged and its
master session matches. Other target/changed/foreign-session ownership remains
fenced. No executing sibling writer is closed or retired by this exception.

Because subtab revisions are project-wide, an unchanged sibling runner refreshes
its captured admission revision before fresh prompt/quota admission. Refresh
requires the same current master session, existing stable ID, canonical valid
captured execution root/kind, unchanged configured quota and monotonic revision.
Pending quota intents cannot refresh their owned revision. Changed execution,
changed quota, removed tab, session change or revision regression refuse refresh.
Progress retains the same runtime instance and original request/device identity;
old exact receipts remain queryable. This allows independent live worktrees to
continue after sibling commits without ignoring the caller's revision CAS.

New quota identities are serialized while any durable quota intent remains
unresolved. The reservation owns a global workspace revision, not merely one
lane or project. Both live-lane and idle-subtab adapters return Busy before
reserving another identity or preparing worker capacity; exact receipt queries
and request-ID conflicts retain their existing precedence. The pool enforces
the same reservation guard independently of the adapters. A restart, device
revocation or task completion does not release an unresolved reservation.
Only its original authorized settlement or explicit stopped-host local repair
allows another quota request using fresh revisions. This does not suspend
unrelated sibling task execution or turn an unknown outcome into a failure.

Live quota growth preflights the missing ownership range against the host-wide
64-owner bound before reserving an intent. Known capacity exhaustion returns
ResourceLimit without a receipt, journal preparation or scheduler change, so a
caller can retry after other idle owners retire. Initial runner construction
also preflights its whole range rather than leaving a partially prepared lane
when only some slots fit. Existing stable slots do not consume capacity twice.
Preparation-time lease/filesystem failures retain their separate ambiguity
handling; the preflight is not proof that subsequent preparation will succeed.

TranscriptRead also accepts exactly request_id/offset/digest for a completed
worker task, instead of the master turn_id/offset/digest form. The host resolves
only a succeeded durable progress event belonging to the current authenticated
device/project/master session; an internal output_turn_id correlates its final
assistant turn and is not exposed in worker status projections. Journal paths
are derived from host project/session/subtab/slot keys, never client paths.
Reads do not prepare an Agent, acquire an execution lease or create directories.
Canonical private user-owned unaliased journals are required; scanning is
bounded to 64 keys and 8 MiB aggregate journal bytes, with changed-file checks.
Missing legacy correlation fails closed, not by guessing the latest answer.

The page includes request_id/subtab_id plus the ordinary actual turn ID, digest,
role, byte offsets, size and text. Whole-content redaction precedes 8 KiB UTF-8
page slicing; 4 MiB content and NUL refusal still apply. Android's completed-task
output button opens this separate request-scoped route, retaining bounded
previous pages and digest continuity. Pending/running/unknown results cannot
authorize a worker output page. Project/session/task changes, authority loss
and lifecycle suspension invalidate the open read. This adds final assistant
output paging, not full worker user/tool/event history or cursor replay parity.

Worker output correlation now requires matching durable evidence in both
journals. After actual observed worker execution, its journal appends one
remote_worker_output_v1 binding with original device/request/full-envelope
fingerprint, project/master session/subtab, worker session/derived slot key,
final assistant turn and raw-content SHA256. The master progress event stores
the same request fingerprint and output SHA256 internally. Reads require one
matching binding and one assistant turn, checking every scope field, both
content digests and the derived slot key; missing, duplicated or mismatched
evidence fails closed. Internal fingerprints/digests never enter Android status
projections. An append failure does not claim successful output correlation.

The worker reader now passes its 8 MiB bound into SessionStore's actual
descriptor read, including the metadata probe and max+1 read limit, rather
than relying only on a preceding filesystem size check. General existing
session readers keep their previous cap. This guards accidental mixed journals
or changed worker content, not a malicious local operator able to rewrite both
the host master and worker journals; device ACL and filesystem trust remain
separate boundaries.

Successful observed worker bindings additionally contain their admitted input
turn and an ordered manifest of newly appended user/tool/intermediate/final
turn IDs, roles and raw-content hashes. The master success record independently
retains the input turn. TranscriptRead's request_id/view=history/offset/digest
form validates this contiguous range from that input through the correlated
final assistant turn before projecting at most 16 messages per page. Only the
originating device's task range is exposed, never the rest of a reused worker
session. Each preview is redacted then UTF-8 sliced to 2048 bytes; history
continuations bind a digest and total count. At most 1024 turns per task and
the existing journal/content limits apply.

The request_id/turn_id/offset/digest form reads a full message only when that
turn is in the validated task manifest, retaining byte-page bounds. Android's
task-history dialog shows message roles, allows seven bounded previous pages,
and opens individual messages through the same task-scoped full-page route.
History reads and selected-message reads are invalidated on scope/task loss,
authority loss or lifecycle suspension, and pending mutations disable controls.
Legacy tasks without a manifest, failed/cancelled partial execution, and worker
event/cursor replay are not inferred as complete history.

The history manifest SHA256 is now carried from the observed runtime result
into the master success event, independently of the worker journal. History
reads require both digests to match the current manifest, so editing only the
worker's manifest and recomputing its digest cannot expand the authorized task
range. Every non-input message in that range must also have the original input
as its parent. This internal runtime evidence is absent from ordinary model
results and filtered from wire status projections. Deterministic local backend
regression now exercises the actual read_file tool loop and verifies user,
assistant-tool-call, tool-result and final-assistant history plus redaction;
it is not provider HTTP or LAN evidence.

Android task-history navigation now validates the retained page stack as one
task/subtab/session/digest/count with contiguous offsets, rejects duplicate
message IDs across cached pages, and keeps at most seven prior pages. Request
continuations also check the current task's subtab rather than only project and
session. Individual selected-message continuations must retain actual turn ID
and role. Opening full message text preserves the validated history page,
previous-page stack and scroll position while hiding its dialog; dismissing
the message returns to that page only if current scope and selected membership
still match. Otherwise both views are cleared. Broad lifecycle/scope/authority
invalidation continues to clear all retained read content; it is distinct from
an ordinary message dismissal. Hidden history cannot advance behind an open
message dialog, and cached content remains memory-only.

New observed worker success evidence now also carries its exact host-owned
slot number and actual worker session into the master journal. TranscriptRead
derives just that slot's journal key, checks its session and matching worker
binding, and reads only that journal under the 8 MiB descriptor limit. This
supersedes the earlier bounded multi-slot scan: an unrelated oversized or busy
sibling journal no longer prevents reading the selected completed task.
Slot values must be integers in 0..63; missing/foreign slot/session evidence
fails closed without fallback search. These internal fields are not Android
status fields and do not permit client-provided paths or slot selection.

The desktop local /cancel command and ordinary interrupt action now address
live worktree jobs in the selected project/subtab through the same WorktreeHost
runner controller. This is explicit local host authority, not a fabricated
AuthenticatedDevice or a remotely callable ACL bypass. It cancels all tracked
nonterminal jobs in that subtab across originating devices, including queued
jobs, but never a sibling subtab/project. The displayed root/kind must still
match host selection; an uncommitted/stale local view fails closed.

Nonblocking cancellation reports how many requests entered the runtime command
queue and how many were deferred. It does not rewrite admission receipts,
emit synthetic cancelled progress, release worker claims or erase drafts.
Regular polling persists actual terminal outcomes for the original phones;
noncooperative workers retain ownership until exit and completed results can
win a late cancel race. Other desktop submissions remain fenced while worktree
tasks run. Force-kill/shutdown behavior is not changed by this entry point.

Desktop /status now supplements the existing master Agent status with at most
32 current-subtab worktree task identities and phases across phones. Identity
is device plus request ID, never request ID alone. Live controller evidence takes
precedence over durable events, with runtime-active entries retained first.
The runtime-active marker is separate from the persisted outcome: ambiguous
progress persistence can be unknown_outcome while its execution still owns a
worker. A restored running/admission event without that controller is always
unknown_outcome and never proof of a live or successful task.

Status reads validate the same selected root/kind as local cancellation, use
only the host master journal and tracked lane, never reconstruct a worker, and
do not publish local status messages into session journals. Master contention
reports unavailable rather than inventing an empty or terminal view. The local
projection includes only device/request/state/runtime-active fields, not prompt
text, output, provider diagnostics, execution evidence or worker paths. Remote
phone status remains device-scoped and unchanged. An idle tab without task
evidence retains the ordinary /status route.

Phone worker status now optionally includes runtime_active=true, derived only
from a currently tracked original runtime job whose terminal event has not
arrived. It is not copied from durable journal fields, stored in offline cache,
or treated as completion evidence. In particular unknown_outcome can coexist
with this flag after a progress append failure. Android decodes the boolean
strictly and only for accepted/running/unknown_outcome, shows the active runtime
state, and enables cancellation of that unknown task under current Control,
owner/session/revision and foreground authority. Unknown records without the
flag never gain cancellation controls.

The host cancellation router resolves the original device/request/runtime job
even when its persistence outcome is terminal-unknown, provided the runtime is
still active. Existing authorization and durable cancel-receipt deduplication
apply unchanged; no original prompt is resubmitted. After the real terminal
event the flag disappears and new cancels cannot target the retired job. Restart
does not infer runtime evidence from any stored runtime_active field. Offline
cache and authority loss drop all worker statuses as before.

Tracked phone task admission is now visible as accepted plus runtime_active
before the first runtime-event drain. This reflects an actually submitted,
tracked job, not a inferred durable running record. Projection remains read-only
and does not persist this synthesized display state. Within each lane, retained
statuses put runtime-active jobs first in submission order, followed by recent
terminal entries. The aggregate project list also puts live entries ahead of
terminal entries from earlier sibling lanes before applying its 16-entry cap.
Deduplication still prefers tracked state over durable state and remains scoped
to the authenticated device and current master session. More than 16 live tasks
still require a future paged status view; priority is not full-list coverage.

New task manifests also bind each turn's metadata SHA256, carried inside the
master-confirmed manifest digest. History and selected-message reads reject
metadata replacement even if message text and IDs are unchanged. Older
manifests without metadata hashes fail closed for history; no inferred upgrade
or journal rewrite is performed.

Assistant tool-call history entries may include exactly tool_calls and
tool_calls_truncated alongside the existing four message-preview fields.
Each call exposes only id, name, arguments_preview and arguments_truncated:
at most eight calls, 128-byte IDs, 64-byte names, and 512-byte UTF-8 argument
previews. Structured arguments are redacted before serialization and slicing;
arguments over 64 KiB are refused. Execution evidence, policy digests, worker
bindings, annotations and model metadata are not projected. Pages also reserve
control-envelope space and stop before the encoded JSON exceeds the control
frame budget, including escaping expansion. Android validates the exact shape,
bounds, assistant role and unique call IDs, and shows tool names and selectable
monospace argument previews even when the assistant's text is empty. This is a
bounded preview, not full argument paging or live tool-event replay.

TranscriptRead additionally accepts exactly request_id/turn_id/tool_call_id/
offset/digest for full arguments of one completed task invocation. This route
requires the same master-confirmed task manifest, metadata hashes, current Read
grant, originating device, worker slot/session and selected message membership.
The selected turn must be assistant, the call ID must have one matching entry,
and the tool name must satisfy the preview bounds. Only the arguments value is
serialized after structured redaction; neither raw metadata nor execution
evidence is returned. Both raw encoded arguments and the redacted serialized
document are capped at 64 KiB. Byte pages are at most 8192 bytes and remain
UTF-8 aligned, with digest-fenced continuation. Responses add tool_call_id and
tool_name to the existing task message page shape; callers cannot select a
worker path or read another device's invocation.

Android opens arguments from the retained history call with an OpenInFull
action and reuses the bounded message paging surface. Requests, decoding and
previous-page navigation bind task, turn, call ID and tool name, so argument
pages cannot become ordinary message pages or another call's pages. Dismissal
clears the selected call and returns only to a still-valid history selection;
scope/authority/lifecycle invalidation clears both. Pending mutation and offline
states disable argument reads and paging. The text is selectable monospace.
This supersedes the earlier full-argument-paging frontier for calls visible in
the eight-call preview; it does not add paging of omitted calls or live events.

The host-domain worker_admission adapter now accepts Prompt with exact
text/subtab_id/subtab_revision arguments, an already-created WorktreeRunner
and the ordinary owner/workspace revision envelope. It validates current
device Control plus Read grants, master/session/runner scope and captured
subtab revision/quota. It binds the device/request/full-envelope digest in
the existing workspace checkpoint request namespace before appending a
reserved admission event to the master journal or submitting text to the
runner. Worker execution journals remain independent. Accepted/failed
admission receipts are appended afterward; any ambiguous checkpoint or journal
write returns UnknownOutcome, never permission to automatically execute again.
Exact retries read the original admission receipt before current nonce/revision
checks, but still require current ACL. Different content or reuse by a master
prompt fails RequestIdConflict. Restart imports worker admission bindings from
the master journal alongside existing prompt/model bindings. A checkpoint-only
reservation remains unknown and is never resubmitted.

This adapter is not yet installed in the production HostAdapter/TUI/headless
mailbox. Ordinary Prompt continues its existing master path and rejects the
new worker arguments there. An Accepted worker receipt means admission, not
execution success: bounded scheduler rejection, execution progress, completed
output and worker approvals still require status/receipt routing. No Android
client may advertise this as an enabled worker endpoint until that routing and
its acceptance gates are implemented.

Tracked worker admission now registers job/request/device identity immediately
after successful runtime submission, even if the later accepted journal append
is ambiguous. WorkerProgress belongs to one runner generation (not just a
project or tab; job counters can restart). It retains at most 128 requests,
refuses new tracked submission when full before binding or submission, and
still allows exact receipt queries. Each runtime Accepted/Queued/Started,
Rejected or Completed event updates that request's execution projection.
Failure uses typed codes only, never raw Agent errors, provider output or
prompt text. Admission Accepted is immutable and separate from later execution
Failed when the bounded pending queue rejects the job.

Progress is appended to the master journal as remote_worker_progress_v1.
Owner lock contention retains the latest confirmed value, including terminal
completion, for a later flush. An append error is ambiguous: live status becomes
UnknownOutcome, is retained within the cap and is never blindly appended again.
Terminal publication occurs once; delayed Started events cannot overwrite it.
Listing retains a bounded 16-entry device/project/session/subtab projection.
Live tracker values override durable values; after restart an unconfirmed
Accepted/Running record is shown unknown, not inferred to still run. Confirmed
success/failure/cancellation survive restart. Shutdown's Cancelled is unknown
because noncooperative execution may have detached. The current tracker remains
a host-domain API, not yet production network/UI status routing or approval/
transcript multiplexing; Android worker controls remain disabled.

WorkerProgress::cancel now maps an exact original request ID to a tracked
runtime JobId and reuses the existing durable CancelAdmissions path. New
cancellation requires current device Control grant, owner/session/workspace
revision, the original submitting device and the same runner generation.
Terminal, untracked and replacement-runner targets are refused before a
checkpoint side effect. Request strings are device-scoped: when two devices
use the same string, the caller's own request is selected rather than the first
matching foreign request. Queued and running jobs use the existing job-ID
cancellation channel, never every worker's approval emergency-cancel hook.
Independent slot jobs remain unaffected; approval waits already poll their
own cancellation predicate. The cancellation checkpoint reserves unknown
before sending the command and stores accepted/failed afterward. Exact receipt
queries still work after completion/restart without cancelling a new job;
ambiguous receipts never automatically send another command. This remains a
host-domain control API pending production network/TUI/headless routing.

Worker approvals now have a host-domain projection and dispatch adapter over
the existing one-shot ApprovalCoordinator. The bounded combined list includes
master project/session routing plus worker_session_id and stable subtab_id;
it never drains approvals, locks an executing Agent or sends raw tool arguments.
The authenticated listing wrapper requires a read-only Inspect envelope,
current device Read ACL and exact project/master-session identity. The plain
worker_listing helper is local projection only and must not bypass that ACL.

dispatch_worker accepts exact request_id/digest/decision/worker_session_id
arguments, with the normal master-session owner/workspace CAS envelope. It
uses the current project's retained worker control to select the coordinator
before the atomic digest claim. Current Approval ACL is required; Control
alone cannot answer. Existing durable approval request identity reserves
unknown before response and settles afterward; same-ID different worker
session/content fails conflict. Remote actor audit uses the actual worker
session ID while routing/receipt stays master-session scoped. No remembered
grant or arbitrary message is accepted. Same approval request strings in
different worker sessions remain separate. Master approval behavior is
unchanged. Exact worker receipts survive completion/restart and never prepare
or answer replacement workers; revoked-device queries are refused.
Worker owners also retain their originating master-session identity: after a
master-session replacement, stale workers are excluded from listing and new
approval dispatch even when their independent worker coordinator still has a
pending request. A failed approval checkpoint reservation leaves every
coordinator unanswered and returns UnknownOutcome.

HeadlessHost now owns lazy WorktreeHost lanes keyed by project and stable
subtab. Prompt with worker subtab arguments uses durable worker admission;
ordinary text-only Prompt remains on the master path. Current Read/Control,
owner/session/revision, safe text and durable request reservation are checked
before creating a runner or worker journal. Exact receipts and checkpoint-only
ambiguity return without constructing workers, including after restart.
Managed admission releases the master Agent mutex after its durable reservation
so independent owners can be prepared; ACL remains held through submission and
receipt settlement. A failed post-submit owner lock/append remains UnknownOutcome.

The controller polls runtime events, persists bounded progress and projects
device-scoped worker_statuses separately from the existing prompt_statuses.
Fresh Cancel resolves the caller's own live target before foreign-device
matches and sends only its typed runner job ID. Exact cancellation receipts
remain queryable after tracker removal via the shared cancellation namespace.
Master prompt/model admission is fenced while worktree lanes exist. A completed
lane shuts down and retires idle project workers after all project lanes finish;
an ambiguous progress append does not prove runtime termination. Shutdown signals
all worker lanes before joins and shares the master join deadline; unresolved
execution is persisted conservatively as unknown, not cancelled success.

Headless listing now advertises worker_control only with a durable checkpoint
and current project Control grant. Android treats absent worker_control/statuses
as disabled/empty, so desktop adapters without a worker scheduler do not expose
the new composer. Its strict bounded WorkerStatusCodec requires master project/
session plus stable subtab, safe state/failure fields and distinct request IDs;
raw diagnostics, stream fields and tool arguments are refused.

The Android worktree-task composer captures the selected host subtab and integer
revision in an exact Prompt request. It is separate from master messages and
steering, and submits through the existing foreground-only persistent command
path. Accepted receipts require the matching subtab as well as master routing,
request and job IDs; acceptance never implies execution success. Running worker
tasks have a distinct Stop control. Error-phase cancellation requires retained
live worker authority and an exact accepted/running target. Offline saved
decisions can only be explicitly retried with the identical envelope; no
automatic write retry is added. Current display cache v4 still discards
worker targets/statuses and worker authority rather than restoring control.
Authority loss uses that display-only conversion. Main/unit/instrumented source
checks are not current APK or device execution proof.

ProjectRuntimeHost now owns the same WorktreeHost used by HeadlessHost. The
production TUI owner loop polls these lanes, routes subtab Prompt and target
Cancel before master dispatch, advertises worker_control/statuses through its
workspace listing and routes worker approvals with the shared selector. This
last route must not be shadowed by the TUI's previous master-only Approve branch.
Requests queued while local master/session/input work is admitted cannot create
a new worker lane; exact old receipts remain queryable. Existing worker lanes
fence master prompt/model admission and defer queued desktop input. New local
desktop submission preserves rejected draft instead of executing against a
project whose workers are active. The TUI renderer still does not show worker
transcripts or offer local worker task submission/cancel controls.

The TUI restores terminal mode before joining; it signals worker shutdown before
the master join and consumes worker progress with a shared worker deadline.
ProjectRuntimeHost Drop also closes owned worker lanes through the bounded
shutdown path, protecting callers outside the full terminal loop. Desktop-
admitted worker receipts are shared with a later headless host and are answered
without modifying worker journals or creating a parallel session writer.

Local TUI worker task controls, effective live quota resize, worker transcript/
provider stream multiplexing and physical LAN acceptance remain unfinished.

Confirmed successful worker completion now includes an optional bounded output
projection in its durable progress/status record. Only assistant text from the
actual ProcessResult is projected; the admission receipt remains unchanged.
The host first redacts the entire result (up to 64KiB), then truncates the safe
UTF-8 text to 4096 bytes. output_text and a boolean output_truncated appear
together. NUL or larger input bodies produce only output_withheld=true instead
of slicing an unsafe raw prefix. Registered and conventional secrets and private
PEM bodies use the shared redactor; arbitrary unknown/obfuscated secrets remain
a residual risk. The private worker journal is not rewritten.

The master progress journal retains only that redacted output with device/
request/project/master-session/subtab scope, so completion display survives
restart without replaying the worker. Durable projection revalidates/redacts
the stored display again. Failed/cancelled/unknown results have no output fields;
an ambiguous progress append also drops the retained output instead of attaching
success text to UnknownOutcome. Existing target ACL and device-scoped listing
still apply, and no transcript cursor advances merely because task output is
displayed.

Android WorkerStatusCodec accepts exact output-text/boolean-truncation pairs or
the sole true withheld flag only on succeeded tasks. It rejects oversized/NUL,
partial/mixed/string-flag or non-success output. The task display renders the
result and explicit truncation/withholding state. Cache v3 still discards all
worker display/authority. This adds completed-result synchronization, not full
worker transcript paging, tool-event projection or live provider streaming.

The shared WorktreeHost now submits a host-owned WorkerPrompt containing text
plus the immutable request/device correlation, not only a naked String. The
observed prompt runner uses the existing Agent process_with_cancel_and_events
pipeline and feeds TextDelta/TextDone into LivePromptText. Each lane owns a
bounded live map keyed by device/project/master-session/request. Independent
worker Agent sessions remain the execution owners; that separate routing key
is not a duplicate journal writer or transcript cursor.

worker_statuses may include stream_text and optional true stream_truncated
only while accepted/running. Existing LivePromptText publishes complete lines,
withholds NUL, caps raw accumulation at 4096 bytes and redacts every publishable
prefix before transmission. A partial credential line is never published raw;
overflow retains only the previously publishable complete prefix. These values
are transient, never appended to progress journals. The controller removes the
live map entry on runtime completion/rejection before persisting the terminal
state; completed output uses the separate confirmed-result fields above.
Shutdown drops lane-owned live maps, and restart does not manufacture preview.

Admission still checks the full current ACL/owner/revision and immutable receipt
before constructing a worker. The adapter now accepts captured runner scope/
quota independent of input type, so string-based local callers retain their
original path while controller-owned correlated prompts use the same gate.
Exact retries never enqueue another WorkerPrompt or mix its stream identity.
Android reuses the existing strict live-text state/byte/flag validation and
displays worker preview/truncation without restoring it in offline cache. This
adds running text synchronization, not arbitrary worker tool events, complete
transcript paging or actual camera/LAN/device acceptance.

The shared HostAdapter now projects up to 16 combined master/worker approvals
per project and routes Approve by the presence of worker_session_id. Exact
field validation still happens in the chosen adapter; null/invalid targets
do not fall back to the master coordinator. The headless owner mailbox and
desktop remote route both use this adapter. Worker prompt admission and live
scheduler ownership are still separate unfinished wiring, not enabled by
this approval route.

Android accepts either the exact master summary or its worker extension with
worker_session_id and stable subtab_id together. Duplicate IDs are scoped by
worker session, including the distinct master target. Command creation retains
the chosen worker session in the immutable saved request; foreground error
recovery compares that session in addition to request ID and digest. Cached
offline display still contains no approval target or approval authority.
Approval cards and confirmation identify the worker. Receipts retain the
existing six-field master-session routing and request correlation, so exact
saved worker decisions use the same durable explicit-retry path. These are
source/mailbox/JVM checks, not current APK, device UI or physical LAN proof.

`directory_tab_control` separately admits the non-Git directory-tab path.
`subtab_add` currently accepts exact arguments `kind=in_place`, `name`, `cwd`,
and integer `subtab_revision`. The directory must already exist strictly below
the authorized project's canonical root. Parent traversal, escaped symlinks,
the project root itself, duplicate names/roots and the 32-tab overflow fail
before commit. No directory, branch, session or Git worktree is created.
`subtab_close` accepts an ID/revision and only removes an InPlace tab from
the checkpoint. It never deletes a directory or invokes Git. Git Worktree
kinds use a separate Git removal path below; Main is refused.
Active selection falls back by stable identity.
Both use the same ACL, busy, two-revision and exact retry receipt boundaries.
This existing-subdirectory path does not create an isolated workspace.

With `worktree_control`, `subtab_add` also accepts exact `kind=worktree`, `name`
and integer `subtab_revision`, with no client-supplied filesystem destination.
The name is 1..128 ASCII alphanumeric/underscore/hyphen bytes, not a leading
hyphen. The host derives `.zenpi-worktrees/<name>` under the canonical project
and uses the existing bounded Git helper to create a fresh branch/worktree.
Capacity, duplicate name/root, ACL, both revisions and discussion/arch busy gates
are checked before Git. `subtab_close` on a Worktree preflights registration,
shared Git directory and tracked/staged/untracked/ignored cleanliness, then
uses ordinary Git removal. The branch remains. Preflight refusal keeps the tab,
selection, revisions and contents unchanged.

Immediately before Git, the owner checkpoint durably reserves the exact
device/request/fingerprint in the shared bounded workspace receipt namespace
with a null result. After Git succeeds, tab state, receipt and both revisions
commit atomically. Null reservations return `unknown_outcome`, never success,
and exact retry never reexecutes Git, including after process restart. Changed
content under that request ID fails `request_id_conflict` across other command
families too. Git failure/timeout or a post-Git checkpoint conflict retains this
reservation and preserves any filesystem result for explicit host repair; no
automatic rollback, forced deletion or Git replay occurs. Android keeps its
encrypted pending envelope and fences other writes on unknown outcome.
New reservations also store the original envelope, prior/candidate tab states,
derived path and branch as a private bounded worktree intent. Restore validates
its fingerprint, exact command, root, stable identities and revision transition;
unknown legacy null reservations remain readable but cannot be automatically
reconstructed. The local operator can inspect and reconcile completed Git
effects, after stopping the desktop host:

```sh
zenpi host pair --workspace /absolute/project --session /absolute/session.jsonl --list-worktree-intents
zenpi host pair --workspace /absolute/project --session /absolute/session.jsonl --reconcile-worktree REQUEST_ID --device DEVICE_ID
```

These exclusive local actions run before identity/listener creation; they do
not pair, grant capability or send a provider request. Reconciliation requires
the original project/session and workspace/subtab revisions, idle discussion
and arch owners, and a matching observed Git result. Create requires a canonical
registered same-repository worktree on the intended branch (dirty user bytes
are preserved). Remove requires the path absent, no Git registration and an
unchanged canonical non-symlink managed directory. It never reruns Git, removes
files or branches, overwrites a newer tab revision, or invents success from an
absent/unregistered creation target. It atomically settles the original receipt
and candidate using the original request owner value, so Android's explicit
same-envelope retry can validate the result after host restart. A conflicting
checkpoint fails CAS. More complex divergent repairs, proof of a no-effect
failure, legacy intent reconstruction, Android assisted recovery and the full
crash-window matrix remain unfinished; this is not full S2-017 acceptance.

The local managed Git worktree helper no longer forces removal. Add/remove
targets must be direct children of the canonical project's `.zenpi-worktrees`
directory; symlink containers/targets, parent traversal, existing add targets,
invalid branch names and unregistered/foreign remove targets fail before the
Git action. Removal checks the shared Git directory and refuses status output
including tracked, staged, untracked and ignored files, then uses ordinary Git
removal so locked worktrees still fail. It retains the branch. TUI close keeps
the tab/selection on failure and reports the reason. Busy local owners refuse
create/close. Remote commands use the same Git helper, plus the durable
pre-Git request reservation described above. Full recovery/reconciliation
remains required rather than being inferred from successful Git calls.

Managed worktree Git uses the existing Unix owned-process-group supervisor,
closed stdin and nonblocking stdout/stderr. All Git checks and the final Git
action within one add/remove share a 15-second deadline; each output stream
is capped at 256 KiB while reading, not after collecting unlimited output.
Failure/overflow/timeout terminates the owned group and reaps the direct child,
including ordinary hooks/pipe holders that outlive the Git leader. The child
receives the minimal host tool environment, no inherited `GIT_DIR`,
`GIT_WORK_TREE`, config injection or provider credentials; system/global Git
configuration is disabled for these managed operations. Repository-local
configuration/hooks remain active and subject to these process/output bounds.
Timeout/overflow reports an uncertain filesystem outcome, retains the view
checkpoint and does not automatically roll back a potentially created tree.
This is not filesystem transactionality, hook sandboxing, protection against
a descendant deliberately escaping its group, or a nonblocking TUI worker:
slow operations can still occupy the owner thread until the deadline, and
blocking filesystem syscalls are outside this subprocess deadline.

The remote protocol is independent of existing local headless JSONL v1/v2.
Authenticated self-revocation uses `device_revoke_self` with empty arguments,
no project/session/turn/sequence, `required_owner` equal to the authenticated
device ID and `expected_revision` equal to `device_revision` returned by that
connection's authentication response. It needs no project grant and cannot
name another target. The registry lock fences current identity/revision through
durable revocation. A successful `{device_id, revoked:true}` response is sent
before closing this connection; ordinary post-dispatch ACL checks must not
replace that one confirmation with an unauthorized response. All future device
authentication/commands fail after revocation. It also works on a pairing-only
listener without a workspace adapter. Storage failure is `unknown_outcome`,
because atomic replacement may already have occurred. The registry stores
`revoke_request_id` atomically with revoked state; legacy/local-operator
revocations have no matching request receipt. A revoked key cannot use ordinary
authentication or retry any mutation.
For explicit recovery, `revoke_status` is a separate first-message path after
hello/challenge. Its payload contains only device ID, original request ID and
signature. It has its own envelope query ID and no project/session/owner fields.
The signed JSON tuple is `["zenpi-device-revoke-status-v1", host_fingerprint,
nonce, device_id, original_request_id, query_id]`. The connection-bound challenge
is consumed once, even on failed proof. The host verifies the retained public
key and returns only `{device_id, request_id, committed}` before closing, never
an authenticated handle or workspace data. `committed=true` requires the exact
request ID in the durable revoked record. False/missing receipt is not proof
the device remains authorized and never triggers automatic mutation retry.
Android persists a UUID self-revoke intent before transmission in its existing
host-bound encrypted record. An unresolved intent blocks normal resume, new
workspace writes, re-pair and background snapshot admission. Cancellation and
process restoration retain it; this is a durable uncertainty marker, not itself
proof of successful revocation. A user-requested signed receipt query may
resolve a matching host result without re-sending the revoke. Local completion
checks the saved host/intent before removing the signing key and metadata.

Remote envelopes require an explicit integer `schema_version: 1`; missing or
unknown versions fail closed. No implicit downgrade or JSONL fallback exists.
Control `hello` offers 1..8 versions; `welcome` selects the supported version.
New incompatible semantics require a new version and explicit negotiation.
Unknown envelope/message fields and unknown enum values are rejected. Arbitrary
JSON is allowed only inside documented data/result/arguments/state objects and
must be validated by the operation owner before any side effect.

## Framing And Resource Limits

Each frame is a four-byte unsigned big-endian length followed by exactly that
many bytes of UTF-8 JSON. Zero length and length greater than 1,048,576 bytes
are rejected before allocating or reading the body. There is no newline marker.
Control and event frames are capped at 262,144 bytes including the envelope;
bulk snapshot frames are capped at 1,048,576 bytes. Identifiers are 1..128 UTF-8
bytes and contain no control characters. Payload JSON is capped at 32 nesting
levels and 4,096 values. Replay pages contain 1..128 events.

Clean EOF before a prefix means the peer closed. EOF inside a prefix/body is an
error. Any malformed message, timeout, partial write or flush failure makes the
connection unusable; callers must close it rather than retrying the codec on
that stream. Interrupted reads retry. Writers validate and serialize into a
bounded buffer before emitting a prefix. The transport owner must configure
read/write deadlines and enforce an absolute frame/handshake deadline and
connection/queue caps; these blocking primitives do not themselves enforce a
wall-clock deadline on arbitrary `Read`/`Write` implementations.

## Envelope

| Field | Rule |
|---|---|
| `schema_version` | Required integer 1 |
| `channel` | `control`, `event`, or `bulk`; must match message type |
| `id` | Required for request/response; stable request correlation ID |
| `session_id`, `project_id`, `turn_id` | Optional bounded correlation identifiers |
| `sequence` | Event-only unsigned transport sequence; not a journal cursor |
| `code` | Response-only typed error code; required on failed response |
| `execution_state` | Response-only; accepted/running/succeeded/failed/cancelled/unknown_outcome |
| `required_owner` | Request-only expected owner epoch, not an ACL credential |
| `expected_revision` | Request-only unsigned compare-and-swap revision |
| `payload` | Required strict tagged object with `type` |

Control payloads: `hello {supported_versions}`, `welcome {selected_version}`,
`request {operation, arguments}`, `response {result}`, `ack {cursor}`,
`resume {cursor, limit}`. Event payload: `event {name, data}`.
Bulk payload: `snapshot {cursor, state}`. Cursors have `session_id` and
`next_sequence`, and must match the envelope session.

Control `pair {token, fingerprint, device_name, public_key, signature}` requires a
correlation `id`, a 43-character base64url token (32 random bytes), a lowercase
64-character certificate fingerprint and a bounded device name. The former
six-digit safety code was derivable from the QR itself and is removed (ZE-514).
Pairing fields are redacted in Debug output. After TLS handshake,
the host requires hello/welcome negotiation before the pair request. The
current response returns `device_registered: true` only after signature
verification, invite consumption and durable device registration; it includes
`device_id`, `device_revision` and a truthful `control_available`. Scanning the
QR shown only on the host owner's screen is the desktop confirmation: the device
receives Read, Control and Approval on the project mounted by `--workspace`
(none when nothing is mounted, so `control_available` is false).
The connection then closes. This is still a development schema with no released
v1 client; older unsigned pairing clients are rejected, never silently admitted.

## Device Identity And Authorization

Device keys are P-256, uncompressed SEC1 public point (65 bytes, base64url
without padding). Device ID is lowercase SHA-256 of that point. Pairing proof
is an ASN.1 DER ECDSA/SHA-256 signature of compact UTF-8 JSON string array
`["zenpi-device-pair-v1", fingerprint, token, request_id, device_name, public_key]`.
Rust/Kotlin must serialize without HTML escaping. All six fields are signed;
invalid proof never consumes the invite. Android generates the private signing
key in Android Keystore and never exports it.

After welcome, each connection receives `challenge {nonce, host_fingerprint}`.
Registered clients may send `authenticate {device_id, signature}` with an ID.
The proof signs `["zenpi-device-auth-v1", host_fingerprint, nonce, device_id]`.
Nonce is the canonical unpadded base64url encoding of exactly 32 random bytes
(43 characters). Rust and Kotlin validate decoding and round-trip encoding;
nonzero unused bits in the final character are refused before device signing.
It is connection-local, expires after 30 seconds and is
consumed even by a failed authentication attempt. The host independently reads
the current private device registry; revocation and permission revision changes
fence earlier authentication handles. The current authentication response has
`authenticated`, `device_id` and `control_available`. Without a workspace
attachment it then closes. An explicit `--workspace PATH --session PATH`
attachment retains the prepared production owner pool and admits authorized
read requests on the connection with a 30-second absolute per-request deadline.
Android can authenticate a saved host and retrieve a bounded snapshot.

The host registry is bounded, private, owned, non-symlink, host-fingerprint-bound,
cross-process locked and atomically persisted. Corrupt/rotated identity storage
fails closed. A device paired while a project is mounted has Read, Control and
Approval on exactly that project; one paired with nothing mounted has Read and an
empty project ACL. Local host permissions distinguish Read, Control and Approval;
no global wildcard is used. Re-pairing never restores revoked keys; a fresh scan
adds the currently mounted project to an existing grant and is otherwise a no-op.
`zenpi host pair --identity PATH --list-devices` lists local records;
optional `--json` emits only id/name/revoked/revision/capabilities/projects for
each local record, never public/private key or authentication proof fields.
The existing text format remains unchanged. `--json` without --list-devices is
refused. This is local permission inspection, not a remote listing capability.
`zenpi host pair --workspace PATH --session PATH --list-projects` prints local
stable project IDs, directories and active selection for explicit ACL grants.
This exclusive read-only action runs before identity/provider/listener creation,
uses an existing session and bounded non-symlink owner checkpoint, and does not
publish, migrate, repair or grant permissions. A missing checkpoint projects
the bootstrap workspace identity without creating a checkpoint. Malformed or
mismatched stored state refuses rather than inventing an authorization target.
`--revoke-device ID` durably revokes before returning and starts no listener;
the revoked device is refused on its next frame. The former four
`--grant-device-*` actions are removed (ZE-514): pairing itself grants the
mounted project. Revision fencing and restart persistence apply unchanged; no
remote permission-escalation command is added.
Android's Forget device wipes local key/metadata only; it is not host revocation.

The current read adapter exposes `workspace_list` (authorized IDs, display titles,
session IDs and authorized active project only) and `inspect` (matching project
and session, phase, model, last at most 64 redacted turns within a 96 KiB budget).
It uses the actual Agent owner, rejects busy locks and stale/revoked devices,
and does not expose workspace paths or turn metadata. `journal_next_sequence`
is explicitly scoped as `session_journal`, not an event replay cursor. Both
results advertise `mutations_available:false`; unsupported operations fail.
The default standalone attachment prepares its own owner. Explicit desktop mode
`zenpi host pair --desktop --workspace PATH --session PATH` instead starts the
production TUI with its one existing ProjectOwnerPool. It shows a full-screen
QR, refreshes on R, and cancels the invite on Esc before returning to the desktop.
Already registered devices may continue reading until desktop shutdown. The
idle remote-enabled desktop can reopen pairing with F3, refreshing the one-time
invitation on the existing listener and host identity. Ordinary TUI without
remote options never starts a listener from that action. Active/queued model,
session-transition/input-control work and pending approvals refuse reopening;
modals retain keyboard ownership and unsent composer text is not submitted.
listener closes and joins on desktop exit. NO_COLOR is overridden only while
rendering the QR because half-block geometry requires black/white cell colors.
Desktop pairing rendering captures one wall-clock observation for validation,
encoding and countdown. At expiry it shows a recoverable expired state without
QR rather than propagating invite validation failure through the desktop loop.
An unavailable clock or backwards jump beyond the maximum invitation window
likewise suppresses QR with recoverable clock state. Refresh remains an explicit issuer
operation and invalidates the previous token; rendering never renews admission.
ASCII metadata/footer wrap separately from the fixed QR module grid. The minimum
height includes their actual wrapped rows at the current width; insufficient
space shows resize requirements rather than clipping the QR or recovery actions.

Desktop requests use an eight-slot owner-thread mailbox, not another Agent or
file-polling session replica. The TUI drains at most eight requests per pass.
Before starting it atomically claims queued work; a two-second caller timeout
atomically cancels unclaimed work so it cannot mutate later. A write already
claimed when the caller times out returns unknown_outcome, never a false failed
or succeeded state. Retrying the exact request resolves its durable receipt.
A provider-owned busy Agent returns Busy without blocking on its execution
lock. Transcript replay and workspace control use this same mailbox.
Live provider/tool delta delivery remains unimplemented.

## Workspace Control Frontier

The shared-pool adapter now handles workspace_open/select/close through the
existing host-owned `ProjectOwnerPool`. The desktop mailbox routes those same
actions on its owner thread, fences close against local active jobs, queued
inputs, pending input controls and session-replacement work, and reconciles the
TUI tabs, drafts, metadata and active owner after commit. Resource and Gantt
projections refresh on revision changes. Each authorized project advertises
workspace_control only when it has explicit Control permission and a durable
checkpoint. The generic mutations_available flag remains false; individual
write families advertise only their implemented capabilities. Desktop mode
advertises the separate prompt_control and approval_control capabilities below.

Android captures an immutable owner/revision/request-ID envelope and sends it
once. Project tabs select the host project when workspace_control is present;
otherwise they retain read-only client selection. Open and close actions use
explicit dialogs. An ambiguous write disables new actions and offers retry of
the same envelope; a read refresh alone does not silently clear the unknown
request. Cached offline projections deliberately discard write authority until
live reauthentication. Before network submission, Android atomically persists
the exact bounded request envelope in its encrypted host/display record.
Successful write plus subsequent state fetch clears it in the same commit as
the new display/cursor. An explicit correlated refusal clears it without
claiming success; authentication/read failures after a possible write do not.
Process restart restores the request offline and never automatically submits
it. Cancel, rescan and ordinary read caching cannot replace an unresolved
request; explicit Forget wipes it. Same-ID retry is an explicit user action.
The crash-after-host-commit case is tested against the production desktop.
Broader fault cases, schema migration and the complete lifecycle matrix remain
required.

workspace_list includes required_owner (a random pool-generation ID) and
workspace_revision. Writes must carry those values plus the addressed project's
session ID. Local pool publication and session replacement advance the same
revision. A restarted pool gets a different owner generation. Unauthorized
projects, stale device handles, owner mismatch and revision conflict are
rejected before publication. Opening a canonical directory requires explicit
Control and Read grants for its stable target project ID; path entry never
implicitly expands the ACL. Closing requires the replacement active project
to be authorized too and refuses busy/replaced owners and pending input.

For these workspace writes, a successful request's exact envelope fingerprint,
device ID, request ID and bounded result are committed atomically with the
workspace/session mapping in project-workspace.json. An identical retry returns
the durable result, including after restart; reuse with different content
returns request_id_conflict. Authorization is checked before replay. Receipts
have a hard capacity of 128 and are never silently evicted: new IDs return
resource_limit at capacity while retained retries still work. Receipt rotation
and scalable retention policy remain required. The old metadata-only checkpoint
migrates on its next commit. Registry revoke/grant holds the same cross-process
lock as a workspace mutation commit so authorization cannot change mid-commit.

## Transcript Replay Frontier

Inspect results include `event_cursor` and `event_cursor_scope:transcript_events_v1`
in addition to the separate journal sequence. The event sequence is the durable
turn ordinal: opaque journal events, credentials and turn metadata are never
projected. Reopening the same valid journal reconstructs the same sequence.
This projection covers committed transcript turns only, not provider deltas,
approvals, tool progress or all TUI state.

An authenticated `resume` requires request ID, project ID and matching session
cursor. Authorization is independently checked as project Read. The host emits
at most 128 contiguous `transcript_turn` event frames within a 96 KiB page, then
a correlated terminal response containing cursor/scope, has_more, phase and
journal sequence. A cursor ahead of the host or bound to another session is
refused. The retained replay window is the most recent 256 turns. An older
cursor, oversized individual event or branch-tree session yields one explicit
bounded bulk Snapshot followed by a snapshot_required terminal response. Tree
sessions require snapshots because selected ancestry is not append-only.
Snapshot and its cursor are captured under the same owner lock, so a later
append is retrievable on the following resume without a gap.

Durable turn content is redacted in full before limiting the remote projection
to 12 KiB on a UTF-8 boundary. Truncated turns carry optional boolean
`content_truncated: true` in both event and snapshot projections; the original
journal/Agent content is unchanged. Snapshot transcript_truncated also reflects
partial content, not only omitted earlier turns. This prevents a long final
answer from forcing a snapshot which itself omitted that entire answer.
Android retains the last at most 64 turns within a 96 KiB JSON-encoded display
budget (escaping included), preserves per-turn truncation through encrypted
cache restore, and commits the matching event cursor. Legacy bounded pages
without this field remain readable. An individually oversized newest legacy turn keeps its ID and a
scalar-safe prefix with content_truncated, rather than disappearing while its
cursor advances. Snapshot and replay share this encoded-budget reduction, and
cached identity still permits a later authenticated full-message request.
This display projection is not full bulk
content paging by itself; the read-only turn page extension below retrieves
the omitted remainder within its bounded content limit.

### Remote New Session

session_new is a Control write with exactly empty arguments. It requires the
current project/session, request ID, owner epoch and workspace revision, and
forbids turn/sequence fields. Listings advertise session_control only with a
durable workspace and Control permission. Android offers it in the main subtab
with explicit confirmation, foreground/connected/pending-write fences; it is
never granted by the offline cache. No client path or replacement ID is accepted.

The host refuses active/admitted work, pending input/attachments and retained
worker owners before reservation. It reuses NewSessionPlan and the canonical
Agent replacement, preserving the old conversation and inheriting the owner's
connection/persona state. A null durable workspace receipt reserves the exact
fingerprint before creating the replacement. The successful receipt and new
session mapping are committed in the same CAS checkpoint, which advances the
workspace revision once. The owner's current input/approval handles are updated
with the replacement, preventing steering through the old session port.

Success returns exactly request_id/project_id/old_session_id/new_session_id/
required_owner/workspace_revision/active_project_id, without host paths or
conversation content. Exact retries resolve the receipt before current epoch/
session checks, including restart; changed payload/identity conflicts fail.
Reserved incomplete outcomes remain UnknownOutcome and are never recreated.
Preparation uses a cooperative one-second deadline, not a hard wall-clock
guarantee for synchronous filesystem/extension work. Failures after reservation
remain unknown even when no replacement was ultimately published.

The TUI reloads its transcript only when the actual session changes, not on
receipt replay; old PTY identity cannot be relabeled. Android refreshes the
project and obtains a replacement snapshot rather than replaying an old cursor.
Historical-session listing and resume use the extension below; rename/delete
parity and current device/LAN session-control acceptance remain unverified.

### Session Catalog And Resume

Session IDs are opaque identifiers, not filenames or ASCII slugs: nonblank,
no control characters and at most 128 UTF-8 bytes. Android applies this contract
to catalog entries, retained catalogs, persisted resume targets and success
receipt replacement IDs without trimming or rewriting the identity. Catalog,
owner/CAS, exact-target and request-identity fences remain mandatory.

`session_list` is a project Read operation. It binds request/project/current
session IDs and forbids owner/revision/turn/sequence fields. Initial arguments
are exactly `{}`; continuation arguments are exactly integer `offset` (multiples
of 16) and `snapshot_id`. Results contain exactly project_id/session_id/
snapshot_id/offset/next_offset/entries. Each entry contains only session_id,
created_at_ms, turn_count and current. Paths and conversation content never
leave the host in this catalog. Read permission alone permits listing.

The host enumerates the current journal's parent directory, accepts only clean
regular JSONL session journals with matching canonical workspace, excludes the
domain store, and refuses symlinks, malformed/empty journals and duplicate IDs.
Enumeration is capped at 4096 directory entries, 512 files, 8 MiB per journal,
64 MiB total and a cooperative one-second deadline. Metadata is sorted by
creation time descending, then session ID. Each page has at most 16 entries;
continuations fence the SHA-256 catalog snapshot, including durable records.
Limits fail explicitly rather than silently returning an incomplete tail.

`session_resume` is a Control write with exactly target_session_id/snapshot_id
arguments and the same main-tab, owner/CAS/busy/pending-input guards as
session_new. Targets must belong to that fresh catalog and differ from the
current session. No client filesystem path is accepted. The host prepares the
existing canonical Agent resume, rechecks the catalog before committing, and
atomically writes the new session mapping plus seven-field success receipt.
Exact retries and restarts resolve that receipt; incomplete reservations remain
UnknownOutcome and never cause automatic resume replay. Input/approval handles,
TUI transcript and Android snapshot are reconciled as for new-session control.

Android offers a transient paged session dialog with current-session marking,
confirmation, previous/next/refresh and connected/pending-write fences. Catalog
metadata is not offline authority and is cleared on scope/authority changes.
Device execution and physical QR/LAN session switching are still unverified.

### Current Session Name

`session_rename` is a Control write requiring current project/session/request,
owner epoch and workspace revision. Exact arguments are `name` and unsigned
`journal_revision` matching the current journal's next sequence. Names are
normalized by the existing local name validator, bounded to 64 UTF-8 bytes,
control-free and non-empty; remote requests additionally refuse path separators
and names altered by the credential redactor. The operation changes only the
current main-subtab journal, never a client path or another historical session.
Busy/admitted/pending inputs/attachments/worker owners refuse fresh writes.
The same-project catalog rejects duplicate names before intent reservation.

The host durably binds the request identity, appends a reserved receipt, calls
the canonical SessionStore rename and appends success. Success contains exactly
request_id/project_id/session_id/name/journal_revision; the revision is after
rename and before its success receipt. Workspace revision/session ID do not
change. A no-op name still has a request receipt. Exact retries replay the
receipt without overriding later local names. Reserved or uncertain appends
remain unknown and never cause automatic rename replay. Restart restores the
request binding from checkpoint/journal. Current ACL always precedes replay.

Workspace metadata adds nullable session_name and catalog entries add nullable
name. Legacy absent fields remain readable. Sensitive or path-like local names
are withheld; host journal itself is unchanged. Android displays names online,
confirms a validated edit and uses the encrypted pending-write path; these
labels are deliberately omitted from the existing offline cache schema.
TUI reconciles the actual current owner name without reloading the transcript.
Physical device/LAN rename synchronization remains unverified.

### Subscription Session Replacement

`session_changed` is a typed failure for an authenticated Read subscription
whose freshly authorized workspace listing contains the requested project but
a different current session. It carries no new-session ID, content or write
authority. Missing/revoked projects remain unauthorized; ordinary write or
model/terminal owner mismatches remain owner_mismatch. If switching races the
metadata sample and replay, the host rechecks the authorized listing before
classifying it as session_changed. Otherwise it retains the original refusal.

Android recognizes this signal only for the correlated read-only subscribe
request, not from arbitrary text, a mutation or a foreign project response.
In connected foreground with no pending write/revoke/reset, it clears the
old transient view and in-memory cursor and freshly authenticates/inspects the
same project to obtain the actual current session snapshot. No replacement
identity is trusted from the failure. The durable old display is not deleted
until a new snapshot is committed. Pending mutations are never automatically
resubmitted; owner_mismatch/ACL failures retain the explicit authority-loss
fence. Older clients may refuse the new error code and require manual refresh.
Current APK/device execution of desktop-initiated switching remains unverified.

Initial automatic Android reads may race session replacement before the
subscription starts. A correlated owner_mismatch on ACK/resume/subtab_list/
inspect is not itself sufficient evidence of session change. In automatic
foreground subscribe mode only, the client makes one new authenticated
workspace_list read; the requested project must still be authorized and have
a different session before recovery is classified. No wire error semantics
are weakened, no refused request is replayed, and no replacement identity from
the refusal is trusted. Missing/unchanged project retains the refusal, while
probe revocation or malformed data fails closed. Mutation and explicit page
reads never trigger this automatic probe.
The refused original must now be a fully validated control frame with exactly
schema/channel/id/project/session/payload, canonical cursor where applicable,
and empty arguments for inspect/subtab_list. Authority-bearing, malformed,
turn-routed or explicit-argument requests retain the original refusal without
issuing a listing. This is probe admission, not permission to retry a command.

### Durable Turn Pages

`transcript_read` is a project Read operation, not a write. Its exact arguments
are `turn_id`, integer `offset`, and nullable `digest`. The envelope requires
request/project/session IDs and forbids write-owner/revision, turn and sequence
fields. The host resolves only a turn in the current Agent history (not an
abandoned tree branch), under the existing owner lock and current registry ACL.
Busy owner is Busy; missing/foreign turn or invalid UTF-8 offset is refused.

An ordinary assistant turn additionally accepts `tool_call_id` alongside those
three arguments. The host resolves exactly one matching call from that current
turn, using the same Read grant, session binding, owner lock and active-branch
check. Only the structured-redacted arguments are serialized and paged, never
execution evidence or other metadata. Raw and redacted argument documents are
each limited to 64 KiB, with the same 8192-byte UTF-8 pages and digest fence.
Responses add exactly `tool_call_id` and `tool_name`. Android opens this route
from a retained call preview; next/previous pages bind turn, call and name and
cannot be substituted for message content. Changed call metadata invalidates
the dialog together with changed turns. Offline, authority loss and pending
mutations disable reads or clear the transient selection as for message pages.

The host redacts the full message before slicing at UTF-8 byte boundaries.
Each page has at most 8192 bytes. Original and redacted messages over 4 MiB are
ResourceLimit. Result fields are exact project/session/turn/role, SHA-256 digest
of the complete redacted content, offset, next_offset, total_bytes, has_more
and text. Nonzero offsets require the prior digest; a changed digest is
RevisionConflict, never mixed content. Pages do not update journal, transcript
cursor, ACK or workspace revisions. Repeated reads are side-effect-free.

Android's foreground message dialog requests one page per bounded authenticated
connection and retains only the current page. It verifies scope, known role,
digest/size consistency and contiguous UTF-8 byte offsets. Next page is explicit;
errors retain the previous page and require closing/reopening to restart.
Background/network/reset/write/selection fences cancel the read and remove
the dialog. A verified snapshot that removes or changes the target turn also
invalidates the dialog and pending page generation, including branch switches
with an unchanged physical event cursor. An unchanged target survives unrelated
append/status updates. Page text is transient and never enters the encrypted transcript
cache. Older hosts refuse the new operation, not silently return a fabricated
complete answer. Current evidence includes authenticated rustls in-memory
transport, domain/codec/source compilation, not a
fresh APK/LAN paging acceptance.

The client validates the complete page's contiguous sequence, turn/project/
session IDs, terminal cursor and boolean fields before applying it. Missing or
malformed terminal responses never advance state. A snapshot replaces rather
than appends. Each attach has a 15-second absolute bound and fetches at most
three pages; remaining pages are explicit. This is pull replay, not server push.
Android atomically persists bounded transcript/display and its cursor in the
existing encrypted host record. A subsequent pure read may send `ack {cursor}`
only for that previously committed state, not for a just-received page. The
host advertises `event_ack_available` only with a durable owner checkpoint;
clients skip ACK for older hosts without that boolean capability. Explicit
commands do not automatically send ACK alongside their mutation.

ACK uses project Read permission and exact project/session routing, without
workspace owner/revision fields. It validates the cursor is not ahead of the
current durable transcript. Android also validates canonical u64 cursor text
and bounded scope at construction and receipt admission, and checks the supplied
ACK frame matches expected project/session/sequence before any exchange. Only a
valid optional correlation ID may differ from the builder; foreign routing,
operation or authority fields refuse locally rather than validating a receipt
against different expectations. The host then
atomically persists a device/project/session
high-water mark in the existing checkpoint. Duplicate or older positions return
the stored position without rewriting bytes; workspace/subtab revisions do not
change. The registry revoke lock spans authorization and checkpoint CAS. Busy
owners, missing checkpoint, unauthorized/foreign sessions, future cursors,
capacity and CAS failure refuse rather than deleting data or changing the
client cursor. ACK entries are bounded to 128, restored strictly (duplicates
and malformed identity fail), and legacy checkpoints lacking entries remain
compatible. There is no ACK-driven journal deletion or replay-window eviction.
This records a client's durable claim; it cannot prove an untrusted client
actually persisted bytes. Live push, stream-delta ACK and ACK retention/cleanup
policy across many sessions remain unfinished.

## Pairing Transport

Device-registry locking retries only exclusive-lock contention for at most
100 ms, then returns the distinct `busy` error. Once acquired, authorization
reloads the current private registry; no cached grant or stale revision is
accepted. Busy authentication consumes its connection nonce and requires a
fresh challenge on a new connection. Corrupt/inaccessible registry, invalid
proof, stale device revision and revoked identity remain fail-closed.
Read requests refused before dispatch return failed/busy without projection.
Mutations unable to acquire authorization return failed/busy before reservation
or side effects. If dispatch has already happened and a later frame-delivery
authorization check is unavailable, a write returns unknown_outcome/busy,
never a false no-effect refusal. Saved envelopes are not automatically replayed.
Android admits a coded unknown outcome only for a typed write response with
null result, after response correlation/routing checks. It preserves the code
in WorkspaceUnknownOutcomeException without confirming or resolving the saved
request. unauthorized/revoked_device/owner_mismatch remove live authority while
retaining exact pending identity; Busy does not mean revocation. Coded unknown
steering outcomes stop receipt polling rather than transmitting another proof.
Read/subscription outcomes and object-valued unknown results cannot use this
exception to smuggle success/error combinations. Ordinary succeeded responses
with nonnull error codes remain refused. Self-revoke also retains its unknown
intent instead of treating the delivery refusal as a confirmed wipe.
Self-revocation lock contention is a no-effect Busy refusal; ambiguous storage
failure after publication remains unknown. Ordinary parallel reads therefore
do not translate transient lock contention into device revocation.

`zenpi host pair` alone starts the listener; normal TUI/headless invocation does
not. TLS is restricted to 1.3, with ALPN `zenpi/1`. The host has a persistent
self-signed ECDSA certificate whose complete DER SHA-256 is the v1 pin. Clients
must compare that pin before sending the one-time token; ordinary public-CA
validation is not a substitute for the QR pin. Host identity is stored in one
bounded, private, owner-checked file, published atomically without overwriting
an existing identity. Invalid/public/symlink/hard-linked identity files fail
closed. Unix identity loads use non-blocking no-follow open and require a
regular file with exactly one link before parsing, so a FIFO/device cannot
stall pairing startup or become identity authority. Rejected files are not
replaced or repaired automatically.

QR URI: `zenpi://pair?v=1&address=<literal IP:port>&fingerprint=<sha256>&token=<base64url>&expires=<unix seconds>`.

Shared invitation fixtures bind raw URI, validation clock, normalized address
across Rust and Android. Scheme/authority must use
the exact zenpi://pair origin, v must be exactly 1, and expiry must be unsigned
decimal with at most 19 digits. IPv4 link-local and IPv4-mapped IPv6 wildcard,
multicast/broadcast/link-local targets are refused on the host as on Android.
Mapped broadcast detection on Android uses the parsed IP rather than only the
original host spelling. Duplicate query names, including escaped equivalents,
remain invalid. Loopback is allowed for explicitly scoped local fixtures.

The remote_qr_fixture example rasterizes the actual production terminal QR
cells into PGM fixtures. Android JVM tests decode those pixels with ZXing and
then validate the exact URI/address. Fixtures use a public deterministic
test token and never start a listener or create an accepted invite. This proves
tested terminal-renderer/scanner-engine compatibility, not physical camera,
foreground Android transport or LAN success.
There is no hostname resolution, credential, fragment, userinfo or extra query
field. Duplicate fields are refused. IPv4 and unscoped IPv6 are supported;
unspecified/multicast/broadcast/link-local scoped addresses are refused. Invites
last at most 300 seconds and are held only in process memory. Both monotonic and
wall-clock expiry are checked. Refresh/cancel invalidate earlier tokens. Host
admission also enforces the complete remaining 1..300 second wall window,
matching QR validation. Observed expiry, an unavailable wall clock or a rollback
beyond that window permanently retires the active invitation under the issuer
mutex. Correcting the clock cannot revive it; explicit refresh issues a new
token. Invalid proofs on an otherwise fresh invitation remain non-consuming.
The monotonic deadline independently prevents a smaller rollback from extending
the invitation's original lifetime. No invitation is persisted across restart.
A mutex serializes consumption so concurrent scans yield exactly one winner.
There is no separate safety code: anything derived from the QR proves nothing the
QR does not. The client requires a human tap after showing the pinned
fingerprint. This is not mTLS or an untrusted-network release.

The opt-in default binds `0.0.0.0:0`. `--bind IP:PORT` narrows the interface;
`--advertise IP` selects a reachable literal IP, with address-family/interface
consistency checks. Without an explicit advertised IP or exact bind, Unix
interface enumeration prefers a unique same-family RFC1918/IPv6 ULA address.
Down, loopback, point-to-point, multicast, unspecified, broadcast, link-local
and scoped IPv6 candidates are excluded. A packet-free UDP routing hint
resolves multiple private candidates only when it names one of them; otherwise
the command requires `--bind IP:PORT` or `--advertise IP`. A unique private
interface works even when route selection fails (offline LAN), and takes
precedence over a public or CGNAT default route. With no private candidate,
the existing valid same-family route fallback remains. Exact bind/advertise
does not depend on enumeration. LAN reachability still
depends on the actual network and firewall. No NAT traversal claim is made.
The listener caps active connections (including incomplete TLS handshakes),
enforces an absolute handshake deadline, closes owned sockets on shutdown and
joins owned connection threads. Handlers must set an absolute per-frame
deadline using `DeadlineSocket::set_deadline` rather than extending the deadline
for each arriving byte. Desktop pairing supports full-screen QR, refresh and
cancel; `--plain` prints a sensitive one-time URI only on explicit/non-TTY output.

## Desktop Prompt Admission Frontier

The non-TUI `host pair --workspace ... --session ...` host now services the
same bounded owner mailbox on an owned thread. One bounded BackgroundRunner
per active independent project uses the existing project Agent and the same prompt reservation, checkpoint
binding, ACL, selected-root validation, steering, cancellation and approval
adapters as the desktop path. `workspace_list` advertises `prompt_control`
only for a checkpoint-backed project with a current Control grant, and exposes
device-scoped live/durable prompt statuses. Connecting or pairing never grants
Control by itself. Prompt jobs continue after their TLS connection closes;
listener expiry/shutdown signals all active runners before joining them and closes its owned thread.
Shutdown drains matching terminal events before persisting progress: confirmed
runtime success/failure is retained even if the owner loop has not polled it.
Missing results, runtime join failure and cancellation (which may mean detach)
remain conservatively `unknown_outcome`, never permission to retry.
Non-cooperative jobs retain the existing
bounded-runtime detach limitation. A captured active job blocks closing its
project or subtab before execution; selection still cannot redirect its root.
This is not the stdio JSONL command loop, a parallel worktree pool, or proof of
complete headless/TUI feature parity. Inline host-workspace file references now
share the desktop parser and attachment resolver; remote resource control and
phone upload/media browsing remain unfinished requirements.

Both desktop and non-terminal prompt runners resolve explicit `@path` or
`@"file name"` tokens at word boundaries through the same owner helper after
entering the captured subtab workspace. Email-like embedded `@` stays literal;
leading slash-command arguments stay literal. Empty/unclosed paths, more than
eight references, paths over 4096 bytes, traversal, absolute paths, symlinks,
missing files and files above the existing 10 MiB limit fail before provider
invocation. Agent materialization retains aggregate/model capability checks
and reopens bounded bytes through ToolContext. These are desktop-host files,
not uploads from Android storage. Attachment bytes do not enter the journal.
An accepted prompt can later fail file preparation; accepted is not success.
Remote admission refuses Busy before reservation when the local composer has
staged attachments. Both TUI and non-terminal remote workers recheck under the
existing Agent lock before building input, closing the queue-time race. They
never clear, consume or merge those local references into remote input. Exact
retry still returns its old durable receipt even when local attachments have
since been staged. Local submission/explicit clear remains the local owner's
action, not a network side effect.
Exact prompt retry retains the durable admission receipt without another
provider call. This does not add attachment handling to text-only steering.

`files_list` is a Read operation with exact `subtab_id`, unsigned
`subtab_revision` and relative `path` arguments in the legacy request. Paged
requests add exact `offset,snapshot_id`: offset is unsigned, a multiple of 64
and bounded by 2048. A fresh first page uses offset 0 and null snapshot; later
pages (including back to 0) require a lowercase SHA-256 snapshot. Request/project/session IDs
are required; write authority, outer turn and sequence fields are refused.
Host checks current ACL, selected subtab and revision, captured-root validity
and existing owner availability. It returns exact project/session/subtab scope,
path, entries and truncated, plus optional page {snapshot_id,offset,next_offset}.
Entries carry name, native relative path, file or
directory kind, nullable unsigned size and attachable. No file bytes or host
absolute roots are projected. Hidden path components, symlinks, non-UTF8 names,
control-bearing/redacted names and non-file/directory entries are omitted or
refused. Immediate enumeration visits at most 2048 entries, retains at most
64 sorted directory-first entries in legacy mode, checks a 250 ms cooperative deadline, and
refuses a result above 64 KiB. The deadline does not bound a blocked filesystem
syscall. Paged mode retains the complete bounded metadata scan and returns
64-entry pages. An incomplete scan fails ResourceLimit rather than declaring
an inaccessible tail. Snapshot binds project/session/subtab/revision and complete
directory metadata; additions, removals or size changes invalidate later pages.
No content hash/file bytes or recursive traversal is introduced.

Android uses pinned TLS/device proof, socket cancellation/LAN binding and the
existing absolute connection deadline for each list. It strictly checks scope,
path/name relationship, count, kinds, numeric types, sizes and flags. Selecting
a file only inserts its escaped quoted @ reference; subsequent prompt admission
revalidates and materializes it. Read-only directory results are transient,
not encrypted cache entries, transcript cursors, ACKs or workspace mutations.
Android shows previous/next/page number, retains the same bound snapshot for
paging, and clears unavailable/stale pages. Initial explicit invalid_request
may fall back once to the older three-field read; later pages and other errors
never trigger that fallback. Hosts without files_list still refuse rather than
imply an empty successful directory. No failed read is retried as a mutation.

Android file metadata browsing does not require prompt control. A Read-only
device, or a controlling device with an accepted/running task, has a separate
Browse host files entry. Connected current project/session/selected-subtab
scope authorizes browsing; scanning, reset, loading, pending write or revoke
fences it. Directory navigation and paging additionally require a successful
matching metadata response. A failed read can refresh without attachment
authority. Inserting a file reference separately requires prompt control and
no accepted/running task. The shared FileReadPolicy gates Compose and ViewModel;
metadata reads never send a task or grant prompt authority. JVM tests execute
these boundaries; Compose tests compile but still require a device run.

The explicit AndroidRustFileTest lane now writes current framed files_list
requests and consumes framed responses from the actual Rust ProjectRuntimeHost
owner with a Read-only authenticated fixture device. It covers 64-entry pages,
tail/back navigation, a nested directory, size-change snapshot conflict, stale
session refusal and revoked-device denial, with unchanged host journal and no
file bodies/private roots in responses. It exercises fragmented JVM reads and
scope/snapshot mismatch decoding; it is not production TLS, Android Keystore,
APK or physical QR/LAN evidence.

Remote file listings additionally apply the existing tools::credential_path
policy to every path component, case-insensitively. Credentials/auth JSON,
credentials directories, known SSH key names, PEM/KEY/P12 files and private
dot-directories are not browsable/returned. Both legacy and paged remote reads
use this filter; local file discovery is unchanged. Android refuses such paths
in requests, returned entries and draft-reference insertion as defense in depth.
Host admission remains authoritative for manually typed references.

New raw remote prompt attachment references to these private paths fail before
checkpoint ID reservation, journal append or runner submission. Runner-side
prompt_input checks again before resolving attachments. Historical receipts are
looked up first and returned without reexecution, even if their text would now
be refused. Ordinary malformed references retain existing runner validation.
This path policy is not content classification: secrets under arbitrary names
or instructions causing tools to read files still require separate tool,
approval and provider privacy gates. Host-expanded slash resources keep their
existing semantics; raw attachment scanning does not reinterpret them.

`resources_list` is a project Read operation. Initial arguments are exactly `{}`;
page navigation uses exact `{offset,snapshot_id}` with unsigned offset, a
multiple of 32 no larger than the loader's 5120 combined-entry bound, and a
64-character lowercase SHA-256 identity. Returning to offset 0 through paging
also retains the identity; only a fresh open omits it.
Request, project and session IDs are required. Outer write authority, revision,
turn and sequence fields are refused. Current device ACL and session identity
are checked while holding registry admission; the Agent lock uses nonblocking
availability. Both desktop owner mailbox and headless/shared adapter return
only the current admitted resource snapshot, without reloading or journaling.

The exact result has project_id, session_id, unsigned generation, snapshot_id,
unsigned offset, nullable next_offset, entries and truncated. Each entry has
name, kind (skill/template), command, description and
boolean invocable. Explicit Markdown skills use `/skill:name`; prompt templates
use `/name`, including dotted filenames, except reserved built-in slash names.
Skill entries restored as reload-required remain visible but invocable=false.
Template descriptions come only from nonblank explicit frontmatter. Missing,
empty or whitespace-only description yields an empty wire string; the local
template loader's first-body-line fallback is never remotely projected. The
loader retains a private nonserialized explicit-metadata flag, so deserialized
metadata cannot claim permission to expose a fallback. Desktop menu/status
fallback behavior and template expansion remain unchanged.
Descriptions redact credentials and the entry's known source/base/project paths,
remove controls except newline/tab, and truncate at a UTF-8 boundary to 1024
bytes. Source/body/hash/provenance fields are never projected. Arbitrary paths
mentioned in authored descriptions are not a general path-redaction guarantee.
Each page retains at most 32 entries (skills then templates), signals truncated
exactly when next_offset exists, and refuses serialized results above 64 KiB.
The snapshot identity binds project/session, generation, ordered projected
metadata and source provenance/content hashes without exposing those fields.
It detects reload, changed visibility/staleness and restart with the same
generation but changed content. A stale identity fails RevisionConflict before
returning any page. Out-of-range offsets fail InvalidRequest. No full projected
catalog is accumulated; the existing host resource snapshot remains loaded.
ACL and owner checks repeat on every page.
Remote reload and full resource-management parity remain unfinished.

Android performs the read over the existing pinned/authenticated TLS route with
cancellation and connection deadline, checks exact fields/scope/types/count,
then retains one page only in transient foreground state. Next/previous requests
validate exact scope, generation, snapshot identity, requested offset and a full
32-entry continuation; a failure clears the page and requires a fresh open.
Paging preserves the draft and never sends. Choosing an invocable
resource prepends its command to plain draft arguments without sending. Existing
slash/shell drafts and byte-limit overflow cannot be overwritten. Offline,
uncertain-write, lost-control, active-prompt and stale-resource states disable
selection. Background/network/project/subtab/reset boundaries cancel and clear
the catalog. The subsequent normal prompt remains owner/revision/dedup checked;
Agent performs resource expansion and hash validation, so invocable is not a
promise that a resource cannot change between listing and Send. Reads do not
alter encrypted display/cursor caches, send ACKs or add provider calls. An older
host refusing resources_list yields an unavailable state, not an empty catalog.

`model_status` is a project Read operation with empty initial arguments or exact
{offset, snapshot_id} page arguments, required
request/project/session IDs and no outer write authority/revision/turn/sequence.
It checks current Read ACL, matching session and nonblocking Agent ownership
through shared/headless or desktop mailbox. It returns exactly project_id,
session_id, nullable model/reasoning_effort, registry_bound, selection_revision,
nullable capabilities, budget, reasoning_choices and catalog. Capabilities, when known, have the seven
existing provider booleans; budget has unsigned max_tokens/reserved_output_tokens.
Reasoning levels mirror the registry, including ultra. Unknown registry or
capabilities are explicit, not inferred from a model name. Model labels are
bounded/control-free/redaction-checked and path/URL-like values are withheld.
Result is bounded to 16 KiB. No prices/source metadata, backend endpoint,
credential or connection snapshot is returned. Selection revision refers to
the existing model-selection journal event, not workspace revision.

Android uses a separate pinned/authenticated read route, strict fields and
unsigned types, transient cancellable state, and a native model
dialog. Missing capabilities show unavailable rather than false. Refresh is
foreground-only and disabled offline or with unresolved writes; reset joins
the job before wiping keys. Reads never change selection, persist model status,
advance replay cursors or invoke provider. Running locked owners may return Busy.
Budget is the existing effective context allowance, not measured usage.

`model_select` is a Control write with exact {model, selection_revision}
arguments, required project/session/request ID, workspace owner and workspace
revision; no outer turn/sequence. The current ACL is held through admission.
The Agent validates the model against the retained backend, reasoning effort
and existing history, checks the selection revision under its lock, and appends
one complete model_selected event before an infallible model swap. Only
registry-bound backends support this operation; it does not replace the account,
endpoint or backend. Headless active project lanes and desktop pending/admitted
jobs fence new writes before they reach the Agent.

Device/request IDs share the existing bounded checkpoint namespace with Prompt
and all other writes. The legacy prompt_bindings field now stores an optional
operation discriminator (missing means Prompt); ModelSelect and ReasoningSelect
are the added kinds. Restore validates the kind and imports model/reasoning
admission events alongside legacy prompt admissions. A checkpoint reservation precedes journal
reservation and selection. Terminal admission records contain succeeded/failed
and the exact response; an unresolved checkpoint/journal reservation returns
unknown_outcome and is never reexecuted. A bound retry behind a locked owner is
unknown, not a definitive refusal. Current ACL/session checks still apply before
returning receipts; retries return the original receipt even after a later
local selection or owner/workspace revision change. Workspace revision is not
incremented by model changes. Success has exactly request_id, project_id,
session_id, model and selection_revision, with no private snapshot fields.

Workspace listing advertises model_control only for durable Control-authorized
hosts. Android requires that live flag plus a scoped registry-bound model status,
persists the immutable model_select envelope through the existing encrypted
pending-write workflow before sending on authenticated pinned TLS, and verifies
the exact receipt scope/model and a strictly newer selection revision. Fresh
writes require foreground/live network/Connected state; recovery retries use
the same envelope. A native Model ID field and explicit Apply action are absent
for read-only/older hosts and disabled offline/loading/unresolved-write. No
optimistic model success, provider invocation, measured usage
or connection mutation is claimed. Full settings parity and
current-source device acceptance remain unfinished.

catalog has exact snapshot_id, offset, next_offset and entries. Entries are
sorted by UTF-8 identity and contain only id, status (active/alpha/beta/deprecated)
and selectable. Names use the same bounded privacy label checks as selected
models. selectable runs actual retained-backend/effort/history validation;
it is a preview, not admission authority. The selection write still validates
again. Unknown/custom IDs remain available through manual entry, not invented
as catalog items. At most 512 descriptors are admitted; each page retains at
most 16 entries with a multiple-of-16 offset. Empty/non-registry catalogs are
explicit. Snapshot digest binds project/session, selection revision, full
host-local descriptors and validated entries; only the digest and allowlisted
page leave the host. Bound previous-page offset zero is legal; a changed
selection/metadata/validation result rejects old pages with revision_conflict.
No journal/checkpoint/provider side effects from catalog reads.

Android accepts older 8/9-field status shapes with no catalog, validates the
new exact shape/entry fields/unique UTF-8 order/flags/offsets/snapshot identity,
and uses the same pinned cancellable read route for pages. Model dialog adds
catalog selection and previous/next controls; disabled entries cannot be picked,
selection edits the Model ID draft only, Apply remains explicit. Paging preserves
the draft within the same selection revision. Page failure clears the transient
status and requires a fresh read; foreground/network/session/reset boundaries
cancel the shared model job. Model status/catalog is not an offline authority.

`reasoning_select` uses the same Control admission/receipt/state-machine path,
with exact {reasoning_effort, selection_revision} arguments. reasoning_effort is
JSON null for the provider default, or one named registry level; explicit
"none" is different from null and "default" is not a wire value. The typed
Agent checks work fences and selection CAS, validates against its current model
and actual backend wire, appends a complete model_selected event and only then
commits the new effort. Model, backend and account stay unchanged. Its journal
admission kind is remote_reasoning_admission_v1; success has exact request_id,
project_id, session_id, reasoning_effort and selection_revision fields. Receipt
lookup still precedes current revision/busy checks, and unresolved reservations
never reexecute. IDs cannot cross ModelSelect, Prompt, steering or workspace writes.

model_status now supplies a bounded unique reasoning_choices array: null and
at most eight registry level strings. Every choice is validated by the actual
backend, not inferred from a model name or nominal capability. Non-registry
owners return []; a registry-bound wire without explicit reasoning support can
advertise only null. Android accepts legacy status without this field as no
choices, defaults missing reasoning_control to false, and presents only supplied
choices in a dropdown plus explicit Apply. Fresh and exact-retry writes use the
same encrypted pending ledger and foreground gates as model selection; the
receipt distinguishes null from "none" and requires a newer selection revision.
No connection, credential, descriptor provenance or pricing is projected.

The prepared Android/headless smoke chain now independently verifies effective
provider selection: two existing prompts must use gpt-5.2/high then
gpt-5.2-2025-12-11/low, with Android model/effort UI mutations in between. Saved
evidence must contain exact reserved -> model_selected -> succeeded chains,
matching device/project/session/request fingerprint and operation bindings,
selection sequence and actual provider request fields. Selections occur only
after the first terminal prompt and before the second admission; no additional
provider call is permitted. Headless API matrix requires this gate and a
checksum-verified selection PNG. Current-source device execution remains
unverified; compiling the harness or passing its synthetic verifier fixtures
is not proof of LAN/camera success.

Local model and reasoning setters now reuse the owner's existing connection
change fence before validation or journal append. An Idle phase alone does not
authorize a change: staged/active attachments, pending approval, worker admission,
unknown operation outcomes, durable queued input, unserviced input tickets and
input preparation reject it. TUI/headless callers use the same setters; clearing
or explicitly settling pending work permits a later selection. This protects
shared-owner configuration; remote model_select uses it after its durable
reservation, in addition to selection-revision CAS. Host runner admission
that has not reached Agent is separately guarded by the host scheduler.

Headless project lanes are keyed by the request's exact project, not current
selection. There is at most one active job per existing project/session writer
and a 64-lane host bound. Same-project busy/unknown behavior remains unchanged.
Cancellation/steering select only that project's original request/device;
closing an unrelated idle project is allowed while another project executes.
Host-owned admission job IDs increment across all lanes; durable exact retry
still returns the original receipt. Each captured root is revalidated through
the same SubtabExecution adapter. This is cross-project parallel execution,
not independent-worktree per-tab quota execution (S2-018) on its own, and does
not change the TUI's master runner topology. Later worker adapters and idle
quota controls are separate from these project lanes.

The desktop owner mailbox accepts `prompt` with exactly `{text:string}`,
at most 64 KiB, explicit project/session, workspace owner/revision and request
ID. Read/Control ACL is checked under the registry's revoke lock. Idle admission
uses the existing desktop BackgroundRunner and existing project Agent, not a
parallel network executor. The runner uses StartIfIdle, retains attachment/
resource handling and continues after the connection closes. Explicit `!`
local-shell input is refused. Admission captures the selected host subtab's
stable ID, kind and canonical root in the host-only prompt journal receipt.
The worker revalidates that selection and executes the ordinary Agent pipeline
inside a request-scoped tool workspace. Main roots must match the project;
directory roots must remain contained canonical directories; worktrees must
still be registered with that project. Missing/replaced/symlink roots fail
instead of falling back to main. The same wrapper is used for local desktop
model/shell jobs. It restores the original ToolContext on success, failure or
unwind without replacing session, approval policy or coordinator. Builtin
handlers remain the same; extension handlers receive fresh scoped leases.

Worker grants and active raw captures cannot transfer to another root. Loaded
extensions are reinitialized against the selected canonical workspace using
the already-loaded manifest catalogue, not an implicit extension reload. Both
the selected generation and a fresh original-workspace restoration generation
must initialize and register successfully before the original lease is revoked.
Preparation failure or cancellation leaves the original registry/root/lease
untouched. Success journals the workspace/generation binding, revokes the old
lease and brackets execution with session-close/start notifications. Restoration
revokes the selected lease and publishes the prevalidated original-workspace
generation even on return/error/unwind; a closed Agent is not reopened. Cleanup
notification errors are isolated as ordinary extension errors. The desktop
cancellation token also covers initialization; cancelled child processes are
killed and reaped. No fresh manifest discovery or new permission grant occurs
at this boundary. Native extensions remain trusted executable code, not an OS
filesystem sandbox. This does not claim full headless parity, per-worktree
concurrency, full media/resource scope parity or every extension workflow.

The Agent's sole journal writer persists a fingerprint-bound reserved receipt
before runner submission, then accepted/failed admission. Accepted responses
contain request/job/project/session IDs; they mean runner admission, not provider
completion. Exact retry returns the receipt without submitting again, including
after journal restoration. A reservation without final admission remains
unknown_outcome and never auto-starts. Busy execution locks return unknown_outcome
because an earlier attempt may already have submitted; safe pre-admission Busy
is failed. The journal bounds retained admission records to 256 events.
Before journal reservation, the owner checkpoint durably binds a prompt's
device/request ID to its exact envelope fingerprint and project/session. This
128-entry non-evicting index fences reuse as workspace/subtab, cancel, approval
or another project's prompt even while the Agent is locked. It does not retain
message text. A binding without journal admission returns unknown_outcome and
never resubmits. Reservation CAS failure also prevents runner submission.
Restoration imports legacy admissions from the saved current session journals,
including when no checkpoint exists; malformed/conflicting/capacity-exhausted
imports fail closed. Bindings survive project close and session replacement,
but pre-upgrade admissions in historical journals absent from the saved session
mapping cannot be retroactively recovered by this migration. There is no
claim of an unlimited global ledger or automatic recovery of lost storage.

Android uses its existing encrypted pending-envelope mechanism for prompts,
preserving exact request ID/text before sending. Accepted receipt shape and
identity are checked before authorized workspace/subtab refresh. Every supported
write now uses a separate receipt-confirmation boundary. Workspace/subtab
receipts require exact fields, matching request ID/owner, bounded numeric
revisions incremented once, valid active identities and target project.
Prompt/cancel/approve admission requires matching request/project/session and
job/target/digest/decision as applicable. Only after validation is encrypted
pending state durably cleared; display authority is stripped until a fresh
read succeeds. Later projection/replay failure does not recreate pending state
or resend the write. Foreground refresh retries reads only. Missing, malformed,
unknown or interrupted receipts retain the exact envelope. Failure to persist
local confirmation also retains it for explicit exact retry. For prompt and
cancel, that refresh resolves admission without acquiring the busy transcript
owner. Foreground snapshot/replay then runs separately. If no matching admission
receipt is received, the pending request remains until an explicit same-envelope
retry. Cached project display drops prompt_control along with other authority.
No background worker flushes it. Desktop workspace listings now include up to
16 device/project/session-scoped prompt_statuses, with exact request IDs and
accepted/running/succeeded/failed/cancelled/unknown_outcome states. A bounded
live projection remains readable while the provider holds the Agent lock.
Its bounded 128-entry display cache orders requests by their most recent
status update rather than random request ID. Active accepted/running records
are retained ahead of terminal history; capacity eviction removes only the
oldest terminal/unknown display record, never admission/deduplication receipts.
Listings prioritize active work, then recent live records, and fill remaining
slots from reverse journal order, deduplicating latest request status. Session
identity is part of live retention, so an old session cannot replace current
status. The desktop currently admits one global foreground runner job; this
display cache is not a complete multi-runner retention/admission policy.
Headless completion now retains one typed terminal result per existing active
lane when the Agent mutex is temporarily held by a reader. The lane remains
admitted/busy until its owner can attempt terminal persistence; each owner-loop
pass uses try_lock and continues servicing the bounded mailbox, never blocking
all projects or resubmitting execution. A WouldBlock makes no append or terminal
publication; the previous active display remains until persistence resolves.
Once an append is attempted, success publishes the durable result and an append
error/poisoned owner publishes unknown, retires the lane and is never retried
automatically because the append's disk outcome may be ambiguous. Shutdown
retains a consumed completion and uses one shared runtime join deadline for all
lanes (default one second), followed by one shared 250ms owner-lock acquisition
window. Persistence sweeps all pending owners, so a held project does not hide
another free project's result. A short reader can release during this window;
an append is still attempted at most once, never retried on IO error. These
deadlines bound cooperative runtime/lock waits, not a kernel journal sync that
cannot be preempted. Owners still held at the deadline cannot be claimed durably
resolved. Detached/cancelled shutdown results remain unknown even if the job
thread later returns; it is never resubmitted or relabeled successful.
The runner records running and terminal progress through the same journal
writer; fresh desktop runtime restoration preserves terminal records but
classifies nonterminal durable records as unknown rather than inventing resumed
execution. This is pull status synchronization, not provider delta streaming.
Android validates exact status shape/owner and cache v4 retains terminal display
while converting accepted/running to unknown offline; v1/v2/v3 remain readable.
Failed prompt status may now contain optional `failure_code`, using only the
existing wire ErrorCode enum. Invalid input/closed turn maps to invalid_request,
non-idle owner to busy, tool-loop/governance limits to resource_limit and other
execution errors/panic to internal_error. Raw Agent/provider error messages,
paths and credentials are not copied into this diagnostic field. TUI and
headless terminal writers persist it with progress before live publication;
confirmed headless shutdown failures retain the same code. Failed persistence,
uncertain shutdown and cancellation do not manufacture a confirmed failure.
Restored status reads deserialize only known enum values and preserve existing
device/project/session boundaries. Old status records without the optional
field remain readable by the current Android client; older strict clients are
not claimed compatible with new failed-status fields. The code is display-only,
not admission authority or permission to retry a provider call.
Authorized workspace listings also expose at most 16 pending_approvals per
project from the existing ApprovalCoordinator. Each bounded record has
request/project/session/turn/call IDs, tool name, host-derived side_effect and
origin, and a SHA-256 digest of the complete host request. An optional diff
preview contains kind/path/patch/changed/truncated/redacted/withheld. The host
redacts registered secrets and conventional assignment/header/JSON credential
values, removes control characters other than patch newline/tab, and caps the
patch at 2048 UTF-8 bytes after redaction. Credential paths (including .env,
.ssh, .aws, .gnupg, credentials/auth JSON and key/PEM files) and private-key
blocks are withheld rather than transmitted. This is a defensive heuristic,
not a guarantee that arbitrary unregistered secrets can be recognized.
Raw arguments, policy/lease details and source-file digests remain omitted.
The request digest binds the original complete request, not the display patch.
Reading does not reveal or
drain desktop prompts, answer them or acquire the Agent execution lock. The
digest is streamed into the hash rather than allocating a full request copy.
Answered/cancelled requests disappear on the next listing. Android validates
exact fields, identities, enums, digest shape, uniqueness and capacity. The UI
shows an awaiting-approval summary, with confirmed one-shot Allow/Deny only
when the host advertises separate approval_control authority.
Pending approvals are intentionally omitted from offline cache, and offline
views do not display a saved pending decision target. `approve` accepts exactly
request_id/digest/decision (allow or deny), plus envelope request identity,
project/session and workspace owner/revision. The registry's Approval capability
is checked independently of Control; pairing grants both on the mounted project.
The coordinator compares the full request digest and claims the answer under
one lock, so a local answer or different request cannot be overwritten. Remote
answers always have remember=false and no policy grant/message. The worker's
existing approval audit-before-tool boundary remains responsible for side
effects. Accepted means decision admission, not successful tool execution.
The authenticated adapter passes a host-built remote actor through the same
atomic coordinator claim: device ID, envelope request ID/fingerprint, project,
session and original request digest. The worker's approval_resolved journal
record persists this optional remote object before tool execution. Local
responses and host cancellation carry no remote actor; caller-supplied tool
arguments cannot inject one. This is attribution, not a policy grant. A lost
coordinator race cannot replace the winner's actor, and audit persistence
failure retracts the accepted decision/actor and restores the pending request
even when no local renderer revealed it. A request_response caller may already
have left its wait, so this does not promise automatic worker continuation
after a storage failure. The existing failed turn still prevents tool dispatch.
An owner checkpoint reserves unknown before answer delivery, then commits
accepted/failed; exact retries return that receipt across owner reconstruction
without reanswering. Interrupted reservations stay unknown and never auto-send.
The 128-entry approval ledger validates identities/digest/state/code and rejects
workspace/cancel/prompt ID collisions. Broader sensitive-preview coverage, historical cross-operation ID
binding and killed-host/race/storage-fault acceptance remain required. The
production PTY/Android harness now drives two real HTTP provider write_file
calls and confirms Allow/Deny on Android over pinned TLS. It checks Allow file
content, Deny file absence, matching checkpoint/journal request identities,
remember=false, host_answer source, exact remote actor/checkpoint correlation
and an Allow-only consumed record with matching target request ID. This is
bounded tool acceptance, not the full preview/tool/race/cancellation matrix.
The confirmation also renders and asserts the live built-in write_file path
and diff before either decision; cache serialization excludes both preview
and decision authority. Preview display does not grant Approval capability.
The desktop owner mailbox also accepts `cancel` with exactly
`{request_id:string}` targeting the original prompt. It requires the same
device/project/session as the active remote prompt, plus current workspace
owner/revision and Control ACL. It calls the existing runner's exact active job
ID; local and other-device jobs are not targetable. Accepted responses bind
request/target/project/session IDs and mean cancellation admission, not a
terminal outcome. A bounded 128-entry cancellation ledger in the existing owner
checkpoint returns an exact retry without cancelling again, including after
desktop restart/owner-epoch replacement, and rejects changed-envelope reuse.
Control ACL/revocation is checked even on retries. A fingerprint-bound
unknown_outcome reservation is atomically persisted and synced before touching
the runner; accepted/failed admission is committed afterwards. Checkpoint CAS
or uncertain execution failure retains ambiguity and never re-dispatches a
reserved request. Legacy checkpoints default to an empty cancellation ledger;
malformed/duplicate/oversized records fail restore. Workspace/subtab and prompt
admission reject cancellation-ID reuse, and cancellation rejects workspace,
approval and checkpoint-bound prompt IDs. New supported writes share a
device-scoped identity namespace; historical migration and future operations
still need complete coverage before calling it a universal ledger.
Missing active identity on a new request fails closed instead of targeting a
replacement turn. Cancellation receipts do not advance workspace revision.
Cancellation-before-worker/panic reconciliation,
live provider delta/ACK, cross-operation/global request-ID ledger, peer feature
negotiation and interrupted-admission operator recovery remain open.

## Desktop Steering Admission

The desktop mailbox accepts `steer` with exactly
`{request_id:string,text:string}`. The target must be the authenticated device's
still-active remote prompt in the same project and session; owner epoch,
workspace revision and Control ACL are checked before a new admission. Text
must be nonblank, at most 64 KiB, without NUL, leading slash/shell commands or
explicit @ file tokens at Rust whitespace boundaries. Bare/incomplete/quoted
references are refused, not silently treated as attachments or literal provider
instructions. Inline paths/commands and email addresses remain ordinary text.
This text-only check runs after current ACL and exact receipt lookup, before
new owner/queue admission or checkpoint reservation. Previously saved exact
receipts remain resolvable without requeueing now-refused text. Android rejects
new incompatible steering before persisting its intent and disables Send while
retaining the draft. Its whitespace table matches Rust, including NEL/NBSP;
saved legacy envelopes still restore for exact receipt resolution. Local TUI
input enqueue/edit uses the same Rust text-only check even without a catalog.
Idle/scheduled prompts retain file and skill/template resolution. Direct core
input-queue protocol is unchanged; this is a host-admission policy, not new
resource expansion inside the queue.
Steering a local desktop prompt or another device's turn is not yet supported.

A non-evicting 128-entry `steer_receipts` ledger in the existing owner checkpoint
reserves `unknown_outcome` before input submission. The request shares the
device-scoped namespace with prompt, workspace/subtab, cancel and approval
receipts, including restore and legacy prompt-journal import. The existing
session `InputPort` binds submission to its current turn and the sole journal
writer persists the `InputKind::Steer` input. No new runner or journal writer is
created. The desktop retains the ticket and polls it without blocking its owner
thread. Only successful durable queue acknowledgement can commit `accepted`;
that response contains exactly request, target-request, project and session IDs.
It means durable queue admission, not provider completion or immediate input
application. The existing core applies steering at its defined input boundary.

Exact retries return the stored state before checking the current owner/revision
or active turn, but still require current ACL. They never requeue, including
after restart, target completion or replacement. Scope expiry or an ambiguous
journal failure remains unresolved rather than assuming the input was absent.
A rejected port submission can commit terminal `failed/busy`. Checkpoint failure
never becomes a successful receipt. Android polls an unresolved steering receipt
with the same envelope at 150 ms intervals, at most ten retries within its
existing socket deadline; persistent ambiguity retains the encrypted envelope
for explicit retry. Automatic lifecycle/WorkManager resumes still never write.

Full remote/headless/TUI parity, local-turn steering policy, interrupted
admission recovery and the broader race/tool/API matrix remain unfinished.

Operations are inspect, workspace_list/open/select/close, subtab_list/add/select/
move/rename/close, set_concurrency, prompt, steer, approve and cancel. Only
inspect/workspace_list/subtab_list are reads. All writes require an ID, expected owner and
expected revision even when revision is zero. Client-supplied flags cannot turn
a write into a read. The authenticated host owner must independently verify
device capability, owner epoch, revision and canonical paths, and durably bind
request ID to request content/outcome before dispatch. Structural validation
does not implement those checks. Ambiguous completion is `unknown_outcome`,
never automatic success or permission to repeat a side effect.

Errors: invalid_request, unsupported_version, unauthorized, revoked_device,
owner_mismatch, session_changed, revision_conflict, request_id_conflict, busy, replay_gap,
resource_limit and internal_error. Responses require ID and execution state.
Failure responses require a code. Provider credentials are never wire fields.

## Acceptance Evidence

The additive write operation `resources_reload` reloads only resource paths
already selected by the host Agent. Arguments are exactly
`{generation,snapshot_id}` from a freshly read resources_list catalogue;
project/session, current workspace owner/revision and device Control grant are
required. The snapshot digest prevents generation reuse after a process restart
from authorizing a different catalogue. Busy prompt/worktree/input owners refuse
new reloads. No client path, resource body, extension executable or credential
is accepted by this operation.

The existing owner checkpoint binds request ID/content across operations,
followed by a journal reservation before reload. A completed response contains
only request_id/project_id/session_id and the increased generation. Exact
retries return the saved receipt even after owner restart; a reserved or
ambiguous attempt remains unknown_outcome and is never executed again. Reload
uses the Agent's normal candidate/provenance publication and trusted-extension
reload logic, with a one-second cooperative cancellation deadline. Filesystem
or extension operations that do not honor cancellation are not covered by a
hard real-time guarantee.

Workspace listing advertises `resources_reload_control` only with a durable
checkpoint and Control permission. Android sends it only from a connected
foreground resource catalogue, stores the exact pending command before send,
validates scope and an increased unsigned generation on the receipt, and keeps
ambiguous writes for explicit same-request retry. The capability and resource
catalogue are not restored as offline authority. The resource dialog provides
an icon control and closes while the write is unresolved; reopening reads the
new catalogue. Desktop completion metadata is refreshed from the same Agent.
Domain/JVM evidence does not establish new-APK TLS/LAN or device acceptance.

Ordinary transcript turns may additionally contain paired fields `tool_calls`
and boolean `tool_calls_truncated`. Each public summary has exactly
`id/name/arguments_preview/arguments_truncated`: eight calls maximum, 512
UTF-8 preview bytes per call, redacted before slicing. Host execution, policy,
credential and worker-binding metadata are not forwarded. The projector is
shared with worker history. Malformed legacy metadata is wholly omitted from
ordinary replay, so it neither leaks raw metadata nor breaks cursor progress.
Android accepts these fields only on assistant turns, validates unique IDs and
bounds, includes summaries in its encoded display-tail budget and omits them
from offline cache. Older clients can ignore the additive summaries.
New v4 cache writes retain only assistant-turn `tool_calls_omitted: true` when
such summaries were removed. This marker contains no invocation ID/name/argument
or authority. After restore, HostClient takes an authenticated fresh snapshot
for that display before resuming event-tail reads; a tail alone cannot recover
previously omitted summaries. Plain v4 caches continue cursor replay. Since
v1/v2/v3 writers did not consistently mark omissions, decoding any legacy
assistant turn conservatively sets its local omission marker. User/tool turns
are not marked. The original cached bytes, text and cursors are preserved on
read; a later write migrates to v4 without clearing the recovery requirement.
After a fresh snapshot, normal event-tail reads resume. Earlier marked v3
records remain accepted; v1/v2 still reject the additive marker field.

A typed Busy response to initial resume or required inspect may fall back to
the already retained same-project/session cursor only for a read-only foreground
subscription supported by the current authenticated listing. Fresh listing/
subtab metadata must still pass freshness gates. It subscribes from the exact
old cursor, without claiming transcript catch-up or creating a new cursor.
Cached summary omissions survive until a real snapshot replaces them. No
first-attach empty transcript is manufactured; missing/foreign cursors, writes,
unadvertised subscriptions and non-Busy refusals remain failures. Matching error
text alone is not sufficient to admit this fallback.

Display cache v4 caps combined offline prompt-status history at 256 records,
giving selected-project statuses priority without changing project order,
transcript text, cursors or live listing behavior. If long IDs/history plus
the transcript exceed the cache byte cap, lowest-priority status tails are
removed until the encoded record fits. JSON UTF-8 bytes and array separators
are counted, not just string lengths. If the record still exceeds the cap
without any statuses, it is refused rather than dropping displayed messages
while advancing their cursor. PairedHostStore checks its
entire plaintext record using FrameCodec before encryption/atomic write: a
separately valid listing and snapshot must also satisfy combined restore
complexity/scalar limits. Rejected writes preserve previous ciphertext rather
than publishing a record that the next cold start cannot read. Exact unresolved
write/revoke identities are separate records and are not trimmed by this cap.

`tests/remote_host_e2e.rs` now includes a production-binary test for real HTTP
Responses provider calls, pinned TLS prompt admission, pre-approval write
refusal, live preview, remote approval, actual write_file result continuation,
cancelled streaming request, exact retry/conflict, durable replay, clean exit
and owner restart. It is explicitly ignored in the offline suite. Running its
ignored case in the current sandbox fails on HTTP loopback bind with EPERM
before the host starts; compile success and fixture unit tests are not a passed
production or Android/LAN gate. Run in a network-capable environment:

```sh
cargo test --locked --test remote_host_e2e production_prompt_tool_approval_cancel_replay_and_clean_restart -- --ignored --exact --test-threads=1
```

Authorized desktop workspace entries may include nullable `desktop_terminal`,
a read-only screen projection of an already-existing PTY. Exact fields are
project_id/session_id/subtab_id/subtab_revision/text/truncated/read_only, plus
an optional 64-hex terminal_id for the current PTY instance; no root, PID or raw
input is projected. The TUI emits
only for its current project, captured PTY session and matching visible/host
active root. New project/session at the same cwd requires a fresh local PTY;
the old screen cannot be relabeled. Reads never open a shell. The latest 64
screen lines are redacted and control-filtered, sensitive-marker-bearing lines
are wholly withheld, and UTF-8 text is capped at 16384 bytes with a truncation
flag. Known project/tab roots are replaced before transmission. This is best-
effort redaction, not a promise to detect arbitrary unregistered secrets or
other absolute paths in terminal output.

Android requires exact owner/session, stable-ID subtab, unsigned revision,
boolean read_only=true and text/flag bounds. It displays only when connected
and the active subtab ID/revision still matches. It caches neither the terminal
field nor its text and never moves the event cursor for screen output. Headless
has no terminal screen and cannot create a shell through these endpoints.
This screen uses authenticated listing polling, not bulk paging or a PTY event
subscription. Production PTY domain tests pass, but Android/TLS/physical-device
screen acceptance remains pending for new artifacts.

Current instance-bearing screens may also include optional size, exactly
{rows, cols}, with integer dimensions 1..512. The TUI reads actual kernel PTY
geometry (TIOCGWINSZ), not its last requested desktop size or a guessed default.
Failed queries, zero or out-of-contract geometry omit the field rather than
inventing dimensions. Size reads do not resize, open a PTY, journal an effect
or confer Control permission. Android accepts legacy absence, rejects malformed
or instance-less size metadata, and synchronizes resize fields when actual
geometry changes. Output-only refresh preserves edited size/input drafts;
geometry changes reset only size drafts. Geometry is transient and excluded
with the terminal from offline cache. Existing instance/tab/session fences and
durable input/resize admission remain unchanged.

The desktop PTY reader now bounds raw output in a 32-chunk FIFO (8192 bytes
per chunk, 256 KiB queued plus one pending reader chunk). A full queue applies
backpressure to the slave instead of dropping or reordering bytes. Stop cancels
a full-queue wait before joining the reader. Each TUI pump processes at most
eight chunks (64 KiB), leaving the rest for later ticks; this is a byte budget,
not a hard wall-clock deadline. Incomplete CSI/OSC sequences retain at most
4096 bytes. Oversized incomplete controls are discarded until their terminator,
including split OSC ESC/backslash terminators, rather than being rendered as
ordinary text. Existing 4000-line/512-column parser and remote redaction/history
bounds remain. This is host-side resource control, not Android/LAN acceptance
or complete terminal emulation.

TUI workspace listings advertise terminal_control only for an existing matching
PTY, a durable workspace and the device's current Control grant. The screen
itself retains read_only=true: projection does not confer input authority.
terminal_input accepts exactly terminal_id/subtab_id/subtab_revision/text;
terminal_resize accepts exactly terminal_id/subtab_id/subtab_revision/rows/cols.
Both require project/session/request, owner epoch and workspace revision, and
reject turn/sequence fields. Input is nonempty UTF-8, at most 4096 bytes, without
NUL; other terminal control bytes are intentional input. Dimensions are 1..512.
Input/resize route only to the already-owned matching active shell. Reads never
start a shell. Modal/pairing/admission Busy gates apply.

terminal_open is a distinct Control write with exactly subtab_id/subtab_revision,
plus project/session/request, owner and workspace revision. Only the desktop TUI
advertises terminal_open_control for its current project and a durable workspace.
It creates the active subtab's desktop-owned PTY when none exists, or returns the
same matching instance. It never replaces an unrelated existing shell. Busy
jobs, worktree tasks, modal/pairing state and stale scope refuse fresh creation.
The host journals reservation before spawn and completion with the terminal ID;
exact retries resolve that receipt without spawning, even if the shell has since
disappeared. Ambiguous reserved creation remains unknown_outcome. The plain
headless adapter does not advertise creation and refuses new creation as busy.
No cwd, shell program or command can be provided by Android.

Current instance projections include an optional boolean running flag. The host
observes only its owned child with nonblocking waitpid; exited instances retain
their bounded output/history but cannot admit new input or resize. The child is
reaped once and its PID is never signalled after reaping. Exit changes wake the
subscription even when no output arrives. Android hides input/resize and shows
Terminal exited; explicit terminal_open may create a new instance at the same
scope. The old request's retry still resolves its old receipt, never reopening.
Reads and desktop redraws do not respawn the exited shell. Legacy projections
without running retain legacy behavior until a new host is used.

Current authorization, session, active subtab/revision, instance ID and cwd are
checked before admission. The checkpoint binds the request fingerprint and a
reserved journal receipt precedes any PTY effect. Exact retries resolve the
durable result even after host restart or shell replacement; conflicting IDs
fail. Incomplete reserved effects remain UnknownOutcome and never run again.
The receipts contain hashes and scoped byte-count/dimension results, not input
text. A succeeded input receipt means admission to the bounded PTY write queue,
not successful command execution; output and effects must be observed separately.
The remote queue adds at most 4096 bytes per operation and refuses growth beyond
16384 queued bytes. Resize uses checked ioctl; it persists until desktop pane
geometry changes, rather than being overwritten by an unchanged redraw.

Android persists the exact pending request through the existing encrypted write
workflow. Fresh controls are foreground/connected only; unknown outcomes retain
the same request and never auto-create replacement input. UI sends explicit text
with optional Enter, special keys and row/column resize. Control and terminal
identity never enter the display cache. True pixel/cursor/ANSI terminal emulation
and current connected-device acceptance remain incomplete.

terminal_read is a Read-only request with exactly terminal_id/subtab_id/
subtab_revision/offset/digest. It requires project/session/request IDs and
forbids write owner/revision, turn and sequence fields. The TUI resolves only
its existing matching active PTY; headless rejects missing terminals. The host
checks the current Read grant, session, active subtab revision, instance and cwd
before accessing the parser's retained screen. It never opens a shell, pumps
input, reads raw PTY bytes, or changes journal/checkpoint/event/ACK state.

Android TerminalReadPolicy is shared by the history entry, dialog and ViewModel
read/back/forward paths. It requires connected current unique project/session/
active-tab/terminal scope and rejects scanner/reset/loading/pending write/revoke
states. Navigation additionally requires an open successful scoped page and
valid bounded retained history chain. Read-only and exited-terminal projections
remain eligible without input/resize capability. Failed or stale pages may
refresh when the current terminal is still authorized, but cannot navigate.
The terminal codec rejects missing/duplicate active tabs before constructing a
request. Policy JVM tests execute these boundaries; extended Compose cases
compile only. The current PTY interop lane also checks policy against its real
response pages using a Read-only client projection, not a device UI run.

The current PTY interop lane now also lowers the actual fixture registry grant
to Read, authenticates at that revision, obtains its host workspace listing,
and dispatches all history pages under that Read-only identity. Fresh Kotlin
input/resize requests are refused by Rust with unauthorized; Kotlin decodes
those refusals and cannot construct controls from the Read-only projection.
Foreign-session reads fail, and revocation refuses the retained read request.
History reads and denied controls leave the owner journal/checkpoint and prior
PTY effect count unchanged. This is real host ACL/PTY and JVM frame evidence,
not production TCP/TLS, Android Keystore or physical LAN.

The retained maximum 4000 parser lines are reassembled using existing soft-wrap
metadata and sanitized before hashing/slicing. Raw and redacted documents are
capped at 8 MiB, pages at 8192 UTF-8-aligned bytes. Result fields are exact
project_id/session_id/terminal_id/subtab_id/subtab_revision/digest/offset/
next_offset/total_bytes/has_more/text. Nonzero offsets require the SHA-256 digest
of the full redacted document. Output changes cause RevisionConflict on
continuation, never mixed snapshots. This is retained screen history, not an
unbounded command log or proof that evicted output remains available. No
server-side frozen copy or silent restart is created.

Android opens a selectable monospace history dialog, keeps at most seven prior
pages and offers explicit previous/next/refresh/close. Refresh starts from zero;
page errors disable continuation but keep the last verified page visible.
Prior terminal pages must independently satisfy digest/offset/total/UTF-8 byte
and progress/control-character checks before continuation requests or response
decoding. The complete retained chain is validated before both back and forward
navigation, including its seven-page bound; corrupted same-identity history
cannot authorize a read. Explicit Refresh still starts from zero.
Normal output updates retain selection so a subsequent read can report digest
conflict; missing/replaced terminal, project/session/subtab changes, authority,
background/network and pending writes clear the reader. No history is cached.
Sanitization is best-effort: shell-generated hard wraps or cursor rewrites can
separate a secret from its identifying marker. Unknown or unlabelled secrets
are not guaranteed detectable. History access should be granted only to trusted
devices. Current domain/source tests are not device/LAN acceptance.

Remote-owned prompt statuses may carry optional `stream_text` while accepted or
running. This is a transient, cumulative provider-text preview scoped to the
exact device/project/session/original request, not a durable transcript event.
Desktop and non-TUI production provider callbacks feed the same bounded buffer
without taking the Agent lock. At most MAX_PROJECT_TABS (currently 64) active
previews of 4096 raw UTF-8 bytes are retained, without active-slot eviction.
Overflow freezes the previous publishable prefix and adds optional boolean
`stream_truncated: true` alongside `stream_text`; an oversized first chunk
produces an empty prefix with truncation, not a sliced incoming line. NUL
withholds that preview; terminal progress removes
it and a subsequent provider response resets it. Reasoning, tool arguments,
warnings and credentials are not projected as event objects. Text is published
only through complete lines (or TextDone), then the complete retained prefix is
redacted, so a split credential-bearing line is not exposed midway through its
value. This conservative line boundary can delay single-line answers until done.

Android checks exact status fields/identity/state and the same text byte cap;
truncation must be boolean true, accompany text and belong to an active status.
It displays an explicit truncation indicator. Older clients that do not
understand this optional field fail closed rather than interpret a partial
preview as a complete result. Desktop-local preview continues to withhold
overflow because its existing string field has no truncation metadata.
Android
displays the preview only in online/error live states, and drops it from encrypted
display/cache records. No cursor or ACK advances because of partial text. Final
durable turns still replace this preview through ordinary transcript replay.
These previews currently use the existing authenticated foreground listing
poll, not unsolicited event-channel push. Local discussion-model requests on
the desktop now have a separate optional nullable `desktop_stream_text` field
on their authorized workspace entry. It uses the same text cap/redaction and is
scoped to the captured original project and actual worker session. A worker
guard clears it on success, failure, cancellation or unwind. It does not create
a remote request identity/status or grant cancellation/steering rights. Arch,
shell and resource-control jobs are not projected through this model-text field.
Android drops it from the cache and displays it separately while online/error.
These callbacks are wired but current-device/TLS/Compose acceptance remains
pending for the newer artifacts. Full streaming transport, replay/backpressure
and broader local/remote parity cases remain required.

`cargo test --locked --test remote_protocol` covers version refusal, strict
fields, enum refusal, owner/revision requirements, correlation, cursor/session
binding, replay bounds, ID/JSON/channel caps and existing local version values.
Remote byte decoding preflights every object before typed deserialization:
duplicate members, including escaped spellings of the same key inside dynamic
arguments/result/data/state, are rejected rather than silently overwritten.
The wire preflight allows at most 8192 JSON nodes and depth 40; existing dynamic
payload limits remain 4096 nodes/depth 32. This matches Android's preparse and
payload caps. Names repeated in distinct objects are legal. Direct typed
`serde_json::from_slice::<Envelope>` is not the remote ingress API; callers must
use `Envelope::decode` or `read_frame`. Programmatically constructed envelopes
still validate typed semantics without redundant serialization/parsing.
`cargo test --locked --test remote_frame` covers network byte order, concatenated
frames, fragmented/interrupted reads, every truncation position, preallocation
length refusal, malformed UTF-8/JSON, exact channel boundary, output refusal
before prefix, broken write/flush and real loopback read timeout.
`remote_faults` adds deterministic injected timeout/reset/would-block reads,
clean versus partial EOF and partial/zero-progress writes. These do not substitute
for socket faults, TLS, address change or replay-gap acceptance. In the current
restricted environment the real loopback timeout test fails to bind with
PermissionDenied, while the eight non-socket frame tests pass with that one test
explicitly filtered. The failure remains an unfulfilled network gate.

The shared `fuzz/remote_protocol/corpus.jsonl` now supplies raw-wire cases to
both Rust remote_protocol and Android RemoteProtocolCorpusTest. It must not be
preparsed/reserialized before ingress decoding, which would erase duplicates
and integer token spellings. Android preserves raw numeric tokens, refuses
decimal/exponent/negative-zero unsigned control fields, null failure codes,
non-finite numbers, unpaired UTF-16 surrogates and leading UTF-8 BOMs. Its UTF-8
writer also refuses malformed programmatic Unicode before emitting a prefix.
Pairing public-key base64url must be canonical in both codecs; structural
acceptance still does not prove a valid curve point or signed device identity.
Nullable optional routing fields use the same absent semantics as Rust.

Both suites run 2048 bounded deterministic mutations and verify accepted
values roundtrip; exact payload node/depth boundaries are accepted and one
more is refused. Rust frame tests also inject deadlines at every prefix/body
offset and exercise fragmented multi-frame output/flushes. This is bounded
regression fuzz replay, not coverage-guided libFuzzer evidence.

These tests prove only the tested codec acceptance/roundtrip interoperability,
not TLS authentication, replay storage, LAN pairing, TUI parity or host dispatch.

Additional tests: `remote_listener` exercises real TLS frames and negative
certificate/timeout/cap/shutdown behavior; `remote_pairing` covers private
persistence, symlink/public file refusal, invite parsing, atomic one-use
consumption, refresh/cancel and expiry. `remote_tui_pairing` independently
decodes the terminal QR pixels for IPv4/IPv6 and runs the production binary
through TLS negotiation, acceptance and repeated-token refusal. These establish
Rust-side pairing transport, not Android camera scanning, persistent device
authorization, provider/tool dispatch or desktop/Android synchronized sessions.

## Bounded Foreground Transcript Subscription

New hosts advertise boolean `event_subscription_available` in workspace_list.
An older host without the flag keeps Android on the existing resume path.
After authentication and snapshot/replay, the foreground client may send a
control message `subscribe` with an ID, project_id, session_id and payload
`{type:"subscribe",cursor:{session_id,next_sequence},limit,duration_ms}`.
Limit is 1..128 and duration_ms is 1000..5000 (Android currently asks for 3000).
Owner/revision/turn fields are forbidden, as for read-only replay. Schema 1's
strict decoder recognizes this additive capability only; old decoders refuse
the new message, so the client must not send it without advertisement.

One request yields repeated existing event/snapshot frames followed by a
correlated successful response per batch. Replay result fields retain their
existing semantics, plus boolean `subscription_done`. The last batch has true.
The host checks the same real owner and current device Read grant each batch;
the listener rechecks current device identity before every frame. The host
internally samples that owner every 250 ms, not a second session state machine.
No journal mutation, provider execution, cursor ACK or queue worker is created
by subscribing. This is bounded server-driven delivery after one client request,
not a provider-callback stream or an indefinite background connection.

Total event/snapshot frames are capped at 128. All serialized JSON bodies,
including each event/snapshot envelope and terminal control response, share a
196608-byte subscription budget matching Android's reader. Nonterminal batches
reserve space for the longest correlated v1 failure response; a later refusal
remains readable within that budget. Prefix bytes are outside the body budget.
Overflow refuses before emitting any frame in the oversized batch. Replay
tail/gap/snapshot boundaries end the subscription for the existing resume path.
Busy, authorization refusal and output failure stop it, without automatic write
retry. The connection has a fixed seven-second socket deadline for the entire
subscription; successful frame output cannot extend it. Listener shutdown
still owns and closes the socket. There is no unbounded output queue.

Android's reducer bounds total frames/bytes and 32 batches, enforces contiguous
cursor and scope via the existing replay reducer, rejects malformed completion
or a nonterminal gap/tail and retains the prior display on refusal. Foreground
ViewModel persists each changed display/cursor before publishing it. ACK stays
on the next authenticated read, and only for a successfully saved cursor.
Cancellation/network/background/reset still fence generation and close sockets.
The foreground ViewModel also refuses pure refresh and diff reads while not
foreground or while local reset is active/required, before touching view state
or launching a connection. Background opportunity reads use the separate
bounded ResumeWorker and its encrypted-record compare-and-set; they do not
borrow retained UI callbacks or send unresolved writes. Headless Android
acceptance requires both write-fence and read-fence phases before reattach.
WorkManager does not subscribe. See the transient view extension below for
prompt/approval/PTY/workspace display updates on this bounded connection.

Current verification includes Rust subscription/domain/ACL tests and Android
JVM reducer/strict-codec tests plus direct source compilation. It does not yet
prove a newly built APK over real TLS/LAN or foreground streaming screenshots.

### Transient View Extension

Each bounded subscription batch now has boolean `subscription_transcript`.
True retains the usual replay fields. False means the owner was Busy after an
authorized view read: no event frames, no invented journal/phase fields, and
the exact prior transcript cursor with has_more/snapshot_required both false.
It is display availability, not successful transcript synchronization or ACK.
Android now explicitly refuses phase or journal_next_sequence fields on a false
transcript batch, including null fields, and requires the retained projection's
selected project/session to match the subscription before any view can publish.
Contradictory transcript authority leaves the prior projection and completion
state intact. Current Rust-owner Busy frames are consumed by Kotlin and mutated
to confirm this refusal; the fixture does not prove device/TCP/LAN execution.

When changed, `subscription_view` contains `{listing,subtabs,model_status}` from
existing authorized workspace/subtab/model-status dispatch. Android also accepts
the legacy two-field view. model_status is the existing bounded first catalog
page and selected model/reasoning/revision; it is null while the owner is Busy.
Other model read refusals terminate the subscription. Model selection commits
wake the same session subscription without inventing transcript events or ACKs.
The requested project/session must still
exist, and required_owner/workspace_revision must agree across both reads.
Revision drift fails Busy; lost access or changed session refuses instead of
inventing a selection. Views reuse the existing redacted desktop model preview,
device-scoped prompt statuses/text, pending approvals, read-only PTY projection
and stable-ID subtab display. They do not introduce another remote execution
operation. TUI/owned-headless mailboxes add their existing display fields through
normal workspace_list dispatch; no provider state or key is newly exposed.

The host sends a changed view once and omits it in unchanged heartbeat batches.
All batch result bytes (including transient view), not merely event frames,
count toward the existing 196608-byte subscription cap. Frequent/large changes
can terminate with ResourceLimit; there is no unbounded backlog or journal
deletion. The host uses session change wakeups with a 250 ms fallback, and the
entire fixed socket deadline applies. This is not a continuous durable event bus.

Android strictly decodes the view with existing project, approval, text, terminal
and subtab codecs, checks project/session/revision and leaves prior state intact
on malformed batch. A false transcript flag cannot advance either durable
cursor or append turns. Foreground publishes a view-only change without writing
the encrypted cache again when its cache projection is identical. Changed
durable/display cache content is still persisted before publication. Transient
text, PTY, model status/catalog and approvals are excluded from cache; running
statuses restore as unknown_outcome. Busy initial resume may enter a foreground subscription only
with a same-session previous cursor and advertised capability, preserving the
prior transcript. New sessions without a snapshot do not use that fallback.
Replay and Busy fallback now also require canonical u64 event/journal counters
with event <= journal, at most 64 uniquely identified retained projects, and
exact selected-project membership/session agreement. Malformed retained cursors
cannot normalize into resume frames or convert Busy into display availability;
the original refusal is preserved. Zero and maximum-u64 valid counters remain
admitted; unreadable state may use a fresh snapshot but cannot reuse its cursor.

Model revisions cannot regress within a subscription, including across a Busy
null sample. The open Android model panel adopts newer selection revisions and
resets stale drafts without sending any mutation. A concurrent paged read cannot
replace a newer pushed selection. An unchanged catalog snapshot preserves the
current catalog page, while fresh status metadata replaces older effective
budgets/capabilities/reasoning choices even at equal selection revision. Page
retention is keyed to project/session/revision and the complete catalog page,
not equality of the whole model status. Retained backward navigation also
merges current metadata so a prior page cannot restore an obsolete budget.
A reloaded snapshot replaces the page with the pushed first page. Null
availability samples do not grant mutation authority or erase the panel's last
known display. No model/catalog state is restored from the encrypted cache.

Transient-only batches also carry `cursor_scope:"transcript_events_v1"`;
Android refuses a missing or journal-scoped cursor rather than relying solely
on a same-looking sequence number. `subscription::serve_authorized` owns the
production current-device check and framed output for both PairingHost and the
new encrypted-stream tests. The caller still configures socket deadlines.

`remote_subscription_tls` exercises real rustls TLS 1.3 with the host's actual
certificate, ALPN zenpi/1, signed device challenge, durable registry and actual
owned session adapter over bounded fragmented in-memory byte channels. It
proves encrypted turn delivery, Busy-to-replay recovery, wrong certificate/proof
refusal, encrypted-record corruption, peer disconnect and mid-subscription
revocation through that shared function. It does not bind TCP, run PairingHost's
full hello/pair handler, scan QR, run Android TLS/Keystore, or replace any LAN,
listener shutdown, absolute socket deadline or device acceptance gate.

### Session Change Wakeups

Subscriptions now wait on a session-scoped content-free notification counter
instead of an unconditional 250 ms sleep. Successful durable turn/event append,
live model text update/removal and Prompt progress update notify the session.
The subscription captures the counter before sampling, then waits only while
it is unchanged, so a concurrent change during owner reading is not lost.
Multiple changes coalesce; no raw text, keys, requests or event queue is stored
in the notification. Each wake still performs fresh authorized view/replay
reads; notification is neither access nor proof of a journal commit.

The registry retains at most 128 live watched sessions via weak references.
Dropped subscriptions release their signal; later watch prunes dead entries.
Capacity exhaustion falls back to bounded timed sampling. A 250 ms timeout
still covers unpumped PTY output and cross-process external writes.
Notifications themselves are process-local.
One subscription sends at most 32 batches, ending with subscription_done=true
at the cap, in addition to existing duration/frame/byte/deadline limits. This
coalescing wakeup is incremental event-driven delivery, not a complete durable
stream-delta event bus or an unbounded queue.

Approval coordinators installed by Agent now bind a shared notification scope
to that Agent's session. Pending registration, accepted answer, cancellation
and response retraction notify that scope. Invalid responses do not notify.
Successful session replacement rebinds the shared scope and wakes both old/new
watchers; failed replacement leaves it unchanged. The coordinator itself is
not replaced, so ProjectOwnerPool and host-held clones retain the existing
approval rendezvous and digest/one-shot/audit authority. Standalone unbound
coordinators remain valid and send no remote wakeup.

Desktop TUI notifies the scoped session after pump_shell parses real output
into its bounded screen, and when ensure_shell replaces its scoped shell.
Notifications carry no PTY bytes or command authority. Authorization/redaction
and exact project/session/root checks still happen on the subsequent view read.
Output not yet pumped remains covered by timed sampling; this does not
introduce remote PTY input or an Android shell.

Workspace publication notifies retained and removed session watchers after the
owner checkpoint CAS succeeds and in-memory ownership is published. New and
resumed session commits wake old/new scopes and other retained project sessions,
because their workspace listing revision changes too. Local subtab commits wake
only their project's session: they change subtab revision, not workspace
revision. Remote subtab commits include a durable request receipt and workspace
revision, so they wake all retained workspace sessions. Local no-op selection,
identical local subtab commits, rejected candidates, failed CAS and exact remote
receipt retries do not issue workspace commit notifications. A new remotely
requested logical no-op still commits its own receipt/revision and does notify.

The notification is a hint, not an event or ACK. Every later subscribed view
is authorized and read again; display-only commits do not advance the durable
transcript cursor. Session replacement or project removal can instead terminate
an old-scoped subscription with the existing owner/access refusal. Tests now
cover successful local/remote subtab display changes over the shared authorized
TLS subscription with contiguous unchanged transcript cursor and one changed
view per commit. Their transport remains a bounded encrypted in-memory stream,
not proof of TCP/LAN or an Android APK run.

### Android Framed Response Consumption

HostClient now uses a shared ResponseReader for ordinary control/replay and
bounded subscription exchanges. It reads actual length-prefixed frames from
the already authenticated pinned TLS socket input, rejects unsolicited event
frames on control requests and requires the terminating response ID to match.
HostClient's existing outcome and workspace receipt checks run before the
subscription consumer; no parsing helper grants execution or write authority.
The reader also rejects a non-failed response carrying a non-null error code
before outcome validation or page publication. A succeeded page with a
revoked_device/unauthorized/busy/session_changed code is contradictory, not a
successful sync. Explicit null code remains compatible. The lower-level frame
schema still recognizes these typed codes; this is response-consumption
semantics, not a Rust/Kotlin wire-schema change.
Complete pages then pass through SubscriptionReducer and the existing
persist-before-publication callback. Partial prefixes/bodies, EOF after an
event, timeout, refusal and callback failure stop consumption without publishing
that incomplete or failed page. Previously committed pages remain available.

The reader and reducer both require batch 32 to terminate; a nonterminal batch
at that bound is rejected before publication, matching the host's done-at-cap
behavior. The reducer also validates the refreshed listing's active project,
not only the requested session and subtab revision. Per-page frame/byte and
aggregate subscription budgets remain enforced. Normal resume still consumes
one correlated page; it never invokes a subscription callback or consumes the
following control response.

Android's reader now counts original frame-body lengths rather than normalized
JSON reserialization. Event/snapshot bytes in an ordinary replay page use the
received lengths; subscriptions additionally count all terminal control pages
across the whole request toward 196608 bytes. After a valid four-byte length
prefix, subscription overflow refuses before body allocation/read, outcome
validation or publication. Prefix bytes themselves are outside this JSON-body
budget and remain bounded by frame/page counts. Whitespace or escaped spelling
cannot evade the limit. Last completely persisted pages remain intact; the
partially consumed connection is discarded by its owner, never reused.

JVM tests exercise this exact production reader plus codec/reducer with
three-byte fragmented input, every truncation of a later page, timeout,
wrong response ID/outcome, foreign/gapped events, page/frame/byte limits,
publication IO/cancellation failure and display preservation across heartbeats.
These tests do not instantiate Android SSLSocket/Keystore or scan QR, and do
not replace a fresh APK or real LAN acceptance run.

### Cache Commit Freshness

A previously known transcript event cursor must remain present on any
same-project/session projection replacement. An omitted cursor is refused
before live publication or foreground/request-completion/background cache
commit, independently of optional journal metadata. This does not reject an
initial legacy snapshot or compare counters across a different project/session.
It prevents protocol downgrade or a malformed replacement from erasing an
already committed resume position. Current Rust snapshot mutation/JVM checks
cover the policy; connected encrypted-store and LAN execution remain required.

PairedHostStore applies a shared freshness policy under its existing storage
lock before projection save, display-bearing workspace completion and resume
commit. For the same selected project/session, neither journal nor transcript
cursor may regress. Refusal throws a typed stale-projection error before the
encrypted AtomicFile write, rather than silently returning as though saved.
Different selected project/session counters are not compared. Equal cursors
allow changed transient/metadata views, and a newer journal may replace display
content without advancing the transcript cursor. This is not a complete
workspace-revision ordering protocol across equal journal cursors.

Foreground handling reloads a newer durable projection after this refusal and
remains unavailable until another verified host read. The rejected result never
reaches publish/next-read ACK. A workspace completion rejected with its display
keeps the pending request and prior bytes. Background resume still requires the
exact encrypted-record digest; freshness validation supplements that CAS rather
than replacing it. The workspace-confirmation callback also rechecks generation
inside the storage lock before clearing a pending request.

### Pairing Client Freshness At Send

Android PairingClient delegates its post-pinned-TLS exchange to PairingHandshake.
The socket wrapper still owns TLS 1.3, pinned certificate, zenpi/1 ALPN, absolute
deadline and cancellation close. The exchange checks invite freshness and
coroutine activity before hello, after welcome, after the matching host
challenge and again after signing immediately before sending the token. A slow
welcome/challenge or signing operation cannot intentionally send a locally
expired invitation. Host expiry/one-use admission remains authoritative.
Each handshake identity check also tests coroutine ownership after the
potentially blocking Keystore read, not just before it. Cancellation during
admission, post-welcome/challenge, post-signing or post-receipt identity reads
preserves the original cancellation and refuses the next stage. A final
post-receipt cancellation cannot publish successful registration locally;
it neither resends the one-use invite nor claims remote registration rollback.

Android pairing and authenticated reads share HandshakePrelude for inbound
welcome/challenge validation. These connection-scoped messages must have
exactly schema_version/channel/payload in their envelope; even null request,
project/session/turn, owner/revision, sequence or outcome fields are refused.
The payload still requires supported version, well-formed nonce and the pinned
host fingerprint. No signing or pair/auth/revoke-status proof follows invalid
negotiation. HostClient also checks cancellation between the two reads and
before preparing proof. The current Rust listener emits these exact envelopes.

Successful pairing requires correlated succeeded response, exact expected
device identity and true boolean paired/device_registered. control_available
must be boolean and is true when pairing granted the mounted project. Current
host device_revision is accepted and checked as positive u64 when present;
legacy schema-1 receipts without it remain compatible. Unknown result fields
or contradictory registration flags refuse. An accepted correlated receipt
received after local expiry remains valid: the host may have committed before
expiry, and the client must not misclassify that solely by receipt time.

The exchange never resends a one-use token after partial receipt, cancellation
or outcome ambiguity. JVM tests use actual FrameCodec framed fragmented bytes,
the production exchange and clock/signing hooks to verify expiry boundaries,
fingerprint refusal, invalid receipt, every receipt truncation and cancellation.
They do not exercise Android Keystore, certificate pinning runtime, camera QR
or TCP/LAN and are not a replacement for device acceptance.

### Terminal Android Socket Ownership

PairingClient and HostClient now share a per-operation SocketLease instead of
replacing an AtomicReference after raw TCP-to-TLS wrapping. Cancellation and
the absolute deadline mark the lease terminal before closing its currently
owned socket. A late raw installation or TLS-wrapper transfer is refused and
the new socket is closed, so an already fired close cannot be bypassed by a
subsequent reference replacement. Valid transfer retains raw TCP until its
owning TLS wrapper closes; it does not prematurely close the underlying stream.

Closing and transfer are serialized for ownership, but socket.close runs
outside the ownership lock. Close failure does not reopen the lease. Repeated
close does not retain or re-close the removed owned reference. The existing
finally block still closes raw TCP as a fallback even if owned close throws.
This does not change TLS pinning, request retries, timeout duration, Android
lifecycle generation fencing or commit/ACK policy. JVM tests exercise both
orderings, a 100-iteration transfer/close race, failure and coroutine cancellation
with tracked wrappers plus actual unconnected java.net.Socket close. They do
not prove a real connected Android TLS operation interrupts within a measured
deadline; that remains a device/network acceptance gate.

### Android Certificate Pin Verification

PinnedTrustManager validates the configured fingerprint as exactly 64 lowercase
hex characters at construction and decodes it once. Server trust requires one
certificate, valid at the current device time, and constant-time comparison of
SHA-256 over its DER bytes with the QR/saved pin. It does not substitute a
public-key-only digest or system CA trust. Client certificate trust is not
provided by this manager. PairingClient and HostClient keep using its normal
constructor with current time; the internal clock hook is for deterministic
validity tests only.

The manager's server pin, chain shape, validity and DER encoding refusals use
a typed HostIdentityException. Android follows exception causes by identity
with cycle protection because TLS providers may wrap the trust callback.
Only that local typed signal, not a generic SSLException/CertificateException
or matching diagnostic string, selects host-identity failure. Existing
authority-loss handling removes stale control/transient projections and stops
foreground automatic reads. ResumeWorker does not retry this scheduled
opportunity. Stored host identity, durable text/cursors, unknown writes and
pending revocation are retained; no automatic re-pair, pin replacement, token
retry or deletion is authorized. Explicit later reads still verify the pin.

JVM tests parse and verify a real public self-signed certificate generated by
Rust HostIdentity, check its zenpi.local SAN, accept the exact DER pin, reject
each one-character pin mutation and a key-only pin, reject missing/extra chains,
and enforce both validity boundaries. No private key is embedded. This tests
the production trust callback on JDK X.509, not Android's TLS provider or camera
QR; fresh Android socket/Keystore/pairing acceptance remains required.

### Android Negotiation Refusal

PairingClient and HostClient share ALPN admission requiring exactly zenpi/1.
ALPN mismatch, invalid complete welcome/challenge frames, unsupported version,
routed negotiation or challenge fingerprint mismatch produce a typed
HostNegotiationException before device proof or subsequent requests are sent.
The classifier follows exception causes with identity-cycle protection and
does not interpret diagnostic text. This is distinct from certificate trust
failure; neither is repaired by silently replacing the stored pin or selecting
a fallback protocol.

On reauthentication, the existing authority-loss handling removes old live
controls/transient surfaces and stops automatic foreground reads. Background
ResumeWorker considers the refusal permanent for that scheduled opportunity.
Durable host metadata, transcript/cursor and unresolved exact request identities
remain. Explicit retry still negotiates and authenticates from scratch. EOF,
partial-frame EOF, socket error and timeout remain transient rather than being
misreported as confirmed protocol refusal. Ordinary post-negotiation WireException
handling is unchanged.

JVM tests exercise actual fragmented prelude parsing and ALPN admission, prove
refusal classification through wrappers and cause cycles, distinguish incomplete
traffic from complete invalid negotiation, and check old approval/cancel/steering
authority cannot survive the refusal. These are not Android TLS-provider, APK,
camera or physical LAN acceptance evidence.

### Android Unconfirmed Authentication

AuthenticationReceipt validates the succeeded authentication envelope and
exact device identity/read-only result before workspace follow-ups. Invalid
receipt shape/booleans, false authentication, foreign device ID, extra authority
fields or invalid device revision use HostAuthenticationException. Local
signing-device mismatch with the saved host is classified identically before
opening a socket. This typed refusal is cycle-safe through cause wrappers and
cannot be forged by matching exception text.

Foreground authority-loss handling strips old live controls/transient views
and terminates automatic reads; ResumeWorker fails this scheduled opportunity
without retry. It does not erase durable pairing/text/cursors or resolve an
unknown mutation/revoke. Explicit same-request reconciliation still requires
a fresh successful authentication. Valid current and legacy no-revision
read-only receipts remain compatible. Ordinary EOF/timeout/framing errors that
occur before a complete receipt is admitted retain their existing classification.

JVM regressions validate fragmented correlated receipts before any follow-up,
wrapped semantic refusals, cause cycles, spoofed diagnostic text and denial of
old prebuilt approval/cancel/steering decisions while preserving unresolved
identity. They do not prove production Android TLS/Keystore or physical LAN.

The authentication fence includes reader-level rejection before the semantic
validator: invalid decoded frames, unexpected event/message, response correlation
or forbidden envelope metadata and contradictory success/error codes become
typed unconfirmed-authentication failures for authenticate requests only.
Non-failed incomplete outcome states are checked by AuthenticationReceipt
before generic operation completion. Typed failed host codes preserve their
existing handling. EOF, partial-frame EOF and socket/timeout I/O are not converted
to semantic refusal; other request types keep their existing reader errors.
Tests exercise these framed adversarial paths and every truncated valid receipt
without follow-up reads or false authority. This is not connected device evidence.

### Device Proof Byte Compatibility

Android DeviceIdentity signs DeviceProofCodec's UTF-8 string-array messages for
pair/authenticate/revoke-status, using the existing domain tags and fields.
Gson's HTML escaping remains disabled, and its JavaScript line-separator
escapes are normalized to literal U+2028/U+2029 bytes to match serde_json.
The scanner consumes escaped backslashes as pairs, so a device name containing
the literal characters backslash-u2028 is not confused with the Unicode scalar.
No case folding, name normalization or signature-verification leniency is added.

Rust and JVM tests read the same device-proof-v1.json fixture and compare exact
bytes for ordinary names, HTML punctuation, quotes/backslashes, Unicode names,
line separators and literal escape text across the three proof domains. Both
sides also exercise actual P-256 signatures to demonstrate that equivalent
JSON values with different byte encodings are not interchangeable signatures.
DeviceIdentity still uses non-exportable Android Keystore for production signing;
JVM keys are test-only. These checks do not replace Android Keystore/device
or end-to-end QR/LAN acceptance.

### Terminal Logical-Line Redaction

The owned PTY screen now tracks its parser-generated soft wraps separately from
hard newlines. Desktop local rendering still uses the same physical-row view.
Remote TUI projection uses remote_content to join retained soft-wrapped rows
before terminal line redaction. If the 64-row display window starts inside a
soft wrap, it includes the retained logical prefix for redaction. If that prefix
was evicted from the bounded 4000-row scrollback, its orphaned continuation is
withheld until the next hard newline instead of exposing unknown fragments.
Wrap metadata is maintained through erase/truncate/cursor operations.

Terminal sensitive-line markers also include private_key, secret_key and private
key labels. A visible PEM private-key begin marker withholds its following lines
through the matching end marker. The existing 16KiB wire cap, exact project/
session/subtab/root authorization, read-only scope and cache exclusion remain.
This closes parser soft-wrap fragmentation, not arbitrary program-inserted
hard-newline obfuscation or a PEM block whose begin marker is outside the sampled
window. Unlabelled unknown secrets remain a residual redaction risk. Real PTY
and parser tests verify wrapped-marker refusal without changing local content.

### Busy ACK Deferral

Android sends only a previously saved display cursor as ACK, but the host may
return Busy while its Agent execution lock is held. That confirmed read refusal
now defers the ACK instead of terminating before subtab/view subscription. No
receipt is invented, no cursor is advanced and no ACK is retried in this exchange.
A later authenticated read can acknowledge its then-saved cursor normally.

HostClient emits a typed HostRefusedException only after ResponseReader verifies
the correlated terminal response. EventAckCodec defers only that typed busy
code. Unauthorized, OwnerMismatch, malformed receipt, timeout, disconnected IO,
generic similarly worded exceptions and cancellation remain terminal to this
read. Host outcome/write refusal and snapshot/replay checks are unchanged. The
same stream is aligned after consuming a complete Busy ACK response, so the
following control read remains available. Rust tests verify Busy does not alter
checkpoint ACK records while fresh authorized workspace/subtab view remains
available with the same owner lock held. This is not a claim of new APK/LAN
foreground running-provider streaming acceptance.

## Android Subtab Freshness

For the same selected project/session, an existing subtab projection cannot
be replaced by a lower subtab revision or a missing subtab view, even when
journal/event cursors are equal or newer. These counters have separate owner
domains: a local subtab rename/select/order commit need not append a session
turn. The cached display already preserves subtab revision without command
authority, so ordinary save, resume CAS and confirmed-write display commits
can apply this check without restoring owner permissions. Project/session
changes do not compare unrelated subtab counters.

SubscriptionReducer applies the shared freshness policy before publishing
its next projection or accounting an accepted page. A refused stale page
leaves prior display/cursor/completion intact. Existing typed stale-projection
handling reloads current cache and retries bounded reads; it never ACKs a
discarded cursor or retries mutations automatically. This does not order all
workspace listings across process owner epochs or repair host checkpoint
rollback; same-revision projection inconsistencies remain a separate issue.

## Private PEM In Text Projections

The shared text redactor now withholds recognized PEM private-key block lines
and their bodies before conventional secret-value/assignment/header redaction.
Matching case-insensitive BEGIN/END labels include PRIVATE KEY, RSA, EC,
OPENSSH and ENCRYPTED PRIVATE KEY. Public key/certificate blocks are not
private-key material. An absent or mismatched END does not release the body;
nested recognized blocks remain withheld until their matching ends. Ordinary
LF/CRLF and surrounding nonprivate lines are preserved; an inline private
block withholds its whole containing line.

Remote prompt/desktop cumulative previews, projected transcript turns and
their snapshot/replay paths already call this shared redactor. A stream chunk
boundary does not discard the retained BEGIN prefix; every publishable
complete-line prefix and TextDone replacement is redacted again. The host's
local durable journal remains unchanged. This protects recognized PEM
material, not arbitrary unknown/obfuscated secrets or a projection whose
private BEGIN was lost before the supplied text window. Existing preview byte
overflow/NUL withholding and complete-line publication remain in force.

Regression evidence includes every byte split of one ASCII private-key
preview, unfinished desktop preview, nested/mismatched blocks, durable journal
reopen/replay and authenticated rustls transcript delivery over bounded memory
streams. These checks do not prove fresh Android APK, actual TCP/LAN or camera
scanning. See `progress-20261002-private-pem.md` under the Stage 2 quality root.

## Paged Diff Read

Read-only diff retains its legacy three arguments. A paged request adds offset
and digest together: initially 0/null, then prior next_offset/full-document
digest. Responses add page {digest, offset, next_offset, total_bytes, has_more}.
Patch pages contain at most 8192 UTF-8 bytes. Digest binds the full sanitized
scope/path/patch/status/flags result. Each continuation recomputes the bounded
document under current ACL and scope; changes return revision_conflict. No host
snapshot is retained, and reads do not advance journal/event/ACK cursors.

Controlled Git keeps private-path exclusions, no external diff/textconv/fsmonitor,
literal scope, deadline and child cleanup. Paged patch cap is 1 MiB; legacy/local
cap remains 64 KiB. Status stays at 32 KiB and directory untracked inspection at
32 files; reaching either bound marks the paged document truncated. Redaction
precedes hashing/slicing; post-redaction oversize is refused. This is bounded
review, not unlimited repository history, apply/revert or mutation.

Android diff admission shares DiffReadPolicy between UI and ViewModel. Browsing
requires a connected unique listed project/session advertising diff, a unique
selected subtab and no scan/reset/loading/pending write/revoke. Read-only and
running-task clients remain eligible without prompt authority. Navigation also
requires successful current scoped metadata; failed reads can refresh. The path
field and IME action are disabled while fenced. Request/path validation occurs
before a dialog state change or device-key access, and duplicate selected-tab
or mixed-capability duplicate project identities cannot authorize a request.
JVM policy tests execute these boundaries; Compose coverage is compiled source
evidence until connected tests run on an attributed current APK.

The AndroidRustDiffTest explicit integration lane generates current paged diff
frames from decoded actual workspace/inspect/subtab responses, dispatches them
through ProjectRuntimeHost against a real local Git fixture, and decodes the
response with fragmented JVM reads. A Read-only software P256 fixture device
reads a complete multi-page UTF-8 patch. Changed content invalidates the digest;
foreign session and revoked device are refused. The actual owner journal stays
unchanged and absolute host roots/provider credentials are absent from frames.
This is framing/codec/desktop owner evidence, not Android Keystore, listener
TLS, physical QR, current APK or direct-LAN device acceptance.

### Android Invitation Deadline

No QR wire fields change. Android additionally binds each parsed invite to an
elapsedRealtimeNanos deadline derived from its initially remaining wall-clock
window (at most 300 seconds). Both clocks must permit admission; observed expiry
or clock anomaly retires that in-memory object permanently. Device sleep counts
toward elapsed realtime. UI, credential admission and pre-token pairing checks
share the fence. A fresh parse cannot bypass host expiry, monotonic issuer
deadline or one-use token admission. After token submission a valid confirmed
receipt remains valid even if the local deadline has since passed.

### Current Conversation History

History turn identities are opaque host identifiers, not filenames or ASCII
slugs: nonblank, at most 128 UTF-8 bytes, no controls. Android validates that
same contract on decoded and retained pages without trimming or normalization.
Current owner/Kotlin interoperability includes a Unicode/punctuation historical
ID and exact full-message request/response correlation. Over-byte-limit,
blank/control-bearing IDs remain refused before navigation or detail requests.

Android history initial-read admission now shares SessionHistoryReadPolicy.browse
across entry/refresh and ViewModel. It requires connected unique current listed
project/session and a canonical u64 journal revision, with no scan/reset/loading,
pending write/revoke or open transcript-message read. Read-only and running-task
clients remain eligible. Forward reads additionally use the same open/error-free
scope/history-chain navigation gate as backward/message reads. Invalid scope or
journal bounds fail before another dialog closes or device keys are accessed.
Stale/broken/error pages can refresh only against valid current authority. JVM
tests execute these checks; extended Compose coverage is compiled, not device-run.

The explicit AndroidRustHistoryTest lane now obtains actual desktop workspace/
inspect scope, encodes current framed history requests, and decodes fragmented
ProjectRuntimeHost responses under an authenticated Read-only software P256
fixture device. Seventy durable turns extend beyond the 64-turn live snapshot;
five pages cover each ID in order. Kotlin checks transient historical-message
selection without modifying the live projection, page chains, redaction and
scope/digest rejection. Appending a host turn invalidates the old journal
request; device revocation denies it. Reads do not append journal data or
advance workspace revision. This is owner/JVM framing evidence, not listener
TLS, Android Keystore, current APK or physical QR/direct-LAN acceptance.

The same history integration lane additionally opens an old truncated-preview
message through SessionHistoryReadPolicy's transient projection and current
TranscriptPageCodec requests bound to the captured journal revision. Actual
desktop owner responses supply the complete multi-page UTF-8/redacted body.
Kotlin validates fragmented frame reads, retained previous-message pages,
complete byte length/content digest and unchanged live snapshot. Host journal
append invalidates both the list and detail requests; revocation denies both.
This remains deterministic owner/JVM evidence, not connected Android UI.

Android full-message/tool-argument reads use
the host's opaque turn-identity bound on selected turn request construction and
worker response decoding: nonblank, at most 128 UTF-8 bytes, no controls.
Unicode/punctuation IDs are preserved exactly and do not relax authenticated
task, project/session, subtab, role, digest or continuation matching. The current
worker/JVM lane mutates genuine host responses to exercise these ID boundaries;
it does not claim the host dispatched those altered identities.

Android full-message/tool-argument initial reads now share
TranscriptReadPolicy.browse between visible live-message entries and ViewModel.
It requires connected unique listed project/session and no scanner/reset/
loading/pending write/revoke. The same gate also fences backward/forward
navigation across live, historical and worker message pages. The codec checks
unique listed scope before initial requests; existing exact turn/tool/task and
historical journal checks remain separate. Failed pages can reopen via a fresh
eligible read, never continue the failed page chain. Read-only/running-task
clients remain eligible without prompt authority. JVM tests and actual old-body
interop assertions execute the scanner policy; Compose cases compile only.

transcript_read also accepts exactly {view: "session_history", offset, digest,
journal_revision}. Initial offset zero permits null digest; continuation requires
a lowercase SHA-256 digest. The Read-authorized host checks current project,
session and exact u64 journal revision under the owner lock. Only selected branch
turns are returned, in chronological pages of at most sixteen redacted 2048-byte
previews plus bounded tool summaries. Responses carry project_id, session_id,
journal_revision, digest, offset, next_offset, total_turns, has_more and turns.
Empty sessions produce a zero-turn terminal page. History uses the existing
control-frame budget and refuses more than 16384 durable turns, 64 MiB serialized
selected history or an exhausted one-second opportunity. JSON is streamed into
the bounded SHA-256 writer, not buffered/cloned as a complete conversation;
escaping and metadata count toward this budget. The digest is compatible with
SHA-256 of the full JSON array of selected durable turns. Noninitial empty
terminal pages are refused. The digest covers the
selected turns; revision/digest conflicts refuse continuation without mutation.
Android history is transient and independent of its latest-64-turn cache. It
retains seven previous pages and uses an exact scoped history entry to request
the existing turn/tool-argument read, never to authorize a write. Any journal,
branch, session or authority change invalidates that read scope. Existing worker
history and ordinary transcript arguments are unchanged.

Historical message/tool-argument reads also include optional journal_revision
alongside turn_id, offset, digest and optional tool_call_id. It is an exact u64
checked under the owner lock before branch/message lookup; a mismatch returns
revision_conflict even if the same immutable turn still exists. Successful
responses echo the numeric revision. Android sends it on every historical
detail page and requires exact echo and retained-page continuity, refusing
missing/string/changed revision receipts. Worker reads reject this field;
ordinary live transcript requests without it retain their previous semantics.

### Session Catalog Search

Android SessionCatalogReadPolicy is shared by catalog entry/search/refresh/page
controls and ViewModel admission. Reads require connected unique current listed
project/session and no scan/reset/loading/pending write/revoke. Resume authority
and an idle task are not prerequisites for browsing. Page navigation additionally
requires an open error-free current catalog with the exact retained query.
Foreign/closed/query-mismatched pages cannot navigate; failures can start a fresh
read. The codec also checks unique current project/session before any initial
request. Invalid read admission does not close another dialog or access device
keys. Resume keeps its separate current catalog/CAS/write checks. JVM tests
execute these boundaries; extended Compose cases are compiled, not device-run.

session_list accepts optional query alongside its existing offset/snapshot_id
pair. Query is a trimmed, control-free string of at most 128 UTF-8 bytes;
redaction-sensitive values are rejected. The Read-authorized host computes its
existing bounded same-project catalog, then case-insensitively filters public
session IDs, projected names and redacted user/assistant turn content before
slicing sixteen-entry pages. Tool/system turns, tool arguments and arbitrary
event payloads are not searched. Redaction happens before matching to prevent
hidden values from acting as an existence oracle. Responses echo query only
when supplied; each filtered entry adds matches (matching conversation turn
count) and snippets (the first 240 Unicode code points of up to four matching
redacted turns, stripping controls other than newline/tab). Metadata-only matches
have zero matches and no snippets. Empty query matches metadata only.
Unfiltered entries remain unchanged. Search remains subject to the one-second,
512-journal, 8-MiB/journal and 64-MiB aggregate catalog limits; an incomplete
scan fails resource_limit rather than claiming exhaustive results.
The full catalog digest remains the snapshot/resume token so
filtered results use the existing session_resume transaction unchanged.

Android binds request, response and retained page history to the exact query.
Changing it restarts from offset zero, clears old pages/confirmation, and never
mixes with an unfiltered page. Search does not change session/journal/event/ACK
state. Query stays transient, survives a read error for explicit refresh and is
cleared on scope/authority loss. Legacy unfiltered requests/responses are unchanged;
older hosts may refuse query rather than silently apply a different filter.
Android accepts legacy query responses without search fields; when either field
is present both are required, matches cannot exceed turn_count and exactly
min(matches, 4) snippets are required. Each snippet is bounded to 240 code points
and 960 UTF-8 bytes. Connected desktop/Android search acceptance remains open.
The current session interoperability lane requests query `desktop` from the
real Rust catalog in each new/resume/restart phase. Kotlin validates both
historical conversation matches, exact snippets and the unchanged unfiltered
catalog digest, rejects either partially missing search field, and preserves
the documented legacy response behavior. No search-side journal mutation or
physical network acceptance is implied by this offline lane.

package zenpi.lifecycle

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.drawable.Icon
import android.os.Build
import androidx.work.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.coroutines.ensureActive
import kotlin.coroutines.coroutineContext
import zenpi.security.DeviceIdentity
import zenpi.security.PairedHost
import zenpi.security.PairedHostStore
import zenpi.transport.ApprovalProjectionCodec
import zenpi.transport.HostClient
import zenpi.transport.HostProjection
import zenpi.transport.HostRefusedException
import zenpi.transport.RemoteApproval
import zenpi.transport.SelectedProjectUnavailableException
import zenpi.transport.OperationDeadline
import java.util.concurrent.TimeUnit

/**
 * A bounded read opportunity, never an unresolved-command sender. ZE-544: when the read finds a
 * pending approval it posts [ApprovalNotifier]'s notification, then follows the host (wakeup-driven
 * subscription pages, ZE-542) for at most [WATCH_MS] so the notification is withdrawn as soon as the
 * approval is handled elsewhere. No foreground service, no push.
 */
class ResumeWorker(context: Context, parameters: WorkerParameters) : CoroutineWorker(context, parameters) {
    companion object {
        /** Stays inside WorkManager's 10 minute execution limit. */
        const val WATCH_MS = 9 * 60_000L
    }

    private class Read(val client: HostClient, val host: PairedHost, val identity: DeviceIdentity,
        val projection: HostProjection, val committed: Boolean, val notified: Boolean)

    override suspend fun doWork(): Result = withContext(Dispatchers.IO) {
        val namespace = inputData.getString("namespace") ?: "v1"
        try {
            val read = OperationDeadline.run(20_000) {
                val store = PairedHostStore(applicationContext, namespace)
                val snapshot = store.loadResumeSnapshot() ?: return@run null
                val identity = zenpi.transport.AuthenticationProof.existingIdentity(snapshot.state.host.deviceId, {
                    DeviceIdentity.loadExisting(inputData.getString("identity_alias") ?: "zenpi.device.v1")
                }, { it.deviceId })
                val client = HostClient(LanSocketRouting(applicationContext)::bind)
                val next = ResumeReadPolicy.read(snapshot.state.projection) { previous ->
                    client.inspect(snapshot.state.host, snapshot.state.projection?.selectedId,
                        identity, previous = previous, acknowledgePrevious = previous != null, recoverSessionChanges = true,
                        // A turn waiting on an approval holds the transcript; the listing still names it.
                        busyListing = true)
                }
                val commitContext = coroutineContext
                fun current() = zenpi.transport.AuthenticationProof.current(snapshot.state.host.deviceId,
                    { identity.deviceId }, { commitContext.ensureActive() })
                current()
                val committed = store.commitResume(snapshot, next) {
                    current()
                }
                Read(client, snapshot.state.host, identity, next, committed, ApprovalNotifier.sync(applicationContext, namespace, next))
            } ?: return@withContext Result.success(workDataOf("resumed" to false))
            if (read.notified) watch(read, namespace)
            Result.success(workDataOf("resumed" to true, "committed" to read.committed, "notified" to read.notified))
        } catch (error: CancellationException) { throw error }
        catch (error: Exception) {
            if (ResumeRetryPolicy.retry(error, runAttemptCount)) Result.retry() else Result.failure()
        }
    }

    /**
     * Follows the saved project until its approval queue drains (withdrawing the notification on the
     * page batch that shows it) or [WATCH_MS] passes. Read-only: nothing is acknowledged or committed,
     * and any failure just ends the watch, leaving the notification for the app to settle.
     */
    private suspend fun watch(read: Read, namespace: String) = coroutineScope {
        val drained = CompletableDeferred<Unit>()
        val reader = launch {
            var from = read.projection
            while (isActive && !drained.isCompleted) {
                from = try {
                    read.client.inspect(read.host, from.selectedId, read.identity, previous = from, subscribe = true,
                        onProjection = { next ->
                            // The app's onStart cancels this work and withdraws the notification; never re-post after that.
                            coroutineContext.ensureActive()
                            if (!ApprovalNotifier.sync(applicationContext, namespace, next)) drained.complete(Unit)
                        })
                } catch (error: CancellationException) { throw error }
                catch (_: Exception) { break }
                coroutineContext.ensureActive()
                if (!ApprovalNotifier.sync(applicationContext, namespace, from)) drained.complete(Unit)
            }
        }
        withTimeoutOrNull(WATCH_MS) { drained.await() }
        reader.cancel()
    }
}

/**
 * ZE-544: the single high-priority approval notification for the head of the saved project's queue.
 * Its body opens that project's session; "Deny" / "Allow once" require an unlocked device and open
 * the app, which decides only if the same approval (request, worker, digest) is still the live head.
 */
object ApprovalNotifier {
    const val CHANNEL = "zenpi.approvals"
    const val ACTION = "zenpi.action.APPROVAL"
    const val ID = 544
    private const val PROJECT = "zenpi.approval.project_id"
    private const val REQUEST = "zenpi.approval.request_id"
    private const val WORKER = "zenpi.approval.worker_session_id"
    private const val DIGEST = "zenpi.approval.digest"
    private const val DECISION = "zenpi.approval.decision"

    data class Target(val projectId: String, val requestId: String, val workerSessionId: String?, val digest: String, val decision: String?) {
        fun matches(approval: RemoteApproval): Boolean = approval.projectId == projectId && approval.requestId == requestId
            && approval.workerSessionId == workerSessionId && approval.digest == digest
    }

    /** The approval this device may decide next in the saved project, if any. */
    fun head(projection: HostProjection?): RemoteApproval? {
        val project = projection?.projects?.singleOrNull { it.id == projection.selectedId } ?: return null
        return project.pendingApprovals.firstOrNull()?.takeIf { project.approvalControl && ApprovalProjectionCodec.current(projection, it) }
    }

    /** Shows the queue head of [projection] or withdraws the notification; true while one is pending. */
    fun sync(context: Context, namespace: String, projection: HostProjection?): Boolean {
        val head = head(projection)
        if (head == null) cancel(context, namespace)
        else post(context, namespace, head, projection?.projects?.single { it.id == head.projectId }?.title.orEmpty())
        return head != null
    }

    fun cancel(context: Context, namespace: String = "v1") {
        context.getSystemService(NotificationManager::class.java).cancel(namespace, ID)
    }

    fun target(intent: Intent?): Target? {
        if (intent?.action != ACTION) return null
        val decision = intent.getStringExtra(DECISION)
        val target = Target(intent.getStringExtra(PROJECT) ?: return null, intent.getStringExtra(REQUEST) ?: return null,
            intent.getStringExtra(WORKER), intent.getStringExtra(DIGEST) ?: return null, decision)
        return target.takeIf { decision == null || decision == "allow" || decision == "deny" }
    }

    private fun post(context: Context, namespace: String, head: RemoteApproval, project: String) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU
            && context.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED) return
        val manager = context.getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(NotificationChannel(CHANNEL, "Approvals", NotificationManager.IMPORTANCE_HIGH).apply {
            description = "Tool approvals waiting while Zenpi is in the background"
            lockscreenVisibility = Notification.VISIBILITY_PRIVATE
        })
        val icon = android.R.drawable.ic_dialog_alert
        fun open(decision: String?, code: Int): PendingIntent = PendingIntent.getActivity(context, code,
            Intent(context, zenpi.MainActivity::class.java).setAction(ACTION).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                .putExtra(PROJECT, head.projectId).putExtra(REQUEST, head.requestId).putExtra(WORKER, head.workerSessionId)
                .putExtra(DIGEST, head.digest).putExtra(DECISION, decision),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT)
        fun action(label: String, decision: String, code: Int) = Notification.Action.Builder(
            Icon.createWithResource(context, icon), label, open(decision, code)).setAuthenticationRequired(true).build()
        val effect = when (head.sideEffect) { "workspace_write" -> "Workspace write"; "command_execution" -> "Command execution"; else -> "Read only" }
        // The lock screen shows only that something waits; tool, effect and project need an unlock.
        val public = Notification.Builder(context, CHANNEL).setSmallIcon(icon).setContentTitle("Zenpi approval pending").build()
        manager.notify(namespace, ID, Notification.Builder(context, CHANNEL).setSmallIcon(icon)
            .setContentTitle("Approve ${head.tool}?").setContentText("$effect · $project")
            .setCategory(Notification.CATEGORY_REMINDER)
            .setVisibility(Notification.VISIBILITY_PRIVATE).setPublicVersion(public)
            .setOnlyAlertOnce(true).setAutoCancel(true).setContentIntent(open(null, 0))
            .addAction(action("Deny", "deny", 1)).addAction(action("Allow once", "allow", 2)).build())
    }
}

/** Authority failures cannot be repaired by retrying the same saved device grant. */
internal object ResumeRetryPolicy {
    private val permanentCodes = setOf("unauthorized", "revoked_device", "owner_mismatch")

    fun permanent(error: Exception): Boolean = error is SelectedProjectUnavailableException
        || zenpi.transport.HostIdentityFailure.refused(error)
        || zenpi.transport.HostNegotiationException.refused(error)
        || zenpi.transport.HostAuthenticationException.refused(error)
        || error is HostRefusedException && error.code in permanentCodes

    fun retry(error: Exception, attempt: Int): Boolean {
        if (error is CancellationException) throw error
        return attempt < 2 && !permanent(error)
    }
}

object ResumeScheduler {
    private fun name(namespace: String) = "zenpi-read-resume-$namespace"
    fun cancel(context: Context, namespace: String = "v1") {
        require(namespace.matches(Regex("[A-Za-z0-9._-]{1,80}")))
        WorkManager.getInstance(context).cancelUniqueWork(name(namespace))
    }
    fun enqueue(context: Context, namespace: String = "v1", identityAlias: String = "zenpi.device.v1", delaySeconds: Long = 30): java.util.UUID {
        val request = OneTimeWorkRequestBuilder<ResumeWorker>()
            .setInputData(workDataOf("namespace" to namespace, "identity_alias" to identityAlias))
            // CONNECTED can require Internet validation; a reachable offline LAN is valid.
            .setInitialDelay(delaySeconds, TimeUnit.SECONDS)
            .setBackoffCriteria(BackoffPolicy.EXPONENTIAL, 30, TimeUnit.SECONDS)
            .build()
        WorkManager.getInstance(context).enqueueUniqueWork(name(namespace), ExistingWorkPolicy.KEEP, request)
        return request.id
    }
}

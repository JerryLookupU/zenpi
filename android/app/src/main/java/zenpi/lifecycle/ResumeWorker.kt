package zenpi.lifecycle

import android.content.Context
import androidx.work.*
import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.ensureActive
import kotlin.coroutines.coroutineContext
import zenpi.security.DeviceIdentity
import zenpi.security.PairedHostStore
import zenpi.transport.HostClient
import zenpi.transport.HostRefusedException
import zenpi.transport.SelectedProjectUnavailableException
import zenpi.transport.OperationDeadline
import java.util.concurrent.TimeUnit

/** A bounded read opportunity, never a socket keeper or unresolved-command sender. */
class ResumeWorker(context: Context, parameters: WorkerParameters) : CoroutineWorker(context, parameters) {
    override suspend fun doWork(): Result = withContext(Dispatchers.IO) {
        try {
            OperationDeadline.run(20_000) {
                val store = PairedHostStore(applicationContext, inputData.getString("namespace") ?: "v1")
                val snapshot = store.loadResumeSnapshot() ?: return@run Result.success(workDataOf("resumed" to false))
                val identity = zenpi.transport.AuthenticationProof.existingIdentity(snapshot.state.host.deviceId, {
                    DeviceIdentity.loadExisting(inputData.getString("identity_alias") ?: "zenpi.device.v1")
                }, { it.deviceId })
                val client = HostClient(LanSocketRouting(applicationContext)::bind)
                val next = ResumeReadPolicy.read(snapshot.state.projection) { previous ->
                    client.inspect(snapshot.state.host, snapshot.state.projection?.selectedId,
                        identity, previous = previous, acknowledgePrevious = previous != null, recoverSessionChanges = true)
                }
                val commitContext = coroutineContext
                fun current() = zenpi.transport.AuthenticationProof.current(snapshot.state.host.deviceId,
                    { identity.deviceId }, { commitContext.ensureActive() })
                current()
                val committed = store.commitResume(snapshot, next) {
                    current()
                }
                Result.success(workDataOf("resumed" to true, "committed" to committed))
            }
        } catch (error: CancellationException) { throw error }
        catch (error: Exception) {
            if (ResumeRetryPolicy.retry(error, runAttemptCount)) Result.retry() else Result.failure()
        }
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

package zenpi

import androidx.test.platform.app.InstrumentationRegistry
import androidx.work.WorkInfo
import androidx.work.WorkManager
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import zenpi.lifecycle.ApprovalNotifier
import zenpi.lifecycle.ResumeScheduler
import zenpi.security.*
import zenpi.transport.*
import zenpi.lifecycle.LanSocketRouting
import java.util.concurrent.TimeUnit

class ResumeWorkerTest {
    private fun hostClient() = HostClient(LanSocketRouting(InstrumentationRegistry.getInstrumentation().targetContext)::bind)

    /** ZE-544: what ResumeWorker posts for a pending approval, and its withdrawal once the queue drains. */
    @Test fun pendingApprovalNotificationIsWithdrawnWithinOneSecondWithoutForegroundService() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val context = instrumentation.targetContext
        if (android.os.Build.VERSION.SDK_INT >= 33)
            instrumentation.uiAutomation.grantRuntimePermission(context.packageName, android.Manifest.permission.POST_NOTIFICATIONS)
        val namespace = "ze544.${System.nanoTime()}"
        val approval = RemoteApproval("r-544", "p", "s", "t", "c", "bash", "command_execution", "agent_tool", "a".repeat(64))
        val project = RemoteProject("p", "zenpi", "s", approvalControl = true, pendingApprovals = listOf(approval))
        val pending = HostProjection(listOf(project), "p", "s", null, emptyList(), null, false)
        val manager = context.getSystemService(android.app.NotificationManager::class.java)
        fun posted() = manager.activeNotifications.filter { it.tag == namespace && it.id == ApprovalNotifier.ID }
        try {
            assertTrue(ApprovalNotifier.sync(context, namespace, pending))
            val shown = posted().single().notification
            assertEquals(android.app.NotificationManager.IMPORTANCE_HIGH, manager.getNotificationChannel(shown.channelId).importance)
            assertEquals(listOf("Deny", "Allow once"), shown.actions.map { it.title.toString() })
            assertTrue(shown.actions.all { it.isAuthenticationRequired })
            assertNotNull(shown.contentIntent)
            val handled = android.os.SystemClock.elapsedRealtime()
            assertFalse(ApprovalNotifier.sync(context, namespace, pending.copy(projects = listOf(project.copy(pendingApprovals = emptyList())))))
            while (posted().isNotEmpty()) { assertTrue(android.os.SystemClock.elapsedRealtime() - handled < 1_000); Thread.sleep(10) }
        } finally { ApprovalNotifier.cancel(context, namespace) }
        val info = context.packageManager.getPackageInfo(context.packageName,
            android.content.pm.PackageManager.PackageInfoFlags.of((android.content.pm.PackageManager.GET_SERVICES or android.content.pm.PackageManager.GET_PERMISSIONS).toLong()))
        assertTrue(info.services.orEmpty().none { it.foregroundServiceType != 0 || it.name.endsWith("SystemForegroundService") })
        assertFalse(info.requestedPermissions.orEmpty().any { it.startsWith("android.permission.FOREGROUND_SERVICE") })
        assertTrue(android.Manifest.permission.POST_NOTIFICATIONS in info.requestedPermissions.orEmpty())
    }
    @Test fun realWorkManagerReadsSharedHostAndNeverFlushesUnknownWrites() = runBlocking {
        val namespace = requireNotNull(InstrumentationRegistry.getArguments().getString("process_namespace"))
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val store = PairedHostStore(context, namespace)
        val alias = "zenpi.network-process.$namespace"
        val manager = WorkManager.getInstance(context)
        val before = requireNotNull(store.loadState())
        val identity = requireNotNull(DeviceIdentity.loadExisting(alias))
        val expectedText = requireNotNull(InstrumentationRegistry.getArguments().getString("background_text"))
        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
            putString("zenpi_phase", "background_update_requested")
        })
        withTimeout(10_000) {
            var observed = false
            while (!observed) {
                try { observed = hostClient().inspect(before.host, identity = identity).turns.any { it.content.contains(expectedText) } }
                catch (error: WireException) { if (error.message != "Host request refused: busy") throw error }
                if (!observed) delay(100)
            }
        }
        assertEquals(before.projection?.eventSequence, store.loadState()?.projection?.eventSequence)
        suspend fun runWorker(identityAlias: String = alias): WorkInfo {
            val id = ResumeScheduler.enqueue(context, namespace, identityAlias, 0)
            try {
                return withTimeout(30_000) {
                    var info: WorkInfo?
                    do {
                        delay(100)
                        info = withContext(Dispatchers.IO) { manager.getWorkInfoById(id).get(5, TimeUnit.SECONDS) }
                    } while (info?.state?.isFinished != true)
                    requireNotNull(info)
                }
            } finally { manager.cancelWorkById(id) }
        }
        val read = runWorker()
        assertEquals(WorkInfo.State.SUCCEEDED, read.state)
        assertTrue(read.outputData.getBoolean("resumed", false))
        assertTrue(read.outputData.getBoolean("committed", false))
        val after = requireNotNull(store.loadState())
        assertEquals(before.host, after.host)
        assertTrue(java.math.BigInteger(requireNotNull(after.projection?.eventSequence)) > java.math.BigInteger(requireNotNull(before.projection?.eventSequence)))
        assertTrue(after.projection?.turns?.any { it.content.contains(expectedText) } == true)
        val alternateAlias = "zenpi.resume-admission.${System.nanoTime()}"
        val file = java.io.File(context.noBackupFilesDir, "paired-host.$namespace")
        val savedBytes = file.readBytes()
        assertNull(DeviceIdentity.loadExisting(alternateAlias))
        val missing = runWorker(alternateAlias)
        assertEquals(WorkInfo.State.FAILED, missing.state)
        assertEquals(1, missing.runAttemptCount)
        assertFalse(missing.outputData.getBoolean("resumed", false))
        assertArrayEquals(savedBytes, file.readBytes())
        assertNull(DeviceIdentity.loadExisting(alternateAlias))
        val replacement = DeviceIdentity.loadOrCreate(alternateAlias)
        try {
            assertNotEquals(after.host.deviceId, replacement.deviceId)
            val mismatch = runWorker(alternateAlias)
            assertEquals(WorkInfo.State.FAILED, mismatch.state)
            assertEquals(1, mismatch.runAttemptCount)
            assertArrayEquals(savedBytes, file.readBytes())
            assertEquals(replacement.deviceId, DeviceIdentity.loadExisting(alternateAlias)?.deviceId)
        } finally { replacement.wipe() }
        val live = hostClient().inspect(after.host, identity = identity, previous = after.projection)
        val command = WorkspaceCommand.create(live, requireNotNull(live.selectedId), "workspace_select")
        store.beginWorkspace(after.host, command)
        try {
            val skip = runWorker()
            assertEquals(WorkInfo.State.SUCCEEDED, skip.state)
            assertFalse(skip.outputData.getBoolean("resumed", true))
            assertEquals(command.envelope, store.loadState()?.pendingWorkspace?.envelope)
        val unchanged = hostClient().inspect(after.host, identity = identity, previous = after.projection)
            assertEquals(live.workspaceRevision, unchanged.workspaceRevision)
        } finally { store.finishWorkspace(after.host, command) }
        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
            putString("zenpi_phase", "workmanager_read_resume_verified")
        })
    }
}

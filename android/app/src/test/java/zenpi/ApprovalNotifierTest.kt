package zenpi

import android.app.NotificationManager
import android.content.Intent
import org.junit.Assert.*
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.Shadows.shadowOf
import zenpi.lifecycle.ApprovalNotifier
import zenpi.screenshot.Fixtures

@RunWith(RobolectricTestRunner::class)
class ApprovalNotifierTest {
    private val context = org.robolectric.RuntimeEnvironment.getApplication()
    private val manager = context.getSystemService(NotificationManager::class.java)
    private val pending = Fixtures.queuedState.projection!!
    private fun drained() = pending.copy(projects = pending.projects.map { it.copy(pendingApprovals = emptyList()) })
    private fun posted() = manager.activeNotifications.filter { it.tag == "v1" && it.id == ApprovalNotifier.ID }

    @Test fun headOfTheQueuePostsOneHighPriorityNotificationWithUnlockedActions() {
        shadowOf(context).grantPermissions(android.Manifest.permission.POST_NOTIFICATIONS)
        assertTrue(ApprovalNotifier.sync(context, "v1", pending))
        assertTrue(ApprovalNotifier.sync(context, "v1", pending))
        val shown = posted().single().notification
        assertEquals(NotificationManager.IMPORTANCE_HIGH, manager.getNotificationChannel(ApprovalNotifier.CHANNEL).importance)
        assertEquals(ApprovalNotifier.CHANNEL, shown.channelId)
        assertEquals("Approve edit_file?", shown.extras.getString(android.app.Notification.EXTRA_TITLE))
        assertEquals(android.app.Notification.VISIBILITY_PRIVATE, shown.visibility)
        assertEquals(listOf("Deny", "Allow once"), shown.actions.map { it.title.toString() })
        assertTrue(shown.actions.all { it.isAuthenticationRequired })
        val head = Fixtures.queued.first()
        val open = ApprovalNotifier.target(shadowOf(shown.contentIntent).savedIntent)!!
        assertEquals(MainActivity::class.java.name, shadowOf(shown.contentIntent).savedIntent.component?.className)
        assertTrue(open.matches(head)); assertNull(open.decision)
        assertEquals(listOf("deny", "allow"), shown.actions.map { ApprovalNotifier.target(shadowOf(it.actionIntent).savedIntent)!!.decision })
        assertFalse(open.matches(Fixtures.queued.last()))
        assertFalse(open.matches(head.copy(digest = "f".repeat(64))))

        assertFalse(ApprovalNotifier.sync(context, "v1", drained()))
        assertTrue(posted().isEmpty())
    }

    @Test fun nothingIsPostedWithoutApprovalControlOrPermission() {
        shadowOf(context).grantPermissions(android.Manifest.permission.POST_NOTIFICATIONS)
        val readOnly = pending.copy(projects = pending.projects.map { it.copy(approvalControl = false) })
        assertFalse(ApprovalNotifier.sync(context, "v1", readOnly))
        assertTrue(posted().isEmpty())
        shadowOf(context).denyPermissions(android.Manifest.permission.POST_NOTIFICATIONS)
        assertTrue(ApprovalNotifier.sync(context, "v1", pending))
        assertTrue(posted().isEmpty())
    }

    @Test fun onlyApprovalIntentsWithAKnownDecisionAreAccepted() {
        assertNull(ApprovalNotifier.target(Intent(Intent.ACTION_VIEW)))
        val base = Intent(ApprovalNotifier.ACTION).putExtra("zenpi.approval.project_id", "zenpi")
            .putExtra("zenpi.approval.request_id", "r").putExtra("zenpi.approval.digest", "c".repeat(64))
        assertNull(ApprovalNotifier.target(base)?.decision)
        assertNull(ApprovalNotifier.target(Intent(base).putExtra("zenpi.approval.decision", "allow_always")))
        assertEquals("deny", ApprovalNotifier.target(Intent(base).putExtra("zenpi.approval.decision", "deny"))?.decision)
        assertNull(ApprovalNotifier.target(Intent(base).apply { removeExtra("zenpi.approval.digest") }))
    }
}

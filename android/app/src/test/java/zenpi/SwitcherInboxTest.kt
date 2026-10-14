package zenpi

import org.junit.Assert.assertEquals
import org.junit.Test
import zenpi.transport.HostProjection
import zenpi.transport.RemoteApproval
import zenpi.transport.RemoteProject
import zenpi.transport.RemotePromptStatus
import zenpi.transport.RemoteWorkerStatus
import zenpi.ui.InboxGroup
import zenpi.ui.SwitcherInbox

class SwitcherInboxTest {
    private fun approval(id: String, project: String) =
        RemoteApproval(id, project, "s-$project", "turn", "call", "bash", "command_execution", "agent_tool", "a".repeat(64))
    private fun prompt(project: String, state: String) = RemotePromptStatus("r-$project-$state", project, "s-$project", state)
    private fun projection(vararg projects: RemoteProject) =
        HostProjection(projects.toList(), projects.first().id, projects.first().sessionId, "idle", emptyList(), "1", false)

    @Test fun approvalsOutrankRunningWorkAndFinishedRunsAreIdle() {
        val blocked = RemoteProject("blocked", "Blocked", "s-blocked", pendingApprovals = listOf(approval("1", "blocked")),
            promptStatuses = listOf(prompt("blocked", "running")))
        val prompting = RemoteProject("prompting", "Prompting", "s-prompting", promptStatuses = listOf(prompt("prompting", "accepted")))
        val worker = RemoteProject("worker", "Worker", "s-worker",
            workerStatuses = listOf(RemoteWorkerStatus("w", "worker", "s-worker", "b".repeat(64), "running")))
        val desktop = RemoteProject("desktop", "Desktop", "s-desktop", desktopStreamText = "typing")
        val finished = RemoteProject("finished", "Finished", "s-finished",
            promptStatuses = listOf(prompt("finished", "succeeded"), prompt("finished", "failed")))
        val fresh = RemoteProject("fresh", "Fresh", "s-fresh")
        assertEquals(InboxGroup.NEEDS_YOU, SwitcherInbox.group(blocked))
        for (project in listOf(prompting, worker, desktop)) assertEquals(project.id, InboxGroup.RUNNING, SwitcherInbox.group(project))
        for (project in listOf(finished, fresh)) assertEquals(project.id, InboxGroup.IDLE, SwitcherInbox.group(project))
    }

    @Test fun groupsFollowInboxOrderKeepHostOrderAndOmitEmptyGroups() {
        val idleA = RemoteProject("a", "A", "s-a")
        val needsB = RemoteProject("b", "B", "s-b", pendingApprovals = listOf(approval("1", "b")))
        val idleC = RemoteProject("c", "C", "s-c")
        val needsD = RemoteProject("d", "D", "s-d", pendingApprovals = listOf(approval("2", "d")))
        val groups = SwitcherInbox.groups(projection(idleA, needsB, idleC, needsD))
        assertEquals(listOf(InboxGroup.NEEDS_YOU to listOf("b", "d"), InboxGroup.IDLE to listOf("a", "c")),
            groups.map { (group, projects) -> group to projects.map { it.id } })
        assertEquals(emptyList<Any>(), SwitcherInbox.groups(null))
    }

    @Test fun badgeCountsEveryPendingApprovalAcrossProjects() {
        val one = RemoteProject("one", "One", "s-one", pendingApprovals = listOf(approval("1", "one"), approval("2", "one")))
        val two = RemoteProject("two", "Two", "s-two", pendingApprovals = listOf(approval("3", "two")))
        assertEquals(3, SwitcherInbox.pendingApprovals(projection(one, two, RemoteProject("three", "Three", "s-three"))))
        assertEquals(0, SwitcherInbox.pendingApprovals(projection(RemoteProject("idle", "Idle", "s-idle"))))
        assertEquals(0, SwitcherInbox.pendingApprovals(null))
    }
}

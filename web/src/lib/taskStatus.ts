import type { TaskStatus } from "../api/types";

export type TaskPhase = "simple" | "plan" | "coding" | "cross-review" | "done";

/**
 * Get the phase a task status belongs to
 */
export function getTaskPhase(status: TaskStatus): TaskPhase {
  switch (status) {
    case "backlog":
    case "todo":
      return "simple";

    case "plan-pending":
    case "plan-in-progress":
    case "plan-review":
    case "plan-done":
      return "plan";

    case "coding-pending":
    case "coding-in-progress":
    case "coding-review":
    case "coding-done":
    case "in-progress":
    case "in-review":
      return "coding";

    case "cross-review-pending":
    case "cross-review-in-progress":
    case "cross-review-pass":
    case "cross-review-fail":
      return "cross-review";

    case "done":
      return "done";

    default:
      return "simple";
  }
}

/**
 * Check if the status represents active work (not pending/terminal)
 */
export function isWorkingStatus(status: TaskStatus): boolean {
  return [
    "plan-in-progress",
    "plan-review",
    "coding-in-progress",
    "coding-review",
    "cross-review-in-progress",
    "in-progress",
    "in-review",
  ].includes(status);
}

/**
 * Check if task uses the agile workflow
 */
export function isAgileTask(status: TaskStatus): boolean {
  return !["backlog", "todo", "done"].includes(status);
}

/**
 * Check if the status is a terminal state
 */
export function isTerminalStatus(status: TaskStatus): boolean {
  return status === "done" || status === "cross-review-pass";
}

/**
 * Get the next logical status in the workflow
 */
export function getNextStatus(status: TaskStatus): TaskStatus | null {
  const transitions: Partial<Record<TaskStatus, TaskStatus>> = {
    // Simple flow
    backlog: "todo",
    todo: "done",

    // Plan phase
    "plan-pending": "plan-in-progress",
    "plan-in-progress": "plan-review",
    "plan-review": "plan-done",
    "plan-done": "coding-pending",

    // Coding phase
    "coding-pending": "coding-in-progress",
    "coding-in-progress": "coding-review",
    "coding-review": "coding-done",
    "coding-done": "cross-review-pending",

    // Cross-review phase
    "cross-review-pending": "cross-review-in-progress",
    "cross-review-in-progress": "cross-review-pass",
    "cross-review-pass": "done",

    // Legacy
    "in-progress": "in-review",
    "in-review": "done",
  };

  return transitions[status] ?? null;
}

/**
 * Get display label for a status
 */
export function getStatusLabel(status: TaskStatus): string {
  const labels: Record<TaskStatus, string> = {
    backlog: "Backlog",
    todo: "Todo",
    "plan-pending": "Plan Pending",
    "plan-in-progress": "Planning",
    "plan-review": "Plan Review",
    "plan-done": "Plan Done",
    "coding-pending": "Coding Pending",
    "coding-in-progress": "Coding",
    "coding-review": "Code Review",
    "coding-done": "Coding Done",
    "cross-review-pending": "CR Pending",
    "cross-review-in-progress": "Cross Review",
    "cross-review-pass": "CR Passed",
    "cross-review-fail": "CR Failed",
    done: "Done",
    "in-progress": "In Progress",
    "in-review": "In Review",
  };

  return labels[status] ?? status;
}

/**
 * Get phase display label
 */
export function getPhaseLabel(phase: TaskPhase): string {
  const labels: Record<TaskPhase, string> = {
    simple: "Simple",
    plan: "Plan",
    coding: "Coding",
    "cross-review": "Cross Review",
    done: "Done",
  };

  return labels[phase];
}

/**
 * Get status color classes for badges
 */
export function getStatusColorClasses(status: TaskStatus): string {
  const phase = getTaskPhase(status);

  switch (phase) {
    case "plan":
      return "bg-purple-50 text-purple-700 border-purple-200";
    case "coding":
      return "bg-blue-50 text-blue-700 border-blue-200";
    case "cross-review":
      if (status === "cross-review-fail") {
        return "bg-red-50 text-red-700 border-red-200";
      }
      if (status === "cross-review-pass") {
        return "bg-green-50 text-green-700 border-green-200";
      }
      return "bg-amber-50 text-amber-700 border-amber-200";
    case "done":
      return "bg-green-50 text-green-700 border-green-200";
    default:
      return "bg-slate-50 text-slate-600 border-slate-200";
  }
}

/**
 * All status values for each phase
 */
export const PHASE_STATUSES: Record<TaskPhase, TaskStatus[]> = {
  simple: ["backlog", "todo"],
  plan: ["plan-pending", "plan-in-progress", "plan-review", "plan-done"],
  coding: [
    "coding-pending",
    "coding-in-progress",
    "coding-review",
    "coding-done",
    "in-progress",
    "in-review",
  ],
  "cross-review": [
    "cross-review-pending",
    "cross-review-in-progress",
    "cross-review-pass",
    "cross-review-fail",
  ],
  done: ["done"],
};

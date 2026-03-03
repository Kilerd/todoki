import { NavLink } from "react-router-dom";
import { Separator } from "@/components/ui/separator";
import { Button } from "@/components/ui/button";
import { FolderKanban } from "lucide-react";
import { useTasks, useBacklogTasks } from "../hooks/useTasks";
import { ProjectManageModal } from "../modals/ProjectManageModal";
import { useState } from "react";

function NavBar() {
  const { tasks } = useTasks();
  const { tasks: laterTasks } = useBacklogTasks();
  const [showProjectManageModal, setShowProjectManageModal] = useState(false);

  const todoTasksNumber = tasks.filter(
    (item) => item.archived === false && item.status === "todo"
  ).length;

  const laterTasksNumber = laterTasks.filter(
    (item) => item.archived === false
  ).length;

  return (
    <>
      <div className="flex justify-between items-end m-4">
        <div className="flex items-end gap-4">
          <NavLink
            to="/inbox"
            className={({ isActive }) =>
              `decoration-none ${
                isActive ? "text-3xl text-gray-900" : " text-lg  text-gray-500"
              }`
            }
          >
            {todoTasksNumber > 0 ? `Inbox (${todoTasksNumber})` : "Inbox"}
          </NavLink>

          <NavLink
            to="/later"
            className={({ isActive }) =>
              `decoration-none ${
                isActive ? "text-3xl text-gray-900" : " text-lg  text-gray-500"
              }`
            }
          >
            {laterTasksNumber > 0 ? `Later (${laterTasksNumber})` : "Later"}
          </NavLink>

          <NavLink
            to="/projects"
            className={({ isActive }) =>
              `decoration-none ${
                isActive ? "text-3xl text-gray-900" : " text-lg  text-gray-500"
              }`
            }
          >
            Projects
          </NavLink>

          <NavLink
            to="/report"
            className={({ isActive }) =>
              `decoration-none ${
                isActive ? "text-3xl text-gray-900" : " text-lg  text-gray-500"
              }`
            }
          >
            Report
          </NavLink>

          <NavLink
            to="/agents"
            className={({ isActive }) =>
              `decoration-none ${
                isActive ? "text-3xl text-gray-900" : " text-lg  text-gray-500"
              }`
            }
          >
            Agents
          </NavLink>

          <NavLink
            to="/events"
            className={({ isActive }) =>
              `decoration-none ${
                isActive ? "text-3xl text-gray-900" : " text-lg  text-gray-500"
              }`
            }
          >
            Events
          </NavLink>
        </div>

        <Button
          variant="ghost"
          size="icon"
          onClick={() => setShowProjectManageModal(true)}
          title="Manage Projects"
        >
          <FolderKanban className="h-5 w-5" />
        </Button>
      </div>
      <Separator />

      <ProjectManageModal
        open={showProjectManageModal}
        onOpenChange={setShowProjectManageModal}
      />
    </>
  );
}

export default NavBar;

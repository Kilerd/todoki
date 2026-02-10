import { NavLink } from "react-router-dom";
import { Separator } from "@/components/ui/separator";
import { useTasks, useBacklogTasks } from "../hooks/useTasks";

function NavBar() {
  const { tasks } = useTasks();
  const { tasks: laterTasks } = useBacklogTasks();

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
        </div>
      </div>
      <Separator />
    </>
  );
}

export default NavBar;

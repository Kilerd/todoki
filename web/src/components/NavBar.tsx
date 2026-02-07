import { NavLink } from "react-router-dom";
import { Separator } from "@/components/ui/separator";
import { useTasks, useBacklogTasks } from "../hooks/useTasks";

function NavBar() {
  const { tasks } = useTasks();
  const { tasks: backlogTasks } = useBacklogTasks();

  const todoTasksNumber = tasks.filter(
    (item) => item.archived === false && item.status === "todo"
  ).length;

  const backlogTasksNumber = backlogTasks.filter(
    (item) => item.archived === false
  ).length;

  return (
    <>
      <div className="flex justify-between items-end m-4">
        <div className="flex items-end gap-4">
          <NavLink
            to="/tasks"
            className={({ isActive }) =>
              `decoration-none ${
                isActive ? "text-3xl text-gray-900" : " text-lg  text-gray-500"
              }`
            }
          >
            {todoTasksNumber > 0 ? `Today (${todoTasksNumber})` : "Today"}
          </NavLink>

          <NavLink
            to="/backlog"
            className={({ isActive }) =>
              `decoration-none ${
                isActive ? "text-3xl text-gray-900" : " text-lg  text-gray-500"
              }`
            }
          >
            {backlogTasksNumber > 0 ? `Backlog (${backlogTasksNumber})` : "Backlog"}
          </NavLink>

          <NavLink
            to="/timeline"
            className={({ isActive }) =>
              `decoration-none ${
                isActive ? "text-3xl text-gray-900" : " text-lg  text-gray-500"
              }`
            }
          >
            Timeline
          </NavLink>
        </div>
      </div>
      <Separator />
    </>
  );
}

export default NavBar;

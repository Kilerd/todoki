import { NavLink } from "react-router-dom";
import { filter } from "lodash";
import { Separator } from "@/components/ui/separator";
import { useTasks } from "../hooks/useTasks";

function NavBar() {
  const { tasks } = useTasks();

  const todoTasksNumber = filter(
    tasks,
    (item) => item.archived === false && item.done === false
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
            {todoTasksNumber > 0 ? `今日待办(${todoTasksNumber})` : "今日待办"}
          </NavLink>

          <NavLink
            to="/timeline"
            className={({ isActive }) =>
              `decoration-none ${
                isActive ? "text-3xl text-gray-900" : " text-lg  text-gray-500"
              }`
            }
          >
            时间线
          </NavLink>
        </div>
      </div>
      <Separator />
    </>
  );
}

export default NavBar;

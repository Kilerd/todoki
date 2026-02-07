import { NavLink } from 'react-router-dom'
import CanAccess from "./CanAccess";
import useUserSession from "../hooks/useUserSession";
import { LogOut } from "lucide-react";
import useSWR from "swr";
import { fetcher } from "../services/api";
import { filter } from "lodash";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";

function NavBar() {
    const { isAuthenticated, user, signOut } = useUserSession()
    const { data: tasks } = useSWR(isAuthenticated ? "/tasks" : null, fetcher);

    const todoTasksNumber = filter(tasks ?? [], (item: any) => item.archived === false && item.done === false).length
    return (
        <>
            <div className="flex justify-between items-end m-4">
                <div className="flex items-end gap-4">
                    <NavLink to="/tasks" className={({ isActive }) =>
                        `decoration-none ${isActive ? 'text-3xl text-gray-900' : ' text-lg  text-gray-500'}`
                    }>今日待办{todoTasksNumber > 0 ? `(${todoTasksNumber})` : ""}</NavLink>
                    
                    <NavLink to="/timeline" className={({ isActive }) =>
                        `decoration-none ${isActive ? 'text-3xl text-gray-900' : ' text-lg  text-gray-500'}`
                    }>时间线</NavLink>
                    
                    <NavLink to="/habits" className={({ isActive }) =>
                        `decoration-none ${isActive ? 'text-3xl text-gray-900' : ' text-lg  text-gray-500'}`
                    }>习惯</NavLink>

                    <CanAccess permissions={['users.list']}>
                        <NavLink to="/users">Users</NavLink>
                    </CanAccess>

                    <CanAccess permissions={['metrics.list']}>
                        <NavLink to="/metrics">Metrics</NavLink>
                    </CanAccess>
                </div>

                {isAuthenticated && (
                    <div className="flex items-center gap-4">
                        <NavLink to="/profile" className="text-lg text-gray-900 no-underline">
                            {user?.username}
                        </NavLink>
                        <Button variant="ghost" size="icon" onClick={() => signOut("/")}>
                            <LogOut className="h-4 w-4" />
                        </Button>
                    </div>
                )}
            </div>
            <Separator />
        </>
    )
}

export default NavBar

import {Route, Routes} from 'react-router-dom'

import PrivateRoute from './PrivateRoute'
import PublicRoute from './PublicRoute'
import Today from "../pages/Today";
import Login from "../pages/Login";
import Homepage from "../pages/Homepage";
import Timeline from "../pages/Timeline";
import Habits from "../pages/Habits";
import TaskDetail from "../pages/TaskDetail";
import NotFoundTitle from "../pages/NotFound";
import Profile from "../pages/Profile";

function RouteList() {
    return (
        <Routes>
            <Route path="/" element={<Homepage/>}/>

            <Route
                path="/tasks"
                element={
                    <PrivateRoute redirectTo="/login">
                        <Today/>
                    </PrivateRoute>
                }
            />
            <Route
                path="/tasks/:id"
                element={
                    <PrivateRoute>
                        <TaskDetail/>
                    </PrivateRoute>
                }
            />

            <Route
                path="/login"
                element={
                    <PublicRoute to={"/tasks"}>
                        <Login/>
                    </PublicRoute>
                }
            />
            <Route
                path="/timeline"
                element={
                    <PrivateRoute redirectTo="/login">
                        <Timeline/>
                    </PrivateRoute>
                }
            />
            <Route
                path="/habits"
                element={
                    <PrivateRoute redirectTo="/login">
                        <Habits/>
                    </PrivateRoute>
                }
            />
            <Route
                path="/profile"
                element={
                    <PrivateRoute redirectTo="/login">
                        <Profile/>
                    </PrivateRoute>
                }
            />


            <Route path="*" element={<NotFoundTitle/>}/>
        </Routes>
    )
}

export default RouteList

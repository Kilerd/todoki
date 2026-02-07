import { Navigate, Route, Routes } from "react-router-dom";

import Today from "../pages/Today";
import Timeline from "../pages/Timeline";
import TaskDetail from "../pages/TaskDetail";
import NotFoundTitle from "../pages/NotFound";

function RouteList() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/tasks" replace />} />
      <Route path="/tasks" element={<Today />} />
      <Route path="/tasks/:id" element={<TaskDetail />} />
      <Route path="/timeline" element={<Timeline />} />
      <Route path="*" element={<NotFoundTitle />} />
    </Routes>
  );
}

export default RouteList;

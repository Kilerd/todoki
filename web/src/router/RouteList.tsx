import { Navigate, Route, Routes } from "react-router-dom";

import Inbox from "../pages/Inbox";
import Later from "../pages/Later";
import Report from "../pages/Report";
import TaskDetail from "../pages/TaskDetail";
import NotFoundTitle from "../pages/NotFound";

function RouteList() {
  return (
    <Routes>
      <Route path="/" element={<Navigate to="/inbox" replace />} />
      <Route path="/inbox" element={<Inbox />} />
      <Route path="/inbox/:id" element={<TaskDetail />} />
      <Route path="/later" element={<Later />} />
      <Route path="/report" element={<Report />} />
      <Route path="*" element={<NotFoundTitle />} />
    </Routes>
  );
}

export default RouteList;

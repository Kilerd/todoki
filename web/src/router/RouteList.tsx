import { Navigate, Route, Routes } from "react-router-dom";

import Agents from "../pages/Agents";
import EventsPage from "../pages/EventsPage";
import EventTimelineDemo from "../pages/EventTimelineDemo";
import Inbox from "../pages/Inbox";
import Later from "../pages/Later";
import Projects from "../pages/Projects";
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
      <Route path="/projects" element={<Projects />} />
      <Route path="/report" element={<Report />} />
      <Route path="/agents" element={<Agents />} />
      <Route path="/events" element={<EventsPage />} />
      <Route path="/events/demo" element={<EventTimelineDemo />} />
      <Route path="*" element={<NotFoundTitle />} />
    </Routes>
  );
}

export default RouteList;

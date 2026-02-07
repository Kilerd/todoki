import RouteList from "@/router/RouteList";
import { Toaster } from './components/ui/toaster';
import AuthProvider from './providers/AuthProvider';

function App() {

  return (
      <AuthProvider>
        <RouteList/>
        <Toaster />
      </AuthProvider>
  )
}

export default App

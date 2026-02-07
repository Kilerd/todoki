import { useEffect, useState } from "react";
import RouteList from "@/router/RouteList";
import { Toaster } from "./components/ui/toaster";
import TokenInput from "./pages/TokenInput";
import { getToken, validateToken, clearToken } from "./lib/auth";

function App() {
  const [isAuthenticated, setIsAuthenticated] = useState<boolean | null>(null);

  useEffect(() => {
    const checkAuth = async () => {
      const token = getToken();
      if (!token) {
        setIsAuthenticated(false);
        return;
      }

      const isValid = await validateToken(token);
      if (!isValid) {
        clearToken();
        setIsAuthenticated(false);
      } else {
        setIsAuthenticated(true);
      }
    };

    checkAuth();
  }, []);

  if (isAuthenticated === null) {
    return null; // Loading state
  }

  if (!isAuthenticated) {
    return <TokenInput onSuccess={() => setIsAuthenticated(true)} />;
  }

  return (
    <>
      <RouteList />
      <Toaster />
    </>
  );
}

export default App;

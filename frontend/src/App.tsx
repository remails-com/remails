import { useEffect, useState } from "react";
import { Login } from "./Login";
import { WhoamiResponse } from "./types";
import Dashboard from "./Dashboard";

export default function App() {
  const [user, setUser] = useState<WhoamiResponse | null>(null);
  const [loading, setLoading] = useState(true);

  // check whether the user is logged in
  useEffect(() => {
    setLoading(true);
    fetch("/api/whoami")
      .then((res) => res.json())
      .then((data) => {
        if (data.role) {
          setUser(data);
        }
        setLoading(false);
      });
  }, []);

  if (loading) {
    return <div>Loading...</div>;
  }

  if (!user) {
    return <Login />;
  }

  return <Dashboard />
}

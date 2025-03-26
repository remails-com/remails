import { createContext, useContext, useEffect, useState } from "react";
import { User, WhoamiResponse } from "../types";

export const UserContext = createContext<WhoamiResponse | null>(null)

export function useUser(): User {
  const user = useContext(UserContext);

  if (!user) {
    throw new Error("useUser must be used within a UserProvider");
  }

  if ('error' in user) {
    throw new Error(user.error);
  }

  return user;
}

export function useLoadUser() {
  const [user, setUser] = useState<WhoamiResponse | null>(null);
  const [loading, setLoading] = useState(true);

  // check whether the user is logged in
  useEffect(() => {
    setLoading(true);
    fetch("/api/whoami")
      .then((res) => res.json())
      .then((data) => {
        if (data.roles) {
          setUser(data);
        }
        setLoading(false);
      });
  }, []);

  return { user, loading }
}
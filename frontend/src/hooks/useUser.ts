import {createContext, useContext, useEffect, useState} from "react";
import {User, WhoamiResponse} from "../types";

export const UserContext = createContext<{ user: WhoamiResponse, setUser: (user: WhoamiResponse) => void } | null>(null)

export function useUser(): { user: User, setUser: (user: WhoamiResponse) => void } {
  const context = useContext(UserContext);

  if (!context) {
    throw new Error("useUser must be used within a UserProvider");
  }

  if ('error' in context.user) {
    throw new Error(context.user.error);
  }

  return {user: context.user, setUser: context.setUser};
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

  return {user, loading, setUser}
}
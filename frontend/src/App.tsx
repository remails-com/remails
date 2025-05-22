import { Loader } from "@mantine/core";
import { Login } from "./Login";
import { Pages } from "./Pages";
import { useLoadUser, UserContext } from "./hooks/useUser";
import { RemailsContext, useLoadRemails } from "./hooks/useRemails.ts";

export default function App() {
  const { user, loading, setUser } = useLoadUser();
  const { state, dispatch, navigate } = useLoadRemails(user);

  if (loading) {
    return <Loader color="gray" size="xl" type="dots" />;
  }

  if (!user) {
    return <Login setUser={setUser} />;
  }

  return (
    <UserContext.Provider value={{ user, setUser }}>
      <RemailsContext.Provider value={{ state, dispatch, navigate }}>
        <Pages />
      </RemailsContext.Provider>
    </UserContext.Provider>
  );
}

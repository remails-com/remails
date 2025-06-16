import { Stack } from "@mantine/core";
import { Login } from "./Login";
import { Pages } from "./Pages";
import { useLoadUser, UserContext } from "./hooks/useUser";
import { RemailsContext, useLoadRemails } from "./hooks/useRemails.ts";
import { Loader } from "./Loader.tsx";

export default function App() {
  const { user, loading, setUser } = useLoadUser();
  const { state, dispatch, navigate } = useLoadRemails(user);

  if (loading) {
    return (
      <Stack p="xl" align="center" justify="center" h="100vh">
        <Loader />
      </Stack>
    );
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

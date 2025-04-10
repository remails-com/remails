import { Loader } from "@mantine/core";
import { Login } from "./Login";
import { Pages } from "./Pages";
import { useInitRouter, RouterContext } from "./hooks/useRouter";
import { UserContext, useLoadUser } from "./hooks/useUser";

export default function App() {
  const { user, loading, invalidate } = useLoadUser();
  const {
    params,
    route,
    navigate
  } = useInitRouter();

  if (loading) {
    return <Loader color="gray" size="xl" type="dots" />;
  }

  if (!user) {
    return <Login reevaluate={invalidate} />;
  }

  return (
    <RouterContext.Provider value={{ params, route, navigate }}>
      <UserContext.Provider value={user}>
        <Pages />
      </UserContext.Provider>
    </RouterContext.Provider>
  );
}

import {Loader} from "@mantine/core";
import {Login} from "./Login";
import {Pages} from "./Pages";
import {RouterContext, useInitRouter} from "./hooks/useRouter.ts";
import {useLoadUser, UserContext} from "./hooks/useUser";
import {RemailsContext, useLoadRemails} from "./hooks/useRemails.ts";

export default function App() {
  const {user, loading, setUser} = useLoadUser();
  const {state, dispatch} = useLoadRemails();
  const {
    params,
    route,
    navigate,
    fullPath,
    fullName,
    breadcrumbItems
  } = useInitRouter();

  if (loading) {
    return <Loader color="gray" size="xl" type="dots"/>;
  }

  if (!user) {
    return <Login setUser={setUser}/>;
  }

  return (
    <RouterContext.Provider value={{params, route, navigate, fullPath, fullName, breadcrumbItems}}>
      <UserContext.Provider value={user}>
        <RemailsContext.Provider value={{state, dispatch}}>
          <Pages/>
        </RemailsContext.Provider>
      </UserContext.Provider>
    </RouterContext.Provider>
  );
}

import {Loader} from "@mantine/core";
import {Login} from "./Login";
import {Pages} from "./Pages";
import {RouterContext, useInitRouter} from "./hooks/useRouter";
import {useLoadUser, UserContext} from "./hooks/useUser";
import {OrganizationContext, useLoadOrganizations} from "./hooks/useOrganizations.ts";

export default function App() {
  const {user, loading, setUser} = useLoadUser();
  const {currentOrganization, setCurrentOrganization, organizations} = useLoadOrganizations();
  const {
    params,
    route,
    navigate
  } = useInitRouter();

  if (loading) {
    return <Loader color="gray" size="xl" type="dots"/>;
  }

  if (!user) {
    return <Login setUser={setUser}/>;
  }

  return (
    <RouterContext.Provider value={{params, route, navigate}}>
      <UserContext.Provider value={user}>
        <OrganizationContext.Provider value={{currentOrganization, setCurrentOrganization, organizations}}>
          <Pages/>
        </OrganizationContext.Provider>
      </UserContext.Provider>
    </RouterContext.Provider>
  );
}

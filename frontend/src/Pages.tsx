import { Dashboard } from "./layout/Dashboard";
import { useRemails } from "./hooks/useRemails.ts";
import { Setup } from "./components/Setup.tsx";
import { NavigationProgress } from "@mantine/nprogress";
import Login from "./Login.tsx";
import NotFound from "./components/NotFound.tsx";

function Page() {
  const {
    state: { organizations, routerState },
    render,
  } = useRemails();
  const routeName = routerState.name;

  if (organizations?.length === 0) {
    return <Setup />;
  }

  return render(routeName);
}

export function Pages() {
  const {
    state: {
      userFetched,
      routerState: { name },
    },
    dispatch,
  } = useRemails();

  if (!userFetched) {
    return <NavigationProgress />;
  }

  if (name === "login") {
    return <Login setUser={(user) => dispatch({ type: "set_user", user })} />;
  }

  if (name === "not_found") {
    return <NotFound />;
  }

  return (
    <Dashboard>
      <NavigationProgress />
      <Page />
    </Dashboard>
  );
}

import { Login } from "./Login";
import { Pages } from "./Pages";
import { RemailsContext, useLoadRemails } from "./hooks/useRemails.ts";

export default function App() {
  const { state, dispatch, navigate } = useLoadRemails();

  if (!state.user) {
    return <Login setUser={(user) => dispatch({ type: 'set_user', user})} />;
  }

  return (
    <RemailsContext.Provider value={{ state, dispatch, navigate }}>
      <Pages />
    </RemailsContext.Provider>
  );
}

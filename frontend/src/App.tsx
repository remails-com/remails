import { Pages } from "./Pages";
import { RemailsContext, useLoadRemails } from "./hooks/useRemails.ts";

export default function App() {
  const { state, dispatch, navigate } = useLoadRemails();

  return (
    <RemailsContext.Provider value={{ state, dispatch, navigate }}>
      <Pages />
    </RemailsContext.Provider>
  );
}

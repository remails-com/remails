import { Pages } from "./Pages";
import { RemailsContext, useLoadRemails } from "./hooks/useRemails.ts";

export default function App() {
  const { state, dispatch, navigate, render } = useLoadRemails();

  return (
    <RemailsContext.Provider value={{ state, dispatch, navigate, render }}>
      <Pages />
    </RemailsContext.Provider>
  );
}

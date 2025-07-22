import ErrorBoundary from "./error/ErrorBoundary.tsx";
import { Pages } from "./Pages";
import { RemailsContext, useLoadRemails } from "./hooks/useRemails.ts";

function LoadRemails() {
  const { state, dispatch, navigate } = useLoadRemails();

  return (
    <RemailsContext.Provider value={{ state, dispatch, navigate }}>
      <Pages />
    </RemailsContext.Provider>
  );
}

export default function App() {
  return (
    <ErrorBoundary>
      <LoadRemails />
    </ErrorBoundary>
  );
}

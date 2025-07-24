import ErrorBoundary from "./error/ErrorBoundary.tsx";
import { Pages } from "./Pages";
import { RemailsContext, useLoadRemails } from "./hooks/useRemails.ts";
import { NavigationProgress } from "@mantine/nprogress";
import React from "react";

const LazyNavBoundary = React.memo(
  function NavBoundary(remails: ReturnType<typeof useLoadRemails>) {
    return (
      <RemailsContext.Provider value={remails}>
        <Pages />
      </RemailsContext.Provider>
    );
  },
  (_prev, next) => next.state.nextRouterState !== null
);

function LoadRemails() {
  const remails = useLoadRemails();
  return <LazyNavBoundary {...remails} />;
}

export default function App() {
  return (
    <ErrorBoundary>
      <NavigationProgress />
      <LoadRemails />
    </ErrorBoundary>
  );
}

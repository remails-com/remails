import { Dispatch, useCallback, useEffect } from "react";
import { RouteParams, Router } from "../router";
import { Action } from "../types";

export function useRouter(router: Router, dispatch: Dispatch<Action>) {
  // handle back / forward events
  useEffect(() => {
    const onPopState = (event: PopStateEvent) => {
      if (event.state?.routerState) {
        dispatch({
          type: "set_route",
          routerState: event.state.routerState,
        });
      } else {
        dispatch({
          type: "set_route",
          routerState: router.initialState,
        });
      }
    };

    window.addEventListener("popstate", onPopState);

    return () => {
      window.removeEventListener("popstate", onPopState);
    };
  }, [dispatch, router]);

  // Navigate function to change the route
  return useCallback(
    (name: string, params?: RouteParams) => {
      const routerState = router.navigate(name, params || {});

      window.history.pushState(routerState, "", routerState.fullPath);

      dispatch({
        type: "set_route",
        routerState: {
          name: routerState.name,
          params: routerState.params,
        },
      });
    },
    [dispatch, router]
  );
}

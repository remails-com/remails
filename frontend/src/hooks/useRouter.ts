import { Dispatch, useEffect, useRef } from "react";
import { FullRouterState, RouteParams, Router, RouterState } from "../router";
import { Action, State } from "../types";

export interface NavigationState {
  from: RouterState;
  to: FullRouterState;
  state: State;
}

export type Middleware = (
  navstate: NavigationState,
  router: Router,
  dispatch: Dispatch<Action>
) => Promise<FullRouterState>;

export function useRouter(router: Router, state: State, dispatch: Dispatch<Action>, middleware: Middleware[] = []) {
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

  const busy = useRef(false);

  // Navigate function to change the route
  const navigate = async (name: string, params?: RouteParams) => {
    if (busy.current) {
      console.warn("Navigation is already in progress, ignoring new request.");
      return false;
    }

    busy.current = true;
    dispatch({ type: "loading", loading: true });

    let routerState = router.navigate(name, params || {});

    const navState: NavigationState = {
      from: state.routerState,
      to: routerState,
      state,
    };

    for (const mw of middleware) {
      routerState = await mw(navState, router, dispatch);
    }

    window.history.pushState(routerState, "", routerState.fullPath);

    dispatch({
      type: "set_route",
      routerState: {
        name: routerState.name,
        params: routerState.params,
      },
    });

    dispatch({ type: "loading", loading: false });
    busy.current = false;

    return true;
  };

  return navigate;
}

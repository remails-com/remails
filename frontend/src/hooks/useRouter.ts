import { nprogress } from "@mantine/nprogress";
import { Dispatch, useCallback, useEffect, useRef } from "react";
import { FullRouterState, RouteParams, Router, RouterState } from "../router";
import { RouteName } from "../routes";
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
  const busy = useRef(false);

  // Navigate function to change the route
  const navigate = useCallback(
    async (name: RouteName, params?: RouteParams, pushState = true) => {
      if (busy.current) {
        console.warn("Navigation is already in progress, ignoring new request.");
        return false;
      }

      busy.current = true;
      let routerState;
      try {
        routerState = router.navigate(name, params || {});
      } catch (e) {
        console.error(e);
        routerState = router.navigate("not_found", {});
      }

      dispatch({
        type: "set_next_router_state",
        nextRouterState: routerState,
      });

      setTimeout(() => {
        if (busy.current) {
          nprogress.start();
        }
      }, 100);

      const navState: NavigationState = {
        from: state.routerState,
        to: routerState,
        state,
      };

      for (const mw of middleware) {
        try {
          routerState = await mw(navState, router, dispatch);
        } catch (e) {
          console.error(e);
          routerState = router.navigate("not_found", {});
        }
      }

      if (pushState) {
        window.history.pushState({ routerState }, "", routerState.fullPath);
      }

      dispatch({
        type: "set_route",
        routerState: {
          name: routerState.name,
          params: routerState.params,
        },
      });

      busy.current = false;
      nprogress.complete();

      return true;
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [state.routerState, dispatch, router, middleware]
  );

  // handle back / forward events
  useEffect(() => {
    const onPopState = async (event: PopStateEvent) => {
      if (event.state?.routerState) {
        await navigate(event.state.routerState.name, event.state.routerState.params, false);
      } else {
        await navigate(router.initialState.name, router.initialState.params, false);
      }
    };

    window.addEventListener("popstate", onPopState);

    return () => {
      window.removeEventListener("popstate", onPopState);
    };
  }, [dispatch, router, navigate]);

  return { navigate };
}

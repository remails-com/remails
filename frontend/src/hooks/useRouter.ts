import { createContext, useContext, useEffect, useState } from 'react';

export type RouteName = string;
export type RouteParams = Record<string, string>;
export type Navigate = (name: RouteName, params?: RouteParams) => void;

export interface Route {
  name: RouteName;
  path: string;
  children?: Route[];
}

export interface RouterContextProps {
  route: Route;
  params: RouteParams;
  navigate: Navigate;
}

export const routes: Route[] = [
  {
    name: 'projects',
    path: '/projects',
    children: [
      {
        name: 'proj-domains',
        path: '/projects/{id}/domains',
      },
      {
        name: 'streams',
        path: 'projects/{proj_id}/streams',
        children: [
          {
            name: 'credentials',
            path: '/projects/{proj_id}/streams/{stream_id}/credentials',
          },
          {
            name: 'message-log',
            path: '/projects/{proj_id}/streams/{stream_id}/messages',
          },
        ]
      }
    ]
  },
  {
    name: 'domains',
    path: '/domains',
  },
  {
    name: 'organizations',
    path: '/organizations',
    children: [
      {
        name: 'add',
        path: '/add',
      },
      {
        name: 'edit',
        path: '/edit/{id}',
      }
    ]
  },
];

export function matchPath(path: string): Route {
  return routes.slice(0).reverse().find((r) => path.endsWith(r.path)) || routes[0];
}

export function matchName(name: RouteName): Route {
    return recursiveMatchName(routes, name) || routes[0];
}

function recursiveMatchName(routes: Route[], name: RouteName): Route | null {
  for (const route of routes) {
    if (route.name === name) {
      return route;
    } else if (route.children) {
      const next_level = recursiveMatchName(route.children, name);
      if (next_level) {
        return next_level;
      }
    }
  }
  return null;
}

export const RouterContext = createContext<RouterContextProps>({
  route: routes[0],
  params: {},
  navigate: () => {},
});

export function useRouter(): RouterContextProps {
  return useContext(RouterContext);
}

export function useInitRouter(): RouterContextProps {
  const currentPath = window.location.pathname;
  const [route, setRoute] = useState<Route>(matchPath(currentPath));

  const queryString = new URLSearchParams(window.location.search);
  const [params, setParams] = useState<RouteParams>(
    Object.fromEntries(queryString)
  );

  // handle back / forward events
  useEffect(() => {
    window.addEventListener('popstate', (event) => {
      if (event.state?.routeName) {
        setRoute(matchName(event.state.routeName));
      } else {
        setRoute(matchPath(window.location.pathname));
        setParams({});
      }
      if (event.state?.routeParams) {
        setParams(event.state.routeParams);
      }
    });
  }, []);

  // navigate to a new route
  const navigate = (name: RouteName, params: RouteParams = {}) => {
    const newRoute = matchName(name);

    if (params && Object.keys(params).length > 0) {
      const searchParams = new URLSearchParams(params);
      const fullPath = `.${newRoute.path}?${searchParams}`;
      window.history.pushState(
        { routeName: name, routeParams: params },
        '',
        fullPath
      );
      setRoute(newRoute);
      setParams(params);
    } else {
      window.history.pushState(
        { routeName: name, routeParams: {} },
        '',
        `.${newRoute.path}`
      );
      setRoute(newRoute);
      setParams({});
    }
  };

  return {
    params,
    route,
    navigate,
  };
}
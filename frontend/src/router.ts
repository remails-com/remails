export type RouteName = string;
export type RoutePath = string;
export type RouteParams = Record<string, string>;
export type Navigate = (name: RouteName, params?: RouteParams) => void;

export interface Route {
  name: RouteName;
  path: string;
  children?: Route[];
}

export interface RouterState {
  name: string;
  params: { [k: string]: string };
}

export const allRoutes: Route[] = [
  {
    name: "projects",
    path: "/{org_id}/projects",
    children: [
      {
        name: "project",
        path: "/{proj_id}",
        children: [
          {
            name: "domains",
            path: "/domains",
            children: [
              {
                name: "domain",
                path: "/{domain_id}",
              },
            ],
          },
          {
            name: "streams",
            path: "/streams",
            children: [
              {
                name: "stream",
                path: "/{stream_id}",
                children: [
                  {
                    name: "credentials",
                    path: "/credentials",
                    children: [
                      {
                        name: "credential",
                        path: "/{credential_id}",
                      },
                    ],
                  },
                  {
                    name: "messages",
                    path: "/messages",
                    children: [
                      {
                        name: "message",
                        path: "/{message_id}",
                      },
                    ],
                  },
                ],
              },
            ],
          },
        ],
      },
    ],
  },
  {
    name: "domains",
    path: "/{org_id}/domains",
    children: [
      {
        name: "domain",
        path: "/{domain_id}",
      },
    ],
  },
  {
    name: "settings",
    path: "/{org_id}/settings",
  },
  {
    name: "statistics",
    path: "/{org_id}/statistics",
  },
  {
    name: "organizations",
    path: "/{org_id}/organizations",
  },
];

export function matchPath(
  routes: Route[],
  path: string
): {
  params: RouteParams;
  name: string;
} | null {
  const match = matchPathRecursive(path, routes, {}, []);

  const queryString = path.split("?")[1];
  const query = Object.fromEntries(new URLSearchParams(queryString));

  if (match) {
    return {
      params: { ...match.params, ...query },
      name: match.name.join("."),
    };
  }

  return null;
}

function matchPathRecursive(
  path: string,
  routes: Route[],
  params: RouteParams,
  name: string[]
): {
  route: Route;
  params: RouteParams;
  name: string[];
} | null {
  path = path.replace(/^\/|\/$/, "").replace(/\?.*$/, ""); // Remove leading/trailing slashes and query params
  const path_elems = path.split("/");

  route_loop: for (const route of routes) {
    const new_path_params: { [k: string]: string } = {};
    const route_path = route.path.replace(/^\/|\/$/, "");
    const route_elems = route_path.split("/");
    for (const [index, route_elem] of route_elems.entries()) {
      if (route_elem !== path_elems[index]) {
        const path_var = route_elem.match(/^{(\w*)}$/)?.at(1);
        if (path_var) {
          new_path_params[path_var] = path_elems[index];
        } else {
          continue route_loop;
        }
      }
    }

    const new_full_name = [...name, route.name];
    if (route.children && path_elems.length > route_elems.length) {
      const rec_res = matchPathRecursive(
        path_elems.slice(route_elems.length).join("/"),
        route.children,
        new_path_params,
        new_full_name
      );
      if (rec_res) {
        return {
          route: rec_res.route,
          params: { ...params, ...rec_res.params, ...new_path_params },
          name: rec_res.name,
        };
      }

      return null;
    }

    return {
      route,
      params: { ...params, ...new_path_params },
      name: new_full_name,
    };
  }

  return null;
}

export function matchName(routes: Route[], name: RoutePath): string {
  const segments = name.split(".");
  const currentSegment = segments[0];
  const route = routes.find((r) => r.name === currentSegment);

  if (!route) {
    throw new Error(`Route with name ${currentSegment} not found`);
  }

  if (segments.length === 1) {
    return route.path;
  }

  return route.path + matchName(route.children || [], segments.slice(1).join("."));
}

export function routerNavigate(name: RouteName, params: RouteParams, keepParams: RouteParams = {}): RouterState {
  const state = createRouteState(name, params, keepParams);

  const routerState = {
    name: state.name,
    params: state.params,
  };

  window.history.pushState(routerState, "", state.fullPath);

  return routerState;
}

export function createRouteState(
  name: RouteName,
  params: RouteParams,
  keepParams: RouteParams = {}
): RouterState & { fullPath: string } {
  let path = matchName(allRoutes, name);
  const query = { ...params };
  const pathParams = { ...params, ...keepParams };

  path = path.replace(/{(\w+)}/g, (_match, key) => {
    const value = pathParams[key];

    if (!value) {
      throw new Error(`Path variable ${key} not found in params`);
    }

    delete query[key];
    params[key] = value;

    return value;
  });

  if (Object.values(query).length > 0) {
    const searchParams = new URLSearchParams(query);
    path = `${path}?${searchParams}`;
  }

  return { fullPath: path, name, params };
}

export function initRouter(): RouterState {
  const currentPath = window.location.pathname + window.location.search;

  const { params, name } = matchPath(allRoutes, currentPath) || {
    params: {},
    name: allRoutes[0].name,
  };

  return {
    name,
    params,
  };
}

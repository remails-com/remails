export type RouteName = string;
export type RouteParams = Record<string, string>;
export type Navigate = (name: RouteName, params?: RouteParams) => void;

export interface Route {
  name: RouteName;
  path: string;
  children?: Route[];
}

interface FlatRoute {
  name: string;
  path: string;
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

export function flattenRoutes(routes: Route[]): FlatRoute[] {
  return routes
    .map((route) => {
      const flatRoute: FlatRoute = {
        name: route.name,
        path: route.path,
      };

      if (route.children) {
        const childRoutes = flattenRoutes(route.children);
        return [
          flatRoute,
          ...childRoutes.map((childRoute) => ({
            name: `${route.name}.${childRoute.name}`,
            path: `${route.path}${childRoute.path}`,
          })),
        ];
      }

      return flatRoute;
    })
    .flat();
}

export function matchPath(definition: string, path: string): RouteParams | null {
  let i = 0; // Definition index
  let j = 0; // Path index
  const params: RouteParams = {};

  while (i < definition.length && j < path.length) {
    // Match single characters
    if (definition[i] === path[j]) {
      i += 1;
      j += 1;
      continue;
    }

    if (definition[i] === "{") {
      const end = definition.indexOf("}", i);

      if (end === -1) {
        return null; // Invalid path, no closing brace found
      }

      let valueEnd = j;
      while (path[valueEnd] !== "/" && path[valueEnd] !== "?" && valueEnd < path.length) {
        valueEnd += 1;
      }

      const paramName = definition.slice(i + 1, end);
      const paramValue = path.slice(j, valueEnd);

      i = end + 1; // Move past the closing brace
      j = valueEnd; // Move past the parameter value

      if (!paramName) {
        return null; // Invalid path, empty parameter name
      }

      if (!paramValue) {
        return null; // Invalid path, empty parameter value
      }

      params[paramName] = paramValue;

      continue;
    }

    return null; // Mismatch found
  }

  if (i !== definition.length || j !== path.length) {
    // Allow for trailing slashes in the path
    if (j !== path.length - 1 || path[j] !== "/") {
      return null;
    }
  }

  return params;
}

export function matchRoute(
  routes: FlatRoute[],
  path: string
): {
  params: RouteParams;
  name: string;
} | null {
  const pathParts = path.split("?");
  const basePath = pathParts[0];
  const queryString = pathParts[1];
  const query = Object.fromEntries(new URLSearchParams(queryString));

  for (const route of routes) {
    const params = matchPath(route.path, basePath);

    if (params !== null) {
      return {
        name: route.name,
        params: { ...params, ...query },
      };
    }
  }

  return null;
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
  const flatRoutes = flattenRoutes(allRoutes);
  let path = flatRoutes.find((route) => route.name === name)?.path;

  if (!path) {
    throw new Error(`Route with name ${name} not found`);
  }

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
  const flatRoutes = flattenRoutes(allRoutes);

  const { params, name } = matchRoute(flatRoutes, currentPath) || {
    params: {},
    name: allRoutes[0].name,
  };

  return {
    name,
    params,
  };
}

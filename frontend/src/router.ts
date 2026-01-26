import { RouteName } from "./routes";

export type RouteParams = Record<string, string>;
export type Navigate = (name: RouteName, params?: RouteParams) => void;

export interface RouterState {
  name: RouteName;
  params: RouteParams;
}

export interface FullRouterState extends RouterState {
  fullPath: string;
}

export interface Route {
  name: string;
  path: string;
  children?: Route[];
}

interface FlatRoute {
  name: RouteName;
  path: string;
}

export function flattenRoutes(routes: Route[]): FlatRoute[] {
  return routes
    .map((route) => {
      const flatRoute: FlatRoute = {
        name: route.name as RouteName,
        path: route.path,
      };

      if (route.children) {
        const childRoutes = flattenRoutes(route.children);
        return [
          flatRoute,
          ...childRoutes.map((childRoute) => ({
            name: `${route.name}.${childRoute.name}` as RouteName,
            path: `${route.path}${childRoute.path}`,
          })),
        ];
      }

      return flatRoute;
    })
    .flat();
}

export function matchPath(definition: string, path: string): RouteParams | null {
  // definition index
  let i = 0;
  // path index
  let j = 0;
  // collected parameters
  const params: RouteParams = {};

  while (i < definition.length && j < path.length) {
    // match single characters
    if (definition[i] === path[j]) {
      i += 1;
      j += 1;
      continue;
    }

    if (definition[i] === "{") {
      const end = definition.indexOf("}", i);

      if (end === -1) {
        // invalid path definition, no closing brace
        return null;
      }

      let valueEnd = j;
      while (path[valueEnd] !== "/" && path[valueEnd] !== "?" && valueEnd < path.length) {
        valueEnd += 1;
      }

      const paramName = definition.slice(i + 1, end);
      const paramValue = path.slice(j, valueEnd);

      // move past the closing brace
      i = end + 1;

      // move past the parameter value
      j = valueEnd;

      if (!paramName) {
        // invalid path, empty parameter name
        return null;
      }

      if (!paramValue) {
        // invalid path, empty parameter value
        return null;
      }

      params[paramName] = paramValue;

      continue;
    }

    // mismatch found
    return null;
  }

  if (i !== definition.length || j !== path.length) {
    // allow for trailing slashes in the path
    if (j !== path.length - 1 || path[j] !== "/") {
      return null;
    }
  }

  return params;
}

export class Router {
  private routes: FlatRoute[];
  private pathParamCache: RouteParams = {};
  public initialState: RouterState;

  constructor(routes: Route[]) {
    this.routes = flattenRoutes(routes);
    this.initialState = {
      name: "default",
      params: {},
    };
    this.pathParamCache = this.initialState.params;
  }

  match(path: string): {
    name: RouteName;
    params: RouteParams;
  } | null {
    const pathParts = path.split("?");
    const basePath = pathParts[0];
    const queryString = pathParts[1];
    const query = Object.fromEntries(new URLSearchParams(queryString));

    for (const route of this.routes) {
      const params = matchPath(route.path, basePath);

      if (params !== null) {
        return {
          name: route.name,
          params: { ...query, ...params },
        };
      }
    }

    return null;
  }

  navigate(name: RouteName, params: RouteParams = {}, resetParamCache = true): FullRouterState {
    const route = this.routes.find((route) => route.name === name);
    if (!route) {
      throw new Error(`Route with name ${name} not found`);
    }

    let path = route.path;

    const query = Object.fromEntries(Object.entries(params).filter(([, v]) => v !== undefined));
    const pathParams = { ...this.pathParamCache, ...query };

    if (resetParamCache) {
      this.pathParamCache = {};
    }

    path = path.replace(/{(\w+)}/g, (_match, key) => {
      const value = pathParams[key];

      if (!value) {
        throw new Error(`Path variable ${key} not found in params`);
      }

      delete query[key];

      params[key] = value;
      this.pathParamCache[key] = value;

      return value;
    });

    if (Object.values(query).length > 0) {
      const searchParams = new URLSearchParams(query);
      path = `${path}?${searchParams}`;
    }

    return { fullPath: path, name, params };
  }
}

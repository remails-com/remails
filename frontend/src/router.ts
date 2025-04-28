export type RouteName = string;
export type RouteParams = Record<string, string>;
export type Navigate = (name: RouteName, pathParams?: RouteParams, queryParams?: RouteParams) => void;

export interface Route {
  name: RouteName;
  path: string;
  children?: Route[];
}

export const routes: Route[] = [
  {
    name: 'projects',
    path: '/{org_id}/projects',
    children: [
      {
        name: 'project',
        path: '/{proj_id}',
        children: [
          {
            name: 'domains',
            path: '/domains',
            children: [
              {
                name: 'domain',
                path: '/{domain_id}'
              }
            ]
          },
          {
            name: 'streams',
            path: '/streams',
            children: [
              {
                name: 'stream',
                path: '/{stream_id}',
                children: [
                  {
                    name: 'credentials',
                    path: '/credentials',
                  },
                  {
                    name: 'message-log',
                    path: '/messages',
                  },
                ]
              },
            ]
          }
        ]
      },
    ]
  },
  {
    name: 'domains',
    path: '/{org_id}/domains',
    children: [
      {
        name: 'domain',
        path: '/{domain_id}'
      }
    ]
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

export function matchPath(path: string): {
  route: Route,
  params: RouteParams,
  fullName: string,
} | null {
  const match = matchPathRecursive(path, routes, {}, []);
  if (match) {
    return {...match, fullName: match.fullName.join('.')}
  } else {
    return null
  }
}

// TODO remove logging

function matchPathRecursive(
  path: string,
  routes: Route[],
  pathParams: {
    [k: string]: string
  },
  fullName: string[],):
  {
    route: Route,
    params: RouteParams,
    fullName: string[],
  } | null {
  const new_path_params: { [k: string]: string } = {};
  path = path.replace(/^\/|\/$/, '');
  const path_elems = path.split('/');
  route_loop:
    for (const route of routes) {
      const route_path = route.path.replace(/^\/|\/$/, '')
      const route_elems = route_path.split('/');
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
      const new_full_name = [...fullName, route.name]
      if (route.children && path_elems.length > route_elems.length) {
        const rec_res = matchPathRecursive(path_elems.slice(route_elems.length).join('/'), route.children, new_path_params, new_full_name);
        if (rec_res) {
          return {
            route: rec_res.route,
            params: {...pathParams, ...rec_res.params, ...new_path_params},
            fullName: rec_res.fullName,
          }
        } else {
          return null;
        }
      } else {
        return {
          route,
          params: {...pathParams, ...new_path_params},
          fullName: new_full_name,
        };
      }
    }
  return null;
}

export function matchName(name: RouteName): {
  route: Route,
  fullPath: string,
  fullName: string,
} {
  const elems = name.split('.');
  const match = recursiveMatchName(routes, elems, '', [])
  if (match) {
    return {...match, fullName: match.fullName.join('.')}
  }
  return {
    route: routes[0],
    fullPath: routes[0].path,
    fullName: routes[0].name,
  };
}

function recursiveMatchName(routes: Route[], name_elems: string[], fullPath: string, fullName: string[]): {
  route: Route,
  fullPath: string,
  fullName: string[]
} | null {
  const elem = name_elems.at(0);
  if (!elem) {
    return null
  }
  for (const route of routes) {
    if (route.name === elem) {
      const newFullName = [...fullName, route.name]
      if (name_elems.length > 1 && route.children) {
        return recursiveMatchName(route.children, name_elems.slice(1), `${fullPath}/${route.path.replace(/^\/|\/$/, '')}`, newFullName)
      }
      return {
        route,
        fullPath: `${fullPath}/${route.path.replace(/^\/|\/$/, '')}`,
        fullName: newFullName,
      };
    }
  }
  return null
}

export function routerNavigate(name: RouteName, pathParams: RouteParams, queryParams: RouteParams): {
  route: Route,
  fullPath: string,
  fullName: string,
  pathParams: { [k: string]: string },
  queryParams: { [k: string]: string },
} {
  // eslint-disable-next-line prefer-const
  let {route: newRoute, fullPath: path, fullName} = matchName(name);

  const usedPathParams: RouteParams = {}

  path = path.replace(/{(\w*)}/g, (_match, path_var) => {
    const path = pathParams[path_var];
    if (!path) {
      throw new Error(`Path variable ${path_var} not found in pathParams`);
    }
    usedPathParams[path_var] = pathParams[path_var];
    return path
  })

  if (queryParams && Object.keys(queryParams).length > 0) {
    const searchParams = new URLSearchParams(queryParams);
    const fullPath = `${path}?${searchParams}`;
    window.history.pushState(
      {routeName: name, routePathParams: usedPathParams, routeQueryParams: queryParams},
      '',
      fullPath
    );
    return {route: newRoute, fullPath: path, fullName, pathParams: usedPathParams, queryParams};
  } else {
    window.history.pushState(
      {routeName: name, routePathParams: usedPathParams},
      '',
      path
    );
    return {route: newRoute, fullPath: path, fullName, pathParams: usedPathParams, queryParams: {}};
  }
}

export function initRouter() {
  const currentPath = window.location.pathname;
  const {
    route: init_route,
    params: pathParams,
    fullName: initFullName,
  } = matchPath(currentPath) || {
    route: routes[0],
    params: {},
    fullName: routes[0].name,
  };

  return {
    route: init_route,
    fullPath: currentPath,
    fullName: initFullName,
    pathParams,
    queryParams: Object.fromEntries(new URLSearchParams(window.location.search))
  };
}
